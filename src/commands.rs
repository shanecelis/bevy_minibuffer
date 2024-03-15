use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bitflags::bitflags;
use std::borrow::Cow;
use std::future::Future;
use bevy_input_sequence::*;

use crate::hotkey::*;
use crate::proc::*;
use crate::prompt::*;

#[derive(Clone)]
pub struct RunCommandEvent(pub SystemId);
impl Event for RunCommandEvent {}
#[derive(Resource, Default)]
pub struct CommandConfig {
    pub(crate) commands: Vec<Act>,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        const Active       = 0b00000001;
        const AutoComplete = 0b00000010;
    }
}


#[derive(Debug, Clone, Component)]
pub struct Act {
    pub(crate) name: Cow<'static, str>,
    pub(crate) hotkey: Option<KeySeq>,
    pub system_id: Option<SystemId>,
    pub flags: ActFlags,
}

impl Act {
    pub fn new<T>(name: impl Into<Cow<'static, str>>, hotkey: impl IntoIterator<Item = T>) -> Self
        where Key: From<T> {
        Act {
            name: name.into(),
            hotkey: Some(hotkey.into_iter().map(|v| v.into()).collect()),
            system_id: None,
            flags: ActFlags::Active | ActFlags::AutoComplete,
        }
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

impl<T> From<T> for Act
where
    T: Into<Cow<'static, str>>,
{
    fn from(v: T) -> Self {
        Act {
            name: v.into(),
            hotkey: None,
            system_id: None,
            flags: ActFlags::Active | ActFlags::AutoComplete,
        }
    }
}

pub trait AddAct {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Act>,
        system: impl IntoSystem<(), (), Params> + 'static,
    ) -> &mut Self;
}

impl AddAct for App {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Act>,
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
        let system_id = self.world.register_system(system);
        cmd.system_id = Some(system_id);

        if cmd.hotkey.is_some() {
            self.world.spawn(KeySequence::new(RunCommandEvent(system_id), cmd.hotkey.as_ref().unwrap().clone()));
        }
        // Add the command.
        let mut config = self.world.resource_mut::<CommandConfig>();
        if config.commands.iter().any(|i| i.name == cmd.name) {
            let name = cmd.name;
            warn!("nano command '{name}' already added; ignoring.");
        } else {
            config.commands.push(cmd);
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
    query: Query<&Act>,
) -> impl Future<Output = Option<RunCommandEvent>> {
    // let commands = config.commands.clone();
    let commands: Vec<Act> = query.iter().cloned().collect();
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
