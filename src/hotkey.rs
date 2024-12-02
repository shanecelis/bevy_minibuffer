//! Hotkey
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

/// A key sequence and an optional alias
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
