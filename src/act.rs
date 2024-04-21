//! acts, or commands
use crate::{
    event::RunActEvent,
    lookup::{LookUp, LookUpError, Resolve},
    prompt::{CompletionState, PromptState},
    Minibuffer,
};
use asky::Message;
use bevy::{ecs::system::{SystemId, BoxedSystem}, prelude::*, window::RequestRedraw};
use bevy_defer::{world, AsyncAccess};
use bevy_input_sequence::{
    cache::InputSequenceCache,
    KeyChord,
    input_sequence::KeySequence,
    action};
use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display, Write},
    future::Future,
};
use tabular::{Row, Table};
use trie_rs::map::{Trie, TrieBuilder};

bitflags! {
    /// Act flags
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        /// Act is active.
        const Active       = 0b00000001;
        /// Act is shown in [crate::act::exec_act].
        const ExecAct      = 0b00000010;
    }
}

/// Act, a command in `bevy_minibuffer`
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct Act {
    pub name: Cow<'static, str>,
    pub hotkeys: Vec<Vec<KeyChord>>,
    #[reflect(ignore)]
    pub(crate) system_id: SystemId,
    // #[reflect(ignore)]
    // pub(crate) system: Option<Box<System<In = (), Out = ()>>>,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
}

pub struct ActBuilder {
    pub(crate) name: Option<Cow<'static, str>>,
    pub(crate) hotkeys: Vec<Vec<KeyChord>>,
    pub(crate) system: BoxedSystem,
    /// Flags for this act
    pub flags: ActFlags,
}


impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ActBuilder {
    /// Create a new [Act].
    pub fn new<S, P>(system: S) -> Self
    where
        S: IntoSystem<(), (), P> + 'static {
        ActBuilder {
            name: None,
            hotkeys: Vec::new(),
            system: Box::new(IntoSystem::into_system(system)),
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }

    pub fn build(mut self, world: &mut World) -> Act
    {
        Act {
            name: self.name.unwrap_or_else(|| {
                let n = self.system.name();
                if let Some(start) = n.find('(') {
                    if let Some(end) = n.find(&[',', ' ', ')']) {
                        return n[start + 1..end].to_owned().into();
                    }
                }
                n
            }),
            hotkeys: self.hotkeys,
            flags: self.flags,
            system_id: world.register_boxed_system(self.system),
        }
    }

    /// Name the act.
    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a hotkey.
    pub fn hotkey<T>(mut self, hotkey: impl IntoIterator<Item = T>) -> Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys
            .push(hotkey.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Specify whether act should show up in [crate::act::exec_act].
    pub fn in_exec_act(mut self, yes: bool) -> Self {
        self.flags.set(ActFlags::ExecAct, yes);
        self
    }
}

impl Act {
    /// Create a new [ActBuilder].
    pub fn new<S, P>(system: S) -> ActBuilder
    where
        S: IntoSystem<(), (), P> + 'static {
        ActBuilder::new(system)
    }

    /// Return the name of this act or [Self::ANONYMOUS].
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl AsRef<str> for Act {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

impl Resolve for Vec<Act> {
    type Item = Act;
    fn resolve(&self, input: &str) -> Result<Act, LookUpError> {
        let mut matches = self.iter().filter(|command| {
            command.flags.contains(ActFlags::ExecAct | ActFlags::Active)
                && command.name.starts_with(input)
        });
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.
        if let Some(first) = matches.next() {
            if input == first.name() {
                Ok(first.clone())
            } else if let Some(second) = matches.next() {
                let mut result = vec![first.name().to_string(), second.name().to_string()];
                for item in matches {
                    result.push(item.name().to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else {
                Err(LookUpError::Incomplete(vec![first.name().to_string()]))
            }
        } else {
            Err(LookUpError::Message("no matches".into()))
        }
    }
}

impl LookUp for Vec<Act> {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, _input: &str) -> Option<String> {
        None
    }
}

impl bevy::ecs::system::Command for ActBuilder
{
    fn apply(self, world: &mut World) {
        let act = self.build(world);
        world.spawn(act);
    }
}

impl bevy::ecs::system::EntityCommand for ActBuilder
{
    fn apply(self, id: Entity, world: &mut World) {
        let act = self.build(world);
        let mut entity = world.get_entity_mut(id).unwrap();
        entity.insert(act);
    }
}

/// Add an act extension trait
pub trait AddAct {
    /// Add an act with the given system.
    fn add_act(
        &mut self,
        act: ActBuilder,
    ) -> &mut Self;
}

impl AddAct for App {
    fn add_act(
        &mut self,
        act: ActBuilder,
    ) -> &mut Self {
        let act = act.build(&mut self.world);
        self.world.spawn(act);
        self
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn detect_additions(
    query: Query<(Entity, &Act), (Added<Act>, Without<KeySequence>)>,
    mut commands: Commands,
) {
    for (id, act) in &query {
        commands.entity(id).with_children(|builder| {
            for hotkey in &act.hotkeys {
                builder.spawn_empty().add(KeySequence::new(action::send_event(RunActEvent(act.clone())), hotkey.clone()));
            }
        });
    }
}

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
pub fn exec_act(mut asky: Minibuffer, acts: Query<&Act>) -> impl Future<Output = Result<(), crate::Error>> {
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        if act.flags.contains(ActFlags::ExecAct | ActFlags::Active) {
            builder.push(act.name(), act.clone());
        }
    }
    let acts: Trie<u8, Act> = builder.build();
    async move {
        match asky.read(":".to_string(), acts.clone()).await {
            // TODO: Get rid of clone.
            Ok(act_name) => match acts.resolve(&act_name) {
                Ok(act) => {
                    world().send_event(RunActEvent(act)).await?;
                }
                Err(e) => {
                    asky
                        .prompt(Message::new(format!(
                            "Error: Could not resolve act named {:?}: {}",
                            act_name, e
                        )))
                        .await?;
                }
            },
            Err(e) => {
                asky.prompt(Message::new(format!("Error: {e}"))).await?;
            }
        }
        Ok(())
    }
}

/// List acts currently operant.
pub fn list_acts(mut asky: Minibuffer, acts: Query<&Act>) -> impl Future<Output = ()> {
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new().with_cell("ACT").with_cell("KEY BINDING"));
    let mut acts: Vec<_> = acts.iter().collect();
    acts.sort_by(|a, b| a.name().cmp(b.name()));
    for act in &acts {
        let mut name = Some(act.name());
        if act.hotkeys.is_empty() {
            table.add_row(
                Row::new()
                    .with_cell(name.take().unwrap_or(""))
                    .with_cell(""),
            );
        } else {
            let bindings = act.hotkeys.iter().map(|chords| {
                chords.iter().fold(String::new(), |mut output, chord| {
                    let _ = write!(output, "{} ", chord);
                    output
                })
            });

            for binding in bindings {
                table.add_row(
                    Row::new()
                        .with_cell(name.take().unwrap_or(""))
                        .with_cell(binding),
                );
            }
        }
    }
    let msg = format!("{}", table);
    // eprintln!("{}", &msg);
    async move {
        let _ = asky.prompt(Message::new(msg)).await;
    }
}

/// List key bindings for event `E`.
pub fn list_key_bindings(
    mut asky: Minibuffer,
    key_bindings: Query<&KeySequence>,
) -> impl Future<Output = ()> {
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new().with_cell("KEY BINDING").with_cell("EVENT"));

    let mut key_bindings: Vec<(String, Cow<'static, str>)> = key_bindings
        .iter()
        .map(|k| {
            let binding: String = k.acts.iter().fold(String::new(), |mut output, chord| {
                let _ = write!(output, "{} ", chord);
                output
            });

            (binding, "N/A".into())
        })
        .collect();
    key_bindings.sort_by(|a, b| a.0.cmp(&b.0));
    for (binding, e) in &key_bindings {
        table.add_row(Row::new().with_cell(binding).with_cell(format!("{}", e)));
    }
    let msg = format!("{}", table);
    // eprintln!("{}", &msg);
    async move {
        let _ = asky.prompt(Message::new(msg)).await;
    }
}

/// Toggle visibility
pub fn toggle_visibility(
    mut redraw: EventWriter<RequestRedraw>,
    prompt_state: Res<State<PromptState>>,
    completion_state: Res<State<CompletionState>>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
) {
    match (**prompt_state, **completion_state) {
        (PromptState::Invisible, CompletionState::Invisible) => {
            next_prompt_state.set(PromptState::Visible);
            next_completion_state.set(CompletionState::Visible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Visible, CompletionState::Visible) => {
            next_prompt_state.set(PromptState::Invisible);
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Invisible, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Visible, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Finished, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
    }
}

/// Input a key sequence. This will tell you what it does.
pub fn describe_key(
    keyseqs: Query<&KeySequence>,
    mut cache: ResMut<InputSequenceCache<KeyChord, ()>>,
    mut minibuffer: Minibuffer,
) -> impl Future<Output = Result<(), crate::Error>> {
    use trie_rs::inc_search::Answer;
    let trie: Trie<_, _> = cache.trie(keyseqs.iter()).clone();
    async move {
        let mut search = trie.inc_search();
        let mut accum = String::from("Press key: ");

        loop {
            minibuffer.prompt(Message::new(accum.clone())).await?;
            let chord = minibuffer.get_chord().await?;
            match search.query(&chord) {
                Some(x) => {
                    let _ = write!(accum, "{} ", chord);
                    let v = search.value();
                    let msg = match x {
                        Answer::Match => format!("{}is bound to {:?}", accum, v.unwrap().system_id),
                        Answer::PrefixAndMatch => {
                            format!("{}is bound to {:?} and more", accum, v.unwrap().system_id)
                        }
                        Answer::Prefix => accum.clone(),
                    };
                    minibuffer.prompt(Message::new(msg)).await?;
                    if matches!(x, Answer::Match) {
                        break;
                    }
                }
                None => {
                    let _ = write!(accum, "{} ", chord);
                    let msg = format!("{}is unbound", accum);
                    minibuffer.prompt(Message::new(msg)).await?;
                    break;
                }
            }
        }
        Ok(())
    }
}

