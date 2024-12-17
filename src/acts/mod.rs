//! Acts and their flags, builders, and collections
use crate::{event::RunActEvent, input::Hotkey};
use bevy::{
    ecs::{system::{EntityCommand, RegisteredSystemError, SystemId}, world::CommandQueue},
    prelude::*,
};
use bevy_input_sequence::{action, input_sequence::KeySequence, KeyChord};
use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Display,
        // Write
    },
};

mod collection;
pub use collection::*;
mod add_acts;
pub use add_acts::AddActs;
mod plugin;
pub use plugin::*;
mod arg;
pub use arg::*;
mod builder;
pub use builder::*;
mod cache;
pub use cache::*;

pub mod basic;
#[cfg(feature = "async")]
pub mod basic_async;

pub mod universal;
// impl<'w, 's> AddActs for Commands<'w, 's> {
//     fn add_acts(&mut self, acts: impl Into<Acts>) -> &mut Self {
//         let builders = acts.into();
//         self.add(move |world: &mut World| {
//             for builder in builders {
//                 let act = builder.build(world);

//         })
//     }
// }

bitflags! {
    /// `Active | Adverb | RunAct | ShowMinibuffer`
    #[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        /// Act is active.
        const Active         = 0b00000001;
        /// Act is shown in [crate::acts::basic::run_act].
        const RunAct        = 0b00000010;
        /// Act usually runs another act like run_act.
        const Adverb         = 0b00000100;
        /// Act shows the minibuffer when run.
        const ShowMinibuffer = 0b00001000;
    }
}

impl Default for ActFlags {
    fn default() -> Self {
        ActFlags::Active | ActFlags::RunAct
    }
}

struct ActInput(Box<dyn Any>);

pub trait ActRunner {
    type Input<'i>: Default;
    fn run(&self, world: &mut World) -> Result<(), RegisteredSystemError>;
    fn run_with_input(&self, input: Self::Input<'_>, world: &mut World) -> Result<(), RegisteredSystemError>;
    // fn default_input(&self) -> Self::Input<'i>;
}

struct ActSystem(SystemId);
impl ActRunner for ActSystem {
    type Input<'i> = ();
    fn run(&self, world: &mut World) -> Result<(), RegisteredSystemError> {
        world.run_system(self.0)
    }

    fn run_with_input(&self, input: Self::Input<'_>, world: &mut World) -> Result<(), RegisteredSystemError> {
        world.run_system_with_input(self.0, input)
    }

    // fn default_input(&self) -> Self::Input<'i> {
    //     ()
    // }
}

struct ActWithInputSystem<I: SystemInput + Default>(SystemId<I>);
impl<I> ActRunner for ActWithInputSystem<I> where I: SystemInput + Default {
    type Input<'i> = <I as SystemInput>::Inner<'i>;
    fn run(&self, world: &mut World) -> Result<(), RegisteredSystemError> {
        world.run_system_with_input(self.0, Self::Input::default())
    }

    fn run_with_input(&self, input: Self::Input<'_>, world: &mut World) -> Result<(), RegisteredSystemError> {
        world.run_system_with_input(self.0, input)
    }

    // fn default_input(&self) -> Self::Input {
    //     default()
    // }
}

/// A Minibuffer command
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct Act {
    /// An act's name
    pub name: Cow<'static, str>,
    /// Hot keys
    pub hotkeys: Vec<Hotkey>,
    /// What system runs when act is called
    #[reflect(ignore)]
    pub(crate) system_id: SystemId,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
}

// ScriptableAct<()> -> Act
// ScriptableAct<I> when query_id.is_some() -> Act
// ScriptableAct<I:Default> when query_id.is_some() -> Act
// pub struct ScriptableAct<I> {
//     pub name: Cow<'static, str>,
//     pub hotkeys: Vec<Hotkey>,
//     pub(crate) query_id: Option<SystemId<(),I>>,
//     pub(crate) exec_id: SystemId<I>,
//     pub flags: ActFlags,
// }

// enum Act {
//     Interactive { }
//     NonInteractive {  }
// }

// enum NonInteractive<I> {
//     SystemId(SystemId<I>),
//     Fn(Fn(I))
// }

struct MiniInput<I> {
    input: Option<I>,
}

impl<I: 'static> SystemInput for MiniInput<I> {
    type Param<'i> = MiniInput<I>;
    type Inner<'i> = I;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        MiniInput { input: Some(this) }
    }
}

impl<I: Clone> MiniInput<I> {
    pub fn get_or_ask(&mut self, f: impl Fn() -> Option<I>) -> Option<I> {
        match &self.input {
            Some(input) => Some(input.clone()),
            None => {
                let input = f();
                // Do something with it. Send it somewhere?
                input
            }
        }
    }
}

enum RunEntry {
    Act(Act),
    ActWithInput(Act, CommandQueue) // Need act in here?
}

struct Miniscript {
    content: Vec<RunEntry>,

}

impl Miniscript {
    // fn append_run(&mut self, act: &Act) {
    //     self.content.push(RunEntry::Act(act.clone()));
    // }

    // fn append_input<I: Clone>(&mut self, input: &I) {
    //     if let Some(RunEntry::Act(act)) = self.content.pop() {
    //         self.content.push(RunEntry::ActWithInput(act, |world: &mut World| {
    //             world.run_system_with_input(act.system_id, input.clone())
    //         }));
    //     } else {
    //         panic!("Did not expect an append_input from empty or that already had input.");
    //     }
    // }

    // fn run(&self, world: &mut World) {
    //     for entry in self.contents {
    //         match entry {
    //             RunEntry::Act(act) => world.run_system(act.system_id),
    //             RunEntry::ActWithInput(act, command) => command.apply(world),
    //         }
    //     }
    // }

    // fn to_script(&self) -> String {

    // }


}

impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Act {
    /// Create a new [ActBuilder].
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S, P>(system: S) -> ActBuilder
    where
        S: IntoSystem<(), (), P> + 'static,
    {
        ActBuilder::new(system)
    }

    /// Return the name of this act.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Build the [KeySequence]s.
    pub fn build_keyseqs(&self, world: &mut World) -> Vec<Entity> {
        self.hotkeys
            .iter()
            .enumerate()
            .map(|(i, hotkey)| {
                let name = Name::new(hotkey.to_string());
                let id = world.spawn(name).id();
                EntityCommand::apply(
                    KeySequence::new(
                        // XXX: Should this be trigger?
                        action::send_event(RunActEvent::new(self.clone()).with_hotkey(i)),
                        hotkey.chords.clone(),
                    ),
                    id,
                    world,
                );
                id
            })
            .collect()
    }

    /// Find hotkey based on chords.
    pub fn find_hotkey(&self, chords: &[KeyChord]) -> Option<&Hotkey> {
        self.hotkeys.iter().find(|h| *h == chords)
    }
}

impl AsRef<str> for Act {
    fn as_ref(&self) -> &str {
        self.name()
    }
}
