use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{SystemState, SystemId, RunSystemOnce};
use bevy::prelude::*;
use futures_lite::future;
use std::borrow::Cow;
use std::future::Future;
use trie_rs::{Trie, TrieBuilder};

use crate::tasks::*;
use crate::proc::*;
use crate::prompt::*;
use crate::hotkey::*;

pub struct RunCommandEvent(pub SystemId);
impl Event for RunCommandEvent {}
// Could this be make generic? That way people could choose their own command run handles?
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(pub CowStr);
#[derive(Resource, Default)]
pub struct CommandConfig {
    pub(crate) commands: Vec<Command>,
    pub(crate) hotkeys: Option<Trie<Key>>,
}

impl CommandConfig {
    pub fn hotkeys(&mut self) -> &Trie<Key> {
        self.hotkeys.get_or_insert_with(|| {
            let mut builder = TrieBuilder::new();
            for hotkey in self.commands.iter().filter_map(|command| command.hotkey.as_ref()) {
                builder.push(hotkey.clone());
            }
            builder.build()
        })
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub(crate) name: Cow<'static, str>,
    pub(crate) hotkey: Option<KeySeq>,
    pub system_id: Option<SystemId>,
}

impl Command {
    pub fn new(name: impl Into<Cow<'static, str>>, hotkey: Option<Vec<impl Into<Key>>>) -> Self {
        Command {
            name: name.into(),
            hotkey: hotkey.map(|v| v.into_iter().map(|v| v.into()).collect()),
            system_id: None,
        }
    }
}

impl LookUp for Vec<Command> {
    type Item = Command;
    fn look_up(&self, input: &str) -> Result<Command, LookUpError> {

        let matches: Vec<&Command> = self
            .iter()
            // .map(|word| word.as_ref())
            .filter(|command| command.name.starts_with(input))
            .collect();
        match matches[..] {
            [a] => Ok(a.clone()),
            [_a, _b, ..] => Err(LookUpError::Incomplete(
                matches.into_iter().map(|s| s.name.to_string()).collect(),
            )),
            [] => Err(LookUpError::Message(" no matches".into())),
        }
        // self.iter().find(|item| item.name == input)
        //     .ok_or_else(

    }
}

// impl AsRef<str> for Command {
//     fn as_ref(&self) -> &str {
//         self.name
//     }
// }

impl<T> From<T> for Command
where
    T: Into<Cow<'static, str>>,
{
    fn from(v: T) -> Self {
        Command {
            name: v.into(),
            hotkey: None,
            system_id: None,
        }
    }
}

pub trait AddCommand {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self;
}

impl AddCommand for App {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        // system: impl IntoSystemConfigs<Params>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {
        let mut cmd = cmd.into();
        let name = cmd.name.clone();
        cmd.system_id = Some(self.world.register_system(system));
        // self.add_systems(CommandOneShot(name.clone()), system);
        // Create an ad hoc start up system to register this name.
        let sys = move |mut config: ResMut<CommandConfig>| {
            if config.commands.iter().any(|i| i.name == name) {
                warn!("nano command '{name}' already registered.");
            } else {
                config.commands.push(cmd.clone());
            }
        };
        // XXX: Do these Startup systems stick around?
        self.add_systems(Startup, sys);
        self
    }
}

// pub fn run_commands(world: &mut World) {
//     let mut event_system_state = SystemState::<EventReader<RunCommandEvent>>::new(world);
//     for event in event_system_state.get_mut(world).read() {
//         world.run_system_once(event.0);
//     }
    
//     // let schedules: Vec<Box<dyn ScheduleLabel>> = {
//     //     let mut events = event_system_state.get_mut(world);
//     //     events.iter().map(|e| e.0.clone()).collect()
//     // };

//     // for schedule in schedules {
//     //     if let Err(e) = world.try_run_schedule(schedule) {
//     //         eprintln!("Problem running command: {:?}", e);
//     //     }
//     // }
// }
// 
pub fn run_command_listener(
    mut events: EventReader<RunCommandEvent>,
    mut commands: Commands) {
    for e in events.read() {
        commands.run_system(e.0);
    }
}
pub fn exec_command(
    mut prompt: Prompt,
    config: Res<CommandConfig>,
) -> impl Future<Output = Option<RunCommandEvent>> {
    let commands = config.commands.clone();
    async move {
        if let Ok(command) = prompt.read_crit(": ", &commands).await {
            // We can't keep an EventWriter in our closure so we return it from
            // our task.
            // commands.run_system(command.system_id.unwrap())
            Some(RunCommandEvent(command.system_id.unwrap()))
        } else {
            eprintln!("Got err in exec_command");
            None
        }
    }
}

pub fn poll_event_tasks<T: Send + Event>(
    mut commands: Commands,
    mut run_command: EventWriter<T>,
    mut command_tasks: Query<(Entity, &mut TaskSink<Option<T>>)>,
) {
    for (entity, mut task) in &mut command_tasks {
        if let Some(maybe) = future::block_on(future::poll_once(&mut task.0)) {
            eprintln!("Got event poll task");
            if let Some(event) = maybe {
                run_command.send(event);
            }
            commands.entity(entity).despawn();
        }
    }
}
