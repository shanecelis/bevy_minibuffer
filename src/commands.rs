use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use futures_lite::future;
use std::borrow::Cow;
use std::future::Future;
use trie_rs::{Trie, TrieBuilder};

use crate::tasks::*;
use crate::proc::*;
use crate::prompt::*;
use crate::hotkey::*;

pub struct RunCommandEvent(pub Box<dyn ScheduleLabel>);
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
}

impl Command {
    pub fn new(name: impl Into<Cow<'static, str>>, hotkey: Option<Vec<impl Into<Key>>>) -> Self {
        Command {
            name: name.into(),
            hotkey: hotkey.map(|v| v.into_iter().map(|v| v.into()).collect()),
        }
    }
}

impl<T> From<T> for Command
where
    T: Into<Cow<'static, str>>,
{
    fn from(v: T) -> Self {
        Command {
            name: v.into(),
            hotkey: None,
        }
    }
}

pub trait AddCommand {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        system: impl IntoSystemConfigs<Params>,
    ) -> &mut Self;
}

impl AddCommand for App {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        system: impl IntoSystemConfigs<Params>,
    ) -> &mut Self {
        let cmd = cmd.into();
        let name = cmd.name.clone();
        self.add_systems(CommandOneShot(name.clone()), system);
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

pub fn run_commands(world: &mut World) {
    let mut event_system_state = SystemState::<EventReader<RunCommandEvent>>::new(world);
    let schedules: Vec<Box<dyn ScheduleLabel>> = {
        let mut events = event_system_state.get_mut(world);
        events.iter().map(|e| e.0.clone()).collect()
    };

    for schedule in schedules {
        match world.try_run_schedule(schedule) {
            Err(e) => eprintln!("Problem running command: {:?}", e),
            _ => {}
        }
    }
}

pub fn exec_command(
    mut prompt: Prompt,
    config: Res<CommandConfig>,
) -> impl Future<Output = Option<RunCommandEvent>> {
    #[rustfmt::skip]
    let commands: Vec<_> = config
        .commands
        .iter()
        .map(|c| c.name.clone())
        .collect();
    async move {
        if let Ok(command) = prompt.read_crit(": ", &&commands[..]).await {
            // We can't keep an EventWriter in our closure so we return it from
            // our task.
            Some(RunCommandEvent(Box::new(CommandOneShot(command.into()))))
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
