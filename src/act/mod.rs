//! acts, or commands
use crate::event::RunActEvent;
#[cfg(feature = "async")]
use crate::MinibufferAsync;
use bevy::{
    ecs::system::{BoxedSystem, SystemId},
    prelude::*,
};
use bevy_input_sequence::{action, input_sequence::KeySequence, KeyChord};
use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display, Write},
    // cell::RefCell,
    sync::Mutex,
};
use trie_rs::map::{Trie, TrieBuilder};
mod acts;
pub use acts::ActsPlugin;
#[cfg(feature = "async")]
use bevy_defer::AsyncWorld;

bitflags! {
    /// Act flags
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        /// Act is active.
        const Active       = 0b00000001;
        /// Act is shown in [crate::act::exec_act].
        const ExecAct      = 0b00000010;
        /// Act usually runs another act like exec_act.
        const Adverb       = 0b00000100;
    }
}

/// Act, a command in `bevy_minibuffer`
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct Act {
    /// An act's name
    pub name: Cow<'static, str>,
    /// Hot keys
    pub hotkeys: Vec<Vec<KeyChord>>,
    /// What system runs when act is called
    #[reflect(ignore)]
    pub(crate) system_id: SystemId,
    /// Flags for this act
    #[reflect(ignore)]
    pub flags: ActFlags,
}

/// A cache that maps hotkeys to [Act]s.
#[derive(Resource, Default)]
pub struct ActCache {
    trie: Option<Trie<KeyChord, Act>>,
}

impl ActCache {
    /// Retrieve the cached trie without iterating through `sequences`. Or if
    /// the cache has been invalidated, build and cache a new trie using the
    /// `sequences` iterator.
    pub fn trie<'a>(&mut self, acts: impl Iterator<Item = &'a Act>) -> &Trie<KeyChord, Act> {
        self.trie.get_or_insert_with(|| {
            let mut builder: TrieBuilder<KeyChord, Act> = TrieBuilder::new();
            for act in acts {
                for hotkey in &act.hotkeys {
                    builder.insert(hotkey.clone(), act.clone());
                }
            }
            builder.build()
        })
    }

    /// Invalidate the cache.
    pub fn invalidate(&mut self) {
        self.trie = None;
    }
}

/// Builds an act.
#[derive(Debug)]
pub struct ActBuilder {
    pub(crate) name: Option<Cow<'static, str>>,
    /// Hotkeys
    pub hotkeys: Vec<Vec<KeyChord>>,
    pub(crate) system: BoxedSystem,
    /// Flags for this act
    pub flags: ActFlags,
}

impl Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ActBuilder {
    /// Create a new [Act].
    pub fn new<S, P>(system: S) -> Self
    where
        S: IntoSystem<(), (), P> + 'static,
    {
        ActBuilder {
            name: None,
            hotkeys: Vec::new(),
            system: Box::new(IntoSystem::into_system(system)),
            flags: ActFlags::Active | ActFlags::ExecAct,
        }
    }

    pub fn name(&self) -> Cow<'static, str> {
        self.name.clone().unwrap_or_else(|| {
            let n = self.system.name();
            if let Some(start) = n.find('(') {
                if let Some(end) = n.find([',', ' ', ')']) {
                    return n[start + 1..end].to_owned().into();
                }
            }
            n
        })
    }

    /// Build [Act].
    pub fn build(self, world: &mut World) -> Act {
        Act {
            name: self.name.unwrap_or_else(|| {
                let n = self.system.name();
                if let Some(start) = n.find('(') {
                    if let Some(end) = n.find([',', ' ', ')']) {
                        return n[start + 1..end].to_owned().into();
                    }
                }
                n
            }),
            hotkeys: self.hotkeys,
            flags: self.flags,
            system_id: world.register_boxed_system(self.system),
        }
    }

    /// Name the act.
    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a hotkey.
    pub fn hotkey<T>(mut self, hotkey: impl IntoIterator<Item = T>) -> Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys
            .push(hotkey.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Specify whether act should show up in [crate::act::exec_act].
    pub fn in_exec_act(mut self, yes: bool) -> Self {
        self.flags.set(ActFlags::ExecAct, yes);
        self
    }
}

/// A plugin that can only be built once.
pub trait PluginOnce {
    /// Build the plugin.
    fn build(self, app: &mut App);

