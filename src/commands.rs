use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bevy::window::RequestRedraw;
use bitflags::bitflags;
use std::borrow::Cow;
use std::future::Future;
use bevy_input_sequence::*;
use asky::Message;
use std::fmt::{self, Display, Debug, Write};
use trie_rs::map::{Trie, TrieBuilder};
use tabular::{Table, Row};

use crate::prompt::*;

#[derive(Clone, Event)]
pub struct StartActEvent(pub SystemId);

impl Debug for StartActEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rnd_state = bevy::utils::RandomState::with_seed(0);
        let hash = rnd_state.hash_one(self.0);
        write!(f, "StartActEvent({:04})", hash % 10000)
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        const Active       = 0b00000001;
        const ExecAct      = 0b00000010;
    }
}

#[derive(Debug, Clone, Component, Reflect)]
pub struct Act {
    pub(crate) name: Option<Cow<'static, str>>,
    pub(crate) hotkey: Option<Vec<KeyChord>>,
    #[reflect(ignore)]
    pub system_id: Option<SystemId>,
    #[reflect(ignore)]
    pub flags: ActFlags,
}

impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:8}", self.name())?;
        if let Some(keyseq) = &self.hotkey {
            write!(f, "\t")?;
            for key in keyseq {
                write!(f, "{} ", key)?;
            }
        }
        Ok(())
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
pub struct Register<S>(S);

impl<S> Register<S> {
    pub fn new<Into, Param>(system: Into) -> Self
        where Into: IntoSystem<(), (), Param, System = S> + 'static,
    {
        Self(IntoSystem::into_system(system))
    }
}

