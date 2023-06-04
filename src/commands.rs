use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bitflags::bitflags;
use futures_lite::future;
use std::borrow::Cow;
use std::future::Future;
use trie_rs::{Trie, TrieBuilder};

use crate::tasks::*;
use crate::proc::*;
use crate::prompt::*;

pub struct RunCommandEvent(Box<dyn ScheduleLabel>);
// Could this be make generic? That way people could choose their own command run handles?
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(Cow<'static, str>);
#[derive(Resource, Default)]
pub struct CommandConfig {
    commands: Vec<Command>,
    hotkeys: Option<Trie<Key>>,
}

impl CommandConfig {
    fn hotkeys(&mut self) -> &Trie<Key> {
        let hks = self.hotkeys.get_or_insert_with(|| {
            let mut builder = TrieBuilder::new();
            for hotkey in self.commands.iter().filter_map(|command| command.hotkey.as_ref()) {
                builder.push(hotkey.clone());
            }
            builder.build()
        });
        self.hotkeys.as_ref().unwrap()

    }

}

#[derive(Debug, Clone)]
pub struct Command {
    name: Cow<'static, str>,
    hotkey: Option<KeySeq>,
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

pub fn hotkey_input(
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut config: ResMut<CommandConfig>,
    mut last_keys: Local<Vec<Key>>,
) {
    let mods = Modifiers::from_input(&keys);
    let trie = config.hotkeys();
    let mut matches = vec![];

    for key_code in keys.get_just_pressed() {
        let key = Key::new(key_code.clone(), mods);
        last_keys.push(key);
        eprintln!("key seq {:?}", *last_keys);
        if (trie.exact_match(&*last_keys)) {
            eprintln!("got match {:?}", last_keys);
            let mut new_keys = vec![];
            std::mem::swap(&mut new_keys, &mut *last_keys);
            matches.push(new_keys);
            // Let's assume it's for running a command
            // last_keys.clear();
        } else if (trie.predictive_search(&*last_keys).is_empty()) {
            eprintln!("No key seq prefix for {:?}", *last_keys);
            last_keys.clear();
        }
    }

    for amatch in matches.into_iter() {
        for command in &config.commands {
            if let Some(ref keyseq) = command.hotkey {
                eprintln!("Comparing against command {:?}", keyseq);
                if &amatch == keyseq {
                // if hotkey.mods == mods && keys.just_pressed(hotkey.key) {
                    eprintln!("We were called for {}", command.name);

                    run_command.send(RunCommandEvent(Box::new(CommandOneShot(
                        command.name.clone(),
                    ))))
                }
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct Modifiers: u8 {
        const Alt     = 0b00000001;
        const Control = 0b00000010;
        const Shift   = 0b00000100;
        const System  = 0b00001000; // Windows or Command
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Ord)]
pub struct Key {
    pub mods: Modifiers,
    pub key: KeyCode,
}

pub type KeySeq = Vec<Key>;

impl Key {
    fn new(v: KeyCode, mods: Modifiers) -> Self {
        Key {
            key: v,
            mods
        }
    }
}

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

#[cfg(test)]
mod tests {

    use bevy::prelude::*;
    use crate::commands::*;
    #[allow(unused_must_use)]
    #[test]
    fn test_key_eq() {
        let a: Key = KeyCode::A.into();
        let b: Key = KeyCode::A.into();
        assert_eq!(a, b);
        assert!(a == b);
    }

    #[test]
    fn test_key_eq_not() {
        let a: Key = KeyCode::A.into();
        let b: Key = KeyCode::B.into();
        // assert_eq!(a, b);
        assert!(a != b);
    }

    #[test]
    fn test_key_eq_vec() {
        let a: Vec<Key> = vec![KeyCode::A.into()];
        let b: Vec<Key> = vec![KeyCode::B.into()];
        let c: Vec<Key> = vec![KeyCode::A.into()];
        let e: Vec<Key> = vec![];
        assert!(a != b);
        assert!(a == c);
        assert_eq!(a, c);
        assert!(e != a);
        assert!(e != b);
        assert!(e != c);
    }
}
