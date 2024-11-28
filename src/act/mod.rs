//! acts, or commands
use crate::event::RunActEvent;
use bevy::{
    ecs::system::{BoxedSystem, SystemId},
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
use trie_rs::map::{Trie, TrieBuilder};
mod acts;
pub use acts::{Acts, ActsPlugin, AddActs};

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
    /// Act flags
    #[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct ActFlags: u8 {
        /// Act is active.
        const Active       = 0b00000001;
        /// Act is shown in [crate::act::exec_act].
        const ExecAct      = 0b00000010;
        /// Act usually runs another act like exec_act.
        const Adverb       = 0b00000100;
        /// Act shows the minibuffer.
        const Show         = 0b00001000;
    }
}

/// Hotkey is a key sequence and optionally an alias.
#[derive(Debug, Clone, Reflect)]
pub struct Hotkey {
    /// Key chord sequence
    pub chords: Vec<KeyChord>,
    /// Alias
    pub alias: Option<Cow<'static, str>>,
}

impl Hotkey {
    /// New hotkey from any [KeyChord]-able sequence.
    pub fn new<T>(chords: impl IntoIterator<Item = T>) -> Self
    where
        KeyChord: From<T>,
    {
        Self {
            chords: chords.into_iter().map(|v| v.into()).collect(),
            alias: None,
        }
    }

    /// Define an alias.
    pub fn alias(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.alias = Some(name.into());
        self
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(alias) = &self.alias {
            write!(f, "{}", alias)
        } else {
            for key_chord in &self.chords {
                write!(f, "{} ", key_chord)?;
            }
            Ok(())
        }
    }
}

impl Default for ActFlags {
    fn default() -> Self {
        ActFlags::Active | ActFlags::ExecAct
    }
}

/// Act, a command in `bevy_minibuffer`
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
                    builder.insert(hotkey.chords.clone(), act.clone());
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
    pub hotkeys: Vec<Hotkey>,
    pub(crate) system: Option<BoxedSystem>,
    /// Flags for this act
    pub flags: ActFlags,
    /// Shorten the name to just the first system.
    pub shorten_name: bool,
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
            system: Some(Box::new(IntoSystem::into_system(system))),
            flags: ActFlags::Active | ActFlags::ExecAct,
            shorten_name: true,
        }
    }

    /// Return the name of the act. Derived from system if not explicitly given.
    pub fn name(&self) -> Cow<'static, str> {
        self.name.clone().unwrap_or_else(|| {
            let mut n = self.system.as_ref().expect("system").name();
            // Take name out of pipe.
            //
            // "Pipe(cube_async::speed, bevy_minibuffer::sink::future_result_sink<(), bevy_minibuffer::plugin::Error, cube_async::speed::{{closure}}>)"
            // -> "cube_async::speed"
            n = n
                .find('(')
                .and_then(|start| {
                    n.find([',', ' ', ')'])
                        .map(|end| n[start + 1..end].to_owned().into())
                })
                .unwrap_or(n);
            if self.shorten_name {
                n = n
                    .rfind(':')
                    .map(|start| n[start + 1..].to_owned().into())
                    .unwrap_or(n);
            }
            n
        })
    }

    /// Build [Act].
    pub fn build(self, world: &mut World) -> Act {
        Act {
            name: self.name(),
            hotkeys: self.hotkeys,
            flags: self.flags,
            system_id: world.register_boxed_system(self.system.expect("system")),
        }
    }

    /// Name the act.
    pub fn named(&mut self, name: impl Into<Cow<'static, str>>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    /// Add a hotkey.
    pub fn hotkey<T>(&mut self, hotkey: impl IntoIterator<Item = T>) -> &mut Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys.push(Hotkey::new(hotkey));
        self
    }

    /// Add a hotkey with an alias.
    pub fn hotkey_named<T>(
        &mut self,
        hotkey: impl IntoIterator<Item = T>,
        name: impl Into<Cow<'static, str>>,
    ) -> &mut Self
    where
        KeyChord: From<T>,
    {
        self.hotkeys.push(Hotkey::new(hotkey).alias(name));
        self
    }

    /// Set flags.
    pub fn flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Add the given the flags.
    pub fn add_flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags |= flags;
        self
    }

    /// Subtracts the given the flags.
    pub fn sub_flags(&mut self, flags: ActFlags) -> &mut Self {
        self.flags -= flags;
        self
    }
}

impl From<&mut ActBuilder> for ActBuilder {
    fn from(builder: &mut ActBuilder) -> Self {
        Self {
            name: builder.name.take(),
            system: builder.system.take(),
            hotkeys: std::mem::take(&mut builder.hotkeys),
            flags: builder.flags,
            shorten_name: builder.shorten_name,
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
            .enumerate()
            .map(|(i, hotkey)| {
                KeySequence::new(
                    action::send_event(RunActEvent::new(self.clone()).hotkey(i)),
                    hotkey.chords.clone(),
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
