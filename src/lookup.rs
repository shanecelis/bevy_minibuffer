//! Lookup and autocompletion
use bevy::prelude::*;
use std::borrow::Cow;
use std::io;
use trie_rs::{iter::KeysExt, map};

use crate::event::*;
use crate::Error;

/// Look up error
///
/// Alternatives to having an exact match for lookup.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum LookUpError {
    /// An error message
    #[error("{0}")]
    Message(Cow<'static, str>),
    /// An minibuffer error
    #[error("minibuffer {0}")]
    Minibuffer(#[from] Error),
    /// A list of possible matches
    #[error("incomplete {0:?}")]
    Incomplete(Vec<String>),
}

/// Look up possible completions
///
/// This trait is object-safe.
pub trait LookUp {
    /// Look up the `input`.
    fn look_up(&self, input: &str) -> Result<(), LookUpError>;
    /// Return the longest prefix for `input`.
    fn longest_prefix(&self, input: &str) -> Option<String>;
}

/// Resolve the input to a particular kind of item.
///
/// This trait is not object-safe.
pub trait Resolve {
    /// The type this resolves to.
    type Item: Send;
    /// Resolve the `input` or provide an error.
    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<V: Send + Sync + Clone> Resolve for map::Trie<u8, V> {
    type Item = V;

    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
        if let Some(value) = self.exact_match(input) {
            return Ok(value.clone());
        }
        let matches: Vec<String> = self.predictive_search(input).keys().collect();
        match matches.len() {
            0 => Err(LookUpError::Message("no matches".into())),
            // 1 =>
            //     if matches[0] == input {
            //         Ok(self.exact_match(input).cloned().unwrap())
            //     } else {
            //         Err(LookUpError::Incomplete(matches))
            //     },
            _ => Err(LookUpError::Incomplete(matches)),
        }
    }
}

impl<V: Send + Sync + Clone> LookUp for map::Trie<u8, V> {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        map::Trie::<u8, V>::longest_prefix(self, input)
    }
}

impl Resolve for trie_rs::Trie<u8> {
    type Item = ();

    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
        self.0.look_up(input)
    }
}

impl LookUp for trie_rs::Trie<u8> {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.0.resolve(input)
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        self.0.longest_prefix(input)
    }
}

/// Handles arrays of &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> Resolve for &[T] {
    type Item = String;
    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.

        // let matches: Vec<&str> = self
        //     .iter()
        //     .map(|word| word.as_ref())
        //     .filter(|word| word.starts_with(input))
        //     .collect();
        // match matches[..] {
        //     [a] => Ok(a.to_string()),
        //     [_a, _b, ..] => Err(LookUpError::Incomplete(
        //         matches.into_iter().map(|s| s.to_string()).collect(),
        //     )),
        //     [] => Err(LookUpError::Message(" no matches".into())),
        // }

        let mut matches = self
            .iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input));

        if let Some(first) = matches.next() {
            if let Some(second) = matches.next() {
                let mut result = vec![first.to_string(), second.to_string()];
                for item in matches {
                    result.push(item.to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else if input == first {
                Ok(first.to_string())
            } else {
                Err(LookUpError::Incomplete(vec![first.to_string()]))
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }
}

impl<T: AsRef<str>> LookUp for &[T] {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, _input: &str) -> Option<String> {
        todo!();
    }
}