    /// Convert into a standard plugin.
    fn into_plugin(self) -> PluginOnceShim<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

/// A plugin for [ActBuilder], which must consumes `self` to build, so this
/// plugin holds it and uses interior mutability.
#[derive(Debug)]
pub struct PluginOnceShim<T: PluginOnce> {
    builder: Mutex<Option<T>>,
}

impl<T: PluginOnce> From<T> for PluginOnceShim<T> {
    fn from(builder: T) -> Self {
        PluginOnceShim {
            builder: Mutex::new(Some(builder)),
        }
    }
}

impl PluginOnce for ActBuilder {
    fn build(self, app: &mut App) {
        let world = app.world_mut();
        let act = self.build(world);
        let keyseqs = act.build_keyseqs(world);
        world.spawn(act).with_children(|builder| {
            for keyseq in keyseqs {
                builder.spawn(keyseq);
            }
        });
    }
}

impl<T: PluginOnce + Sync + Send + 'static> Plugin for PluginOnceShim<T> {
    fn build(&self, app: &mut App) {
        if let Some(builder) = self.builder.lock().expect("plugin once").take() {
            PluginOnce::build(builder, app);
        } else {
            warn!("plugin once shim called a second time");
        }
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

    /// Return the name of this act or [Self::ANONYMOUS].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Build the [KeySequence]s.
    pub fn build_keyseqs(&self, world: &mut World) -> Vec<KeySequence> {
        self.hotkeys
            .iter()
            .map(|hotkey| {
                KeySequence::new(
                    action::send_event(RunActEvent(self.clone())),
                    hotkey.clone(),
                )
                .build(world)
            })
            .collect()
    }
}

impl AsRef<str> for Act {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

// impl Resolve for Vec<Act> {
//     type Item = Act;
//     fn resolve(&self, input: &str) -> Result<Act, LookupError> {
//         let mut matches = self.iter().filter(|command| {
//             command.flags.contains(ActFlags::ExecAct | ActFlags::Active)
//                 && command.name.starts_with(input)
//         });
//         // Collecting and matching is nice expressively. But manually iterating
//         // avoids that allocation.
//         if let Some(first) = matches.next() {
//             if input == first.name() {
//                 Ok(first.clone())
//             } else if let Some(second) = matches.next() {
//                 let mut result = vec![first.name().to_string(), second.name().to_string()];
//                 for item in matches {
//                     result.push(item.name().to_string());
//                 }
//                 Err(LookupError::Incomplete(result))
//             } else {
//                 Err(LookupError::Incomplete(vec![first.name().to_string()]))
//             }
//         } else {
//             Err(LookupError::Message("no matches".into()))
//         }
//     }
// }

// impl Lookup for Vec<Act> {
//     fn look_up(&self, input: &str) -> Result<(), LookupError> {
//         self.resolve(input).map(|_| ())
//     }

//     fn longest_prefix(&self, _input: &str) -> Option<String> {
//         None
//     }
// }

impl bevy::ecs::world::Command for ActBuilder {
    fn apply(self, world: &mut World) {
        let act = self.build(world);
        let keyseqs = act.build_keyseqs(world);
        world.spawn(act).with_children(|builder| {
            for keyseq in keyseqs {
                builder.spawn(keyseq);
            }
        });

        // for hotkey in &act.hotkeys {
        //     let keyseq = KeySequence::new(
        //         action::send_event(RunActEvent(act.clone())),
        //         hotkey.clone(),
        //     );
        //     apply_to_entity(keyseq, id, world);
        //     // <InputSequenceBuilder<KeyChord, ()> as EntityCommand>::apply(keyseq, id, world);
        // }
    }
}

impl bevy::ecs::system::EntityCommand for ActBuilder {
    fn apply(self, id: Entity, world: &mut World) {
        let act = self.build(world);
        let keyseqs = act.build_keyseqs(world);
        let mut entity = world.get_entity_mut(id).unwrap();

        entity.insert(act).with_children(|builder| {
            for keyseq in keyseqs {
                builder.spawn(keyseq);
            }
        });
    }
}

// #[allow(clippy::type_complexity)]
// pub(crate) fn detect_additions(
//     query: Query<(Entity, &Act), (Added<Act>, Without<KeySequence>)>,
//     mut commands: Commands,
// ) {
//     for (id, act) in &query {
//         commands.entity(id).with_children(|builder| {
//             for hotkey in &act.hotkeys {
//                 builder.spawn_empty().add(KeySequence::new(
//                     action::send_event(RunActEvent(act.clone())),
//                     hotkey.clone(),
//                 ));
//             }
//         });
//     }
// }
