use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bitflags::bitflags;
use std::borrow::Cow;
use std::future::Future;
use bevy_input_sequence::*;
use asky::{Text, Message, bevy::Asky, Printable};
use crate::ui::PromptContainer;

use crate::hotkey::*;
use crate::prompt::*;
use crate::style::MinibufferStyle;

#[derive(Clone, Event)]
pub struct RunCommandEvent(pub SystemId);

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        const Active       = 0b00000001;
        const AutoComplete = 0b00000010;
    }
}

#[derive(Debug, Clone, Component)]
pub struct Act {
    pub(crate) name: Option<Cow<'static, str>>,
    pub(crate) hotkey: Option<KeySeq>,
    pub system_id: Option<SystemId>,
    pub flags: ActFlags,
}

/// Register a system to an act.
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

    pub fn unregistered() -> Self {
        Act {
            name: None,
            hotkey: None,
            system_id: None,
            flags: ActFlags::Active | ActFlags::AutoComplete,
        }
    }
    pub fn new(system_id: SystemId) -> Self {
        Act {
            name: None,
            hotkey: None,
            system_id: Some(system_id),
            flags: ActFlags::Active | ActFlags::AutoComplete,
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
        where Key: From<T> {
        self.hotkey = Some(hotkey.into_iter().map(|v| v.into()).collect());
        self
    }

    pub fn autocomplete(mut self, yes: bool) -> Self {
        self.flags.set(ActFlags::AutoComplete, yes);
        self
    }
}

impl LookUp for Vec<Act> {
    type Item = Act;
    fn look_up(&self, input: &str) -> Result<Act, LookUpError> {
        let mut matches = self
            .iter()
            .filter(|command| {
                command
                    .flags
                    .contains(ActFlags::AutoComplete | ActFlags::Active)
                    && command.name.as_ref().map(|name| name.starts_with(input)).unwrap_or(false)
            });
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.
        if let Some(first) = matches.next() {
            if let Some(second) = matches.next() {
                let mut result = vec![first.name().to_string(), second.name().to_string()];
                for item in matches {
                    result.push(item.name().to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else {
                if input == first.name() {
                    Ok(first.clone())
                } else {
                    Err(LookUpError::Incomplete(vec![first.name().to_string()]))
                }
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
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
            flags: ActFlags::Active | ActFlags::AutoComplete,
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
            spawn.insert(KeySequence::new(RunCommandEvent(system_id), cmd.hotkey.as_ref().unwrap().clone()));
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

pub(crate) fn detect_additions<E>(query: Query<(Entity, &Act),
                                               (Added<Act>, Without<KeySequence<E>>)>,
                                  mut commands: Commands)
where E: Send + Sync + 'static {
    for (id, act) in &query {
        if let Some(ref keys) = act.hotkey {
            eprintln!("add key");
            commands.entity(id).insert(KeySequence::new(RunCommandEvent(act.system_id.unwrap()), keys.clone()));
        }
    }
}

pub fn run_command_listener(mut events: EventReader<RunCommandEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.run_system(e.0);
    }
}

// pub fn exec_command(
//     mut prompt: Prompt,
//     query: Query<&Act>,
// ) -> impl Future<Output = Option<RunCommandEvent>> {
//     let commands: Vec<Act> = query.iter().cloned().collect();
//     async move {
//         match prompt.read_crit(": ", &commands).await {
//             Ok(command) =>
//                 // We can't keep an EventWriter in our closure so we return it from
//                 // our task.
//                 Some(RunCommandEvent(
//                     command
//                         .system_id
//                         .expect("No system_id for command; was it registered?"))),
//             Err(e) => {
//                 eprintln!("Got err in exec_command: {:?}", e);
//                 None
//             }
//         }
//     }
// }
pub fn exec_command(
    mut asky: Asky,
    acts: Query<&Act>,
    query: Query<Entity, With<PromptContainer>>
) -> impl Future<Output = Option<RunCommandEvent>> {
    let commands: Vec<Act> = acts.iter().cloned().collect();
    let id: Entity = query.single();
    async move {
        let _ = asky.clear(id).await;
        match asky.prompt(Text::new(":"), id).await {
            Ok(input) => {
                if let Some(command) = commands.iter().find(|x| x.name() == input) {
                // We can't keep an EventWriter in our closure so we return it from
                // our task.
                Some(RunCommandEvent(
                    command
                        .system_id
                        .expect("No system_id for command; was it registered?")))
                } else {
                    let _ = asky.clear(id);
                    asky.prompt(Message::new(format!("No such command: {input}")), id);
                    None
                }
            }
            Err(e) => {
                eprintln!("Got err in exec_command: {:?}", e);
                None
            }
        }
    }
}
