//! Acts and their flags, builders, and collections
use crate::acts::Act;
use bevy::prelude::*;
use bevy_input_sequence::KeyChord;
use trie_rs::map::{Trie, TrieBuilder};

/// Maps hotkeys to [Act]s
///
/// This is a trie of hotkeys for better performance and it is only updated when
/// acts with hotkeys are added or removed.
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
