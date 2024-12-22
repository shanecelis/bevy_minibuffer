//! Acts and their flags, builders, and collections
use crate::acts::{Act, ActRef, ActFlags};
use bevy::prelude::*;
use bevy_input_sequence::KeyChord;
use trie_rs::map::{Trie, TrieBuilder};
use std::collections::HashMap;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<HotkeyActCache>()
        .init_resource::<NameActCache>();
}

#[derive(Resource, Default)]
pub struct NameActCache {
    trie: HashMap<ActFlags, Trie<u8, ActRef>>,
}

impl NameActCache {
    /// Retrieve the cached trie without iterating through `acts`. Or if the
    /// cache has been invalidated, build and cache a new trie using the
    /// `acts` iterator.
    pub fn trie<'a>(&mut self, acts: impl Iterator<Item = (Entity, &'a Act)>, flags: ActFlags) -> &Trie<u8, ActRef> {
        self.trie.entry(flags).or_insert_with(|| {
            let mut builder: TrieBuilder<u8, ActRef> = TrieBuilder::new();
            for (id, act) in acts {
                if act.flags.contains(flags) {
                    builder.push(act.name.as_ref(), ActRef::from_act(act, id));
                }
            }
            builder.build()
        })
    }

    /// Invalidate the cache.
    pub fn invalidate(&mut self, flags: Option<ActFlags>) {
        if let Some(flags) = flags {
            self.trie.remove(&flags);
        } else {
            self.trie.clear();
        }
    }
}

/// Maps hotkeys to [Act]s
///
/// This is a trie of hotkeys for better performance and it is only updated when
/// acts with hotkeys are added or removed.
#[derive(Resource, Default)]
pub struct HotkeyActCache {
    trie: Option<Trie<KeyChord, Act>>,
}

impl HotkeyActCache {
    /// Retrieve the cached trie without iterating through `acts`. Or if
    /// the cache has been invalidated, build and cache a new trie using the
    /// `acts` iterator.
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
