use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bitflags::bitflags;
use futures_lite::future;
use std::borrow::Cow;
use std::future::Future;

use crate::prompt::*;
use crate::tasks::*;

pub struct RunCommandEvent(Box<dyn ScheduleLabel>);
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(Cow<'static, str>);
#[derive(Resource, Debug, Default)]
pub struct CommandConfig {
    commands: Vec<Command>,
}

#[derive(Debug, Clone)]
pub struct Command {
    name: Cow<'static, str>,
    hotkey: Option<Key>,
}

impl Command {
    pub fn new(name: impl Into<Cow<'static, str>>, hotkey: Option<impl Into<Key>>) -> Self {
        Command {
            name: name.into(),
            hotkey: hotkey.map(|v| v.into()),
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
    // mut run_command: EventWriter<RunCommandEvent>,
    config: Res<CommandConfig>,
) -> impl Future<Output = Option<RunCommandEvent>> {
    let commands: Vec<_> = config
        .commands
        .clone()
        .into_iter()
        .map(|c| c.name)
        .collect();
    async move {
        if let Ok(command) = prompt.read_crit(": ", &&commands[..]).await {
            println!("COMMAND: {command}");
            Some(RunCommandEvent(Box::new(CommandOneShot(command.into()))))
        } else {
            println!("Got err in ask now");
            None
        }
    }
}

pub fn poll_event_tasks(
    mut commands: Commands,
    mut run_command: EventWriter<RunCommandEvent>,
    mut command_tasks: Query<(Entity, &mut TaskSink<Option<RunCommandEvent>>)>,
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

pub fn hotkey_input(
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    config: Res<CommandConfig>,
) {
    let mods = Modifiers::from_input(&keys);
    for command in &config.commands {
        if let Some(ref hotkey) = command.hotkey {
            if hotkey.mods == mods && keys.just_pressed(hotkey.key) {
                eprintln!("We were called for {}", command.name);

                run_command.send(RunCommandEvent(Box::new(CommandOneShot(
                    command.name.clone(),
                ))))
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const Alt     = 0b00000001;
        const Control = 0b00000010;
        const Shift   = 0b00000100;
        const System  = 0b00001000; // Windows or Command
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub mods: Modifiers,
    pub key: KeyCode,
}
pub type KeySeq = Vec<Key>;

impl From<KeyCode> for Key {
    fn from(v: KeyCode) -> Self {
        Key {
            key: v,
            mods: Modifiers::empty(),
        }
    }
}

impl Modifiers {
    fn from_input(input: &Res<Input<KeyCode>>) -> Modifiers {
        let mut mods = Modifiers::empty();
        if input.any_pressed([KeyCode::LShift, KeyCode::RShift]) {
            mods |= Modifiers::Shift;
        }
        if input.any_pressed([KeyCode::LControl, KeyCode::RControl]) {
            mods |= Modifiers::Control;
        }
        if input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]) {
            mods |= Modifiers::Alt;
        }
        if input.any_pressed([KeyCode::LWin, KeyCode::RWin]) {
            mods |= Modifiers::System;
        }
        mods
    }
}
