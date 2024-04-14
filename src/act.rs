//! acts, or commands
use crate::{
    event::RunActEvent,
    lookup::{LookUp, LookUpError, Resolve},
    prompt::{CompletionState, PromptState},
    Minibuffer,
};
use asky::Message;
use bevy::{ecs::system::SystemId, prelude::*, window::RequestRedraw};
use bevy_input_sequence::{KeyChord, KeySequence, InputSequenceCache};
use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display, Write},
    future::Future,
};
use tabular::{Row, Table};
use trie_rs::map::{Trie, TrieBuilder};
use bevy_defer::world;

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
pub struct Act {
    pub(crate) name: Option<Cow<'static, str>>,
    pub(crate) hotkeys: Vec<Vec<KeyChord>>,
    #[reflect(ignore)]
    pub(crate) system_id: Option<SystemId>,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
}

impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// TODO: Do we need a builder?
impl Default for Act {
    fn default() -> Self {
        Self::new()
    }
}

impl Act {
    /// The name of anonymous acts
    pub const ANONYMOUS: Cow<'static, str> = Cow::Borrowed("*anonymous*");

    /// Create a new [Act].
    pub fn new() -> Self {
        Act {
            name: None,
            hotkeys: Vec::new(),
            system_id: None,
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }

    /// Create a new [Act] registered with `system_id`.
    pub fn preregistered(system_id: SystemId) -> Self {
        Act {
            name: None,
            hotkeys: Vec::new(),
            system_id: Some(system_id),
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }

    pub fn register<S, P>(mut self, system: S, world: &mut World) -> Self
    where S: IntoSystem<(), (), P> + 'static,
    {
        if self.system_id.is_some() {
            panic!("cannot register act {}; it has already been registered", self.name());
        }
        let system = IntoSystem::into_system(system);
        let system_id = world.register_system(system);
        self.system_id = Some(system_id);
        self
    }

    /// Name the act.
    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Return the name of this act or [Self::ANONYMOUS].
    pub fn name(&self) -> &str {
        self.name.as_ref().unwrap_or(&Self::ANONYMOUS)
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
                && command
                    .name
                    .as_ref()
                    .map(|name| name.starts_with(input))
                    .unwrap_or(false)
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

impl<T> From<T> for Act
where
    T: Into<Cow<'static, str>>,
{
    fn from(v: T) -> Self {
        Act {
            name: Some(v.into()),
            hotkeys: Vec::new(),
            system_id: None,
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }
}

/// Register a system to an act.
///
/// ```compile
/// fn setup_act(act: Act, mut commands: Commands) {
///     commands.spawn(Act)
///         .add(Register(my_action));
/// }
///
/// fn my_action(query: Query<&Transform>) {
///
/// }
/// ```
struct Register<S>(S);

impl<S> Register<S> {
    /// Create a new Register.
    pub fn new<Into, Param>(system: Into) -> Self
    where
        Into: IntoSystem<(), (), Param, System = S> + 'static,
    {
        Self(IntoSystem::into_system(system))
    }
}

impl<S> bevy::ecs::system::EntityCommand for Register<S>
where
    S: System<In = (), Out = ()> + Send + 'static,
{
    fn apply(self, id: Entity, world: &mut World) {
        let system_id = world.register_system(self.0);
        let mut entity = world.get_entity_mut(id).unwrap();
        let mut act = entity.get_mut::<Act>().unwrap();
        if act.system_id.is_some() {
            panic!("System already registered to act {:?}.", act);
        } else {
            act.system_id = Some(system_id);
        }
    }
}

/// Add an act extension trait
pub trait AddAct {
    /// Add an act with the given system.
    fn add_act<Params>(
        &mut self,
        act: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self;
}

impl AddAct for App {
    fn add_act<Params>(
        &mut self,
        act: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {
        // Register the system.
        let mut act = act.into();
        if act.system_id.is_some() {
            panic!(
                "act '{}' already has a system_id; was it added before?",
                act.name()
            );
        }
        let system_id = self.world.register_system(system);
        act.system_id = Some(system_id);
        self.world.spawn(act);

        self
    }
}

impl AddAct for Commands<'_, '_> {
    fn add_act<Params>(
        &mut self,
        act: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {
        self.spawn(act.into()).add(Register::new(system));
        self
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn detect_additions<E>(
    query: Query<(Entity, &Act), (Added<Act>, Without<KeySequence<E>>)>,
    mut commands: Commands,
) where
    E: Send + Sync + 'static,
{
    for (id, act) in &query {
        commands.entity(id).with_children(|builder| {
            for hotkey in &act.hotkeys {
                builder.spawn(KeySequence::new(
                    RunActEvent(act.clone()),
                    hotkey.clone(),
                ));
            }
        });
    }
}

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
pub fn exec_act(
    mut asky: Minibuffer,
    acts: Query<&Act>,
) -> impl Future<Output = ()> {
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
                Ok(act) => match act.system_id {
                    Some(_system_id) => {
                        world().send_event(RunActEvent(act)).await;
                    },
                    None => {
                        let _ = asky
                            .prompt(Message::new(format!(
                                "Error: No system_id for act {:?}; was it registered?",
                                act
                            )))
                            .await;
                    }
                },
                Err(e) => {
                    let _ = asky
                        .prompt(Message::new(format!(
                            "Error: Could not resolve act named {:?}: {}",
                            act_name, e
                        )))
                        .await;
                }
            },
            Err(e) => {
                let _ = asky.prompt(Message::new(format!("Error: {e}"))).await;
            }
        }
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
pub fn list_key_bindings<E: Event + Display>(
    mut asky: Minibuffer,
    key_bindings: Query<&KeySequence<E>>,
) -> impl Future<Output = ()> {
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new().with_cell("KEY BINDING").with_cell("EVENT"));

    let mut key_bindings: Vec<(String, &E)> = key_bindings
        .iter()
        .map(|k| {
            let binding: String = k.acts.iter().fold(String::new(), |mut output, chord| {
                let _ = write!(output, "{} ", chord);
                output
            });

            (binding, &k.event)
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
pub fn describe_key<E: Event + Clone + Display>(
    keyseqs: Query<&KeySequence<E>>,
    mut cache: ResMut<InputSequenceCache<E, KeyChord>>,
    mut minibuffer: Minibuffer,
) -> impl Future<Output = Result<(), crate::Error>> {
    use trie_rs::inc_search::Answer;
    let trie: Trie<_, _> = cache.trie(keyseqs.iter())
                                .clone();
    async move {
        let mut search = trie.inc_search();
        let mut accum = String::from("Press key: ");

        loop {
            minibuffer.prompt(Message::new(accum.clone())).await?;
            let chords = minibuffer.get_chord().await?;
            match search.query_until(&chords) {
                Ok(x) => {
                    for chord in chords {
                        let _ = write!(accum, "{} ", chord);
                    }
                    let v = search.value();
                    let msg = match x {
                        Answer::Match =>
                            format!("{}is bound to {}", accum, v.unwrap().event),
                        Answer::PrefixAndMatch =>
                            format!("{}is bound to {} and more", accum, v.unwrap().event),
                        Answer::Prefix => accum.clone()
                    };
                    minibuffer.prompt(Message::new(msg)).await?;
                    if matches!(x, Answer::Match) {
                        break;
                    }
                }
                Err(i) => {
                    for chord in chords.into_iter().take(i + 1) {
                        let _ = write!(accum, "{} ", chord);
                    }
                    let msg = format!("{}is unbound", accum);
                    minibuffer.prompt(Message::new(msg)).await?;
                    break;
                }
            }
        }
        Ok(())
    }
}