impl<S> bevy::ecs::system::EntityCommand for Register<S>
where S: System<In = (), Out = ()> + Send + 'static {

    fn apply(self, id: Entity, world: &mut World) {
        eprintln!("registering");
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

// TODO: Do we need a builder?
impl Act {
    const ANONYMOUS: Cow<'static, str> = Cow::Borrowed("*anonymous*");

    pub fn new() -> Self {
        Act {
            name: None,
            hotkey: None,
            system_id: None,
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }
    pub fn preregistered(system_id: SystemId) -> Self {
        Act {
            name: None,
            hotkey: None,
            system_id: Some(system_id),
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }

    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn name(&self) -> &str {
        self.name.as_ref().unwrap_or(&Self::ANONYMOUS)
    }

    pub fn hotkey<T>(mut self, hotkey: impl IntoIterator<Item = T>) -> Self
        where KeyChord: From<T> {
        self.hotkey = Some(hotkey.into_iter().map(|v| v.into()).collect());
        self
    }

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
        let mut matches = self
            .iter()
            .filter(|command| {
                command
                    .flags
                    .contains(ActFlags::ExecAct | ActFlags::Active)
                    && command.name.as_ref().map(|name| name.starts_with(input)).unwrap_or(false)
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
            hotkey: None,
            system_id: None,
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }
}

// impl bevy::ecs::system::Command for Act {
//     fn apply(self, world: &mut World) {

//     }
// }

pub trait AddAct {
    fn add_act<Params>(
        &mut self,
        cmd: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self;
}

impl AddAct for App {
    fn add_act<Params>(
        &mut self,
        cmd: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {
        // Register the system.
        let mut cmd = cmd.into();
        if cmd.system_id.is_some() {
            panic!(
                "nano command '{}' already has a system_id; was it added before?",
                cmd.name()
            );
        }
        let system_id = self.world.register_system(system);
        cmd.system_id = Some(system_id);

        // Add the command.
        // if config.commands.iter().any(|i| i.name == cmd.name) {
        //     let name = cmd.name;
        //     warn!("nano command '{name}' already added; ignoring.");
        // } else {
        //     config.commands.push(cmd);
        // }
        let mut spawn = self.world.spawn(cmd.clone());
        if cmd.hotkey.is_some() {
            spawn.insert(KeySequence::new(StartActEvent(system_id), cmd.hotkey.as_ref().unwrap().clone()));
        }

        // self.world.spawn(cmd.clone());
        self
    }
}

impl AddAct for Commands<'_, '_ >  {
    fn add_act<Params>(
        &mut self,
        act: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {

        self.spawn(act.into())
            .add(Register::new(system));
        self
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn detect_additions<E>(query: Query<(Entity, &Act),
                                               (Added<Act>, Without<KeySequence<E>>)>,
                                  mut commands: Commands)
where E: Send + Sync + 'static {
    for (id, act) in &query {
        if let Some(ref keys) = act.hotkey {
            eprintln!("add key");
            commands.entity(id).insert(KeySequence::new(StartActEvent(act.system_id.unwrap()), keys.clone()));
        }
    }
}

pub fn run_command_listener(mut events: EventReader<StartActEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.run_system(e.0);
    }
}

pub fn exec_act(
    mut asky: Minibuffer,
    acts: Query<&Act>,
) -> impl Future<Output = Option<StartActEvent>> {
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        builder.push(act.name(), act.clone());
    }
    let acts: Trie<u8, Act> = builder.build();
    async move {
        // match asky.prompt(asky::Text::new(":")).await {
        match asky.read(":".to_string(), acts.clone()).await { // TODO: Get rid of clone.
            Ok(act_name) => {
                match acts.resolve(&act_name) {
                    Ok(act) =>
                        match act.system_id {
                            Some(system_id) => Some(StartActEvent(system_id)),
                            None => {
                                let _ = asky.prompt(Message::new(format!("Error: No system_id for act {:?}; was it registered?", act))).await;
                                None
                            }
                        }
                    Err(e) =>
                        {
                            let _ = asky.prompt(Message::new(format!("Error: Could not resolve act named {:?}: {:?}", act_name, e))).await;
                            None
                        }

                }
                // } else {
                //     let _ = asky.prompt(Message::new(format!("No such command: {input}"))).await;
                //     None
                // }
            }
            Err(e) => {
                let _ = asky.prompt(Message::new(format!("Error: {:?}", e))).await;
                None
            }
        }
    }
}

/// List acts currently operant.
pub fn list_acts(
    mut asky: Minibuffer,
    acts: Query<&Act>) -> impl Future<Output = ()> {

    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new()
                    .with_cell("NAME")
                    .with_cell("KEY BINDING"));
    let mut acts: Vec<_> = acts.iter().collect();
    acts.sort_by(|a, b| a.name().cmp(b.name()));
    for act in &acts {

        let binding: String = act.hotkey.as_ref()
                                        .map(|chords|
            chords.iter().fold(String::new(), |mut output, chord| {
                let _ = write!(output, "{} ", chord);
                output
            }))
            .unwrap_or(String::from(""));
        table.add_row(Row::new()
                      .with_cell(act.name())
                      .with_cell(binding));
    }
    let msg = format!("{}", table);
    eprintln!("{}", &msg);
    async move {
        let _ = asky.prompt(Message::new(msg)).await;
    }
}

/// List key bindings for event `E`.
pub fn list_key_bindings<E: Event + Debug>(
    mut asky: Minibuffer,
    key_bindings: Query<&KeySequence<E>>
) -> impl Future<Output = ()>
{
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new()
                    .with_cell("KEY BINDING")
                    .with_cell("EVENT"));

    let mut key_bindings: Vec<(String, &E)> = key_bindings
        .iter()
        .map(|k| {
            let binding: String = k.acts
                .iter()
                .fold(String::new(),
                     |mut output, chord| {
                         let _ = write!(output, "{} ", chord);
                         output
                     });

            (binding, &k.event)
        })
        .collect();
    key_bindings.sort_by(|a, b| a.0.cmp(&b.0));
    for (binding, e) in &key_bindings {
        table.add_row(Row::new()
                      .with_cell(binding)
                      .with_cell(format!("{:?}", e)));

    }
    let msg = format!("{}", table);
    eprintln!("{}", &msg);
    async move {
        let _ = asky.prompt(Message::new(msg)).await;
    }
}

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
