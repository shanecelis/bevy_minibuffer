//! Hotkey
use bevy::prelude::*;
use bevy_input_sequence::KeyChord;
use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        // Write
    },
};

/// A key sequence and an optional alias
#[derive(Debug, Clone, Reflect)]
pub struct Hotkey {
    /// Key chord sequence
    pub chords: Vec<KeyChord>,
    /// Alias
    pub alias: Option<Cow<'static, str>>,
}

impl PartialEq<[KeyChord]> for Hotkey {
    fn eq(&self, other: &[KeyChord]) -> bool {
        self.chords == other
    }
}

// impl PartialEq<[&KeyChord]> for Hotkey {
//     fn eq(&self, other: &[&KeyChord]) -> bool {
//         self.chords == *other
//     }
// }

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

    /// Return an empty hotkey.
    pub fn empty() -> Self {
        Self {
            chords: Vec::new(),
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
            let mut iter = self.chords.iter();
            if let Some(first) = iter.next() {
                write!(f, "{}", first)?;
            }
            for key_chord in iter {
                write!(f, " {}", key_chord)?;
            }
            Ok(())
        }
    }
}
