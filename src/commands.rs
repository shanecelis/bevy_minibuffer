use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bitflags::bitflags;
use futures_lite::future;
use std::borrow::Cow;
use std::future::Future;
use trie_rs::{Trie, TrieBuilder};

use crate::hotkey::*;
use crate::proc::*;
use crate::prompt::*;
use crate::tasks::*;

pub struct RunCommandEvent(pub SystemId);
impl Event for RunCommandEvent {}
#[derive(Resource, Default)]
pub struct CommandConfig {
    pub(crate) commands: Vec<Command>,
    pub(crate) hotkeys: Option<Trie<Key>>,
}

impl CommandConfig {
    pub fn hotkeys(&mut self) -> &Trie<Key> {
        self.hotkeys.get_or_insert_with(|| {
            let mut builder = TrieBuilder::new();
            for hotkey in self
                .commands
                .iter()
                .filter_map(|command| command.hotkey.as_ref())
            {
                builder.push(hotkey.clone());
            }
            builder.build()
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct CommandFlags: u8 {
        const Active       = 0b00000001;
        const AutoComplete = 0b00000010;
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub(crate) name: Cow<'static, str>,
    pub(crate) hotkey: Option<KeySeq>,
    pub system_id: Option<SystemId>,
    pub flags: CommandFlags,
}

impl Command {
    pub fn new(name: impl Into<Cow<'static, str>>, hotkey: Vec<impl Into<Key>>) -> Self {
        Command {
            name: name.into(),
            hotkey: Some(hotkey.into_iter().map(|v| v.into()).collect()),
            system_id: None,
            flags: CommandFlags::Active | CommandFlags::AutoComplete,
        }
    }

    pub fn autocomplete(mut self, yes: bool) -> Self {
        self.flags.set(CommandFlags::AutoComplete, yes);
        self
    }
}

impl LookUp for Vec<Command> {
    type Item = Command;
    fn look_up(&self, input: &str) -> Result<Command, LookUpError> {
        let mut matches = self
            .iter()
            .filter(|command| {
                command
                    .flags
                    .contains(CommandFlags::AutoComplete | CommandFlags::Active)
                    && command.name.starts_with(input)
            });
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.
        if let Some(first) = matches.next() {
            if let Some(second) = matches.next() {
                let mut result = vec![first.name.to_string(), second.name.to_string()];
                for item in matches {
                    result.push(item.name.to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else {
                if input == first.name {
                    Ok(first.clone())
                } else {
                    Err(LookUpError::Incomplete(vec![first.name.to_string()]))
                }
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
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
            system_id: None,
            flags: CommandFlags::Active | CommandFlags::AutoComplete,
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
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self {
        // Register the system.
        let mut cmd = cmd.into();
        if cmd.system_id.is_some() {
            panic!(
                "nano command '{}' already has a system_id; was it added before?",
                cmd.name
            );
        }
        cmd.system_id = Some(self.world.register_system(system));

        // Add the command.
        let mut config = self.world.resource_mut::<CommandConfig>();
        if config.commands.iter().any(|i| i.name == cmd.name) {
            let name = cmd.name;
            warn!("nano command '{name}' already added; ignoring.");
        } else {
            config.commands.push(cmd);
        }
        if config.hotkeys.is_some() {
            warn!("resetting hotkey trie.");
            config.hotkeys = None
        }
        self
    }
}

pub fn run_command_listener(mut events: EventReader<RunCommandEvent>, mut commands: Commands) {
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
        match prompt.read_crit(": ", &commands).await {
            Ok(command) =>
                // We can't keep an EventWriter in our closure so we return it from
                // our task.
                Some(RunCommandEvent(
                    command
                        .system_id
                        .expect("No system_id for command; was it registered?"))),
            Err(e) => {
                eprintln!("Got err in exec_command: {:?}", e);
                None
            }
        }
    }
}

