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
    any::{Any, TypeId},
    sync::Arc,
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
pub mod cache;
mod run_act;
pub use run_act::*;

pub mod basic;
#[cfg(feature = "async")]
pub mod basic_async;

pub mod universal;
pub mod tape;
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
        const RunAct         = 0b00000010;
        /// Act usually runs another act like run_act.
        const Adverb         = 0b00000100;
        /// Act shows the minibuffer when run.
        const ShowMinibuffer = 0b00001000;
        /// Act is recordable.
        const Record         = 0b00010000;
    }
}

impl Default for ActFlags {
    fn default() -> Self {
        ActFlags::Active | ActFlags::RunAct | ActFlags::Record
    }
}

/// A Minibuffer command
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct Act {
    /// An act's name
    pub name: Cow<'static, str>,
    /// Hot keys
    pub hotkeys: Vec<Hotkey>,
    // What system runs when act is called
    // #[reflect(ignore)]
    pub(crate) system_id: Entity,
    // #[reflect(ignore)]
    // pub run_act: Box<dyn RunAct + Send + Sync>,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
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

    pub fn new_with_input<S, I, P>(system: S) -> ActBuilder
        where S: IntoSystem<In<I>,(), P> + 'static,
    I: 'static + Default + Clone + Send + Sync
    {
        ActBuilder::new_with_input(system)
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
