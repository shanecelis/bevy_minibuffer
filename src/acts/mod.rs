//! Acts and their flags, builders, and collections
use crate::{event::RunActEvent, input::Hotkey, ui::ActContainer};
use bevy::{ecs::system::EntityCommand, prelude::*};
use bevy_input_sequence::{action, input_sequence::KeySequence, KeyChord};
use bitflags::bitflags;
use std::{
    any::TypeId,
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
pub mod cache;
mod run_act;
pub use run_act::*;

pub mod basic;
#[cfg(feature = "async")]
pub mod basic_async;

pub mod tape;
pub mod universal;

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Act>()
        .add_plugins(tape::plugin)
        .add_plugins(universal::plugin)
        .add_plugins(cache::plugin)
        .add_plugins(run_act::plugin)
        .add_systems(PostStartup, reparent_acts);
}

fn reparent_acts(
    acts: Query<Entity, With<Act>>,
    act_container: Query<Entity, With<ActContainer>>,
    mut commands: Commands,
) {
    let Ok(act_container) = act_container.get_single() else {
        warn!("No ActContainer");
        return;
    };
    for id in &acts {
        commands.entity(id).set_parent(act_container);
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct ActRef {
    pub id: Entity,
    pub flags: ActFlags,
}

impl ActRef {
    pub fn new(id: Entity, flags: ActFlags) -> Self {
        ActRef { id, flags }
    }

    pub fn from_act(act: &Act, act_id: Entity) -> Self {
        ActRef {
            id: act_id,
            flags: act.flags,
        }
    }
}

/// A Minibuffer command
#[derive(Debug, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct Act {
    /// An act's name
    pub name: Cow<'static, str>,
    /// Hot keys
    pub hotkeys: Vec<Hotkey>,
    // What system runs when act is called
    // #[reflect(ignore)]
    pub(crate) system_id: Entity,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
    pub(crate) system_name: Cow<'static, str>,
    pub(crate) input: Option<TypeId>,
}
impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
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
    where
        S: IntoSystem<In<I>, (), P> + 'static,
        I: 'static + Debug + Default + Clone + Send + Sync,
    {
        ActBuilder::new_with_input(system)
    }

    /// Build the [KeySequence]s.
    pub fn build_keyseqs(&self, act_id: Entity, world: &mut World) -> Vec<Entity> {
        self.hotkeys
            .iter()
            .enumerate()
            .map(|(i, hotkey)| {
                let name = Name::new(hotkey.to_string());
                let id = world.spawn(name).id();
                EntityCommand::apply(
                    KeySequence::new(
                        // XXX: Should this be trigger?
                        // action::send_event(RunActEvent::from_act(self, act_id).with_hotkey(i)),
                        action::trigger(RunActEvent::from_act(self, act_id).with_hotkey(i)),
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
        &self.name
    }
}
