//! Lookup and autocompletion
use crate::Error;
use bevy::prelude::*;
use std::{
    hash::Hash,
    collections::HashMap,
    borrow::{Cow, Borrow},
};
use trie_rs::{iter::KeysExt, map};

/// Look up error
///
/// Alternatives to having an exact match for lookup.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum LookupError {
    /// An error message
    #[error("{0}")]
    Message(Cow<'static, str>),
    /// No matches
    #[error("No matches")]
    NoMatch,
    #[error("One match: {0}")]
    /// One match
    OneMatch(String),
    #[error("Many matches")]
    /// Many matches
    ManyMatches,
}

/// Look up possible completions
///
/// NOTE: This trait is object-safe.
pub trait Lookup {
    /// Look up the `input`. If it matches exactly, this returns `Ok(())`.
    /// Otherwise it returns [LookupError], which can include its partial matches.
    fn lookup(&self, input: &str) -> Result<(), LookupError>;
    /// Return the longest prefix for `input`.
    fn longest_prefix(&self, input: &str) -> Option<String>;
    /// Return all matches for `input`.
    fn all_lookups(&self, input: &str) -> Vec<String>;
}

/// LookupMap the input to a value of type `Item`.
///
/// NOTE: This trait is not object-safe.
pub trait LookupMap: Lookup {
    /// The type this resolves to.
    type Item: Send;
    /// LookupMap the `input`.
    fn resolve(&self, input: &str) -> Option<Self::Item>;

    /// LookupMap the `input` or provide an error.
    fn resolve_res(&self, input: &str) -> Result<Self::Item, LookupError> {
        self.resolve(input).ok_or_else(|| match self.lookup(input) {
            Ok(()) => {
                LookupError::Message("Inconsistent: LookupMap failed but lookup succeeded.".into())
            }
            Err(e) => e,
        })
    }
}

/// Triggered from `.resolve()` with value `T` and input string
#[derive(Event, Debug)]
pub enum Completed<T> {
    /// A completion event with its associated input if available
    Unhandled(Result<T, Error>),
    /// This completion event has been handled.
    Handled,
}

impl<T> Completed<T> {
    /// Create a new completed event.
    pub fn new(value: Result<T, Error>) -> Self {
        Self::Unhandled(value)
    }

    /// Take this completed and leave `Completed::Handled`.
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Completed::Handled)
    }

    /// Return the result; leave a Handled event in its place.
    ///
    /// WARNING: Cannot get input after this. Use `take_inner()` to get result
    /// and input.
    pub fn take_result(&mut self) -> Option<Result<T, Error>> {
        match std::mem::replace(self, Completed::Handled) {
            Completed::Unhandled(r) => Some(r),
            Completed::Handled => None,
        }
    }

}

impl<V: Send + Sync + Clone> LookupMap for map::Trie<u8, V> {
    type Item = V;

    fn resolve(&self, input: &str) -> Option<Self::Item> {
        self.exact_match(input).cloned()
    }
}

fn iter_to_error(mut matches: impl Iterator<Item = impl AsRef<str>>) -> LookupError {
    if let Some(one_match) = matches.next() {
        if matches.next().is_none() {
            LookupError::OneMatch(one_match.as_ref().to_string())
        } else {
            LookupError::ManyMatches
        }
    } else {
        LookupError::NoMatch
    }
}

impl<V: Send + Sync + Clone> Lookup for map::Trie<u8, V> {
    fn lookup(&self, input: &str) -> Result<(), LookupError> {
        if self.exact_match(input).is_some() {
            return Ok(());
        }
        let matches = self
            .predictive_search::<String, trie_rs::try_collect::StringCollect>(input)
            .keys();
        Err(iter_to_error(matches))
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        map::Trie::<u8, V>::longest_prefix(self, input)
    }

    fn all_lookups(&self, input: &str) -> Vec<String> {
        self.predictive_search(input).keys().collect()
    }
}

// // Why have this?
// impl LookupMap for trie_rs::Trie<u8> {
//     type Item = ();

//     fn resolve(&self, input: &str) -> Result<Self::Item, LookupError> {
//         self.0.lookup(input)
//     }
// }

impl Lookup for trie_rs::Trie<u8> {
    fn lookup(&self, input: &str) -> Result<(), LookupError> {
        // self.exact_match(input).cloned()
        // self.0.resolve(input)
        if self.exact_match(input) {
            return Ok(());
        }
        let mut iter = self.predictive_search::<String, trie_rs::try_collect::StringCollect>(input);
        if let Some(x) = iter.next() {
            if iter.next().is_none() {
                Err(LookupError::OneMatch(x))
            } else {
                Err(LookupError::ManyMatches)
            }
        } else {
            Err(LookupError::NoMatch)
        }
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        self.0.longest_prefix(input)
    }

    fn all_lookups(&self, input: &str) -> Vec<String> {
        self.0.predictive_search(input).keys().collect()
    }
}

impl<T: AsRef<str>> Lookup for Vec<T> {
    fn lookup(&self, input: &str) -> Result<(), LookupError> {
        self[..].lookup(input)
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        self[..].longest_prefix(input)
    }

    fn all_lookups(&self, input: &str) -> Vec<String> {
        self.iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input))
            .map(|word| word.to_string())
            .collect()
    }
}

impl<T: AsRef<str>> LookupMap for Vec<T> {
    type Item = String;
    fn resolve(&self, input: &str) -> Option<Self::Item> {
        self[..].resolve(input)
    }
}

/// Handles arrays of &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> LookupMap for [T] {
    type Item = String;
    fn resolve(&self, input: &str) -> Option<Self::Item> {
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.

        // let matches: Vec<&str> = self
        //     .iter()
        //     .map(|word| word.as_ref())
        //     .filter(|word| word.starts_with(input))
        //     .collect();
        // match matches[..] {
        //     [a] => Ok(a.to_string()),
        //     [_a, _b, ..] => Err(LookupError::Incomplete(
        //         matches.into_iter().map(|s| s.to_string()).collect(),
        //     )),
        //     [] => Err(LookupError::Message(" no matches".into())),
        // }

        let mut matches = self
            .iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input));

        if let Some(first) = matches.next() {
            if matches.next().is_none() {
                return Some(first.to_string());
            }
        }
        None
    }
}

impl<K: Borrow<str> + AsRef<str> + Hash + Eq, V: Clone + Send> LookupMap for HashMap<K,V> {
    type Item = V;
    fn resolve(&self, input: &str) -> Option<Self::Item> {
        self.get(input).cloned()
    }
}

impl<K: Borrow<str> + AsRef<str> + Hash + Eq, V> Lookup for HashMap<K, V> {
    fn lookup(&self, input: &str) -> Result<(), LookupError> {
        if self.contains_key(input) {
            Ok(())
        } else {
            Err(iter_to_error(self.keys().filter(|k| k.as_ref().starts_with(input))))
        }
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        // XXX: Don't love this.
        let v: Vec<_> = self.keys().collect();
        v.as_slice().longest_prefix(input)
    }

    fn all_lookups(&self, input: &str) -> Vec<String> {
        self.keys()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input))
            .map(|word| word.to_string())
            .collect()
    }
}

impl<T: AsRef<str>> Lookup for [T] {
    fn lookup(&self, input: &str) -> Result<(), LookupError> {
        let mut one_match = None;
        for x in self {
            let x = x.as_ref();
            if x == input {
                return Ok(());
            }
            if x.starts_with(input) {
                if one_match.is_none() {
                    one_match = Some(x.to_string());
                } else {
                    return Err(LookupError::ManyMatches);
                }
            }
        }
        if let Some(one_match) = one_match {
            Err(LookupError::OneMatch(one_match))
        } else {
            Err(LookupError::NoMatch)
        }
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        let mut accum: Option<String> = None;
        let count = input.chars().count();
        let mut entries: Vec<_> =
            self
            .into_iter()
            .filter_map(|s| {
                let s = s.as_ref();
                s.starts_with(input).then(|| s.chars().skip(count))
            })
            .collect();
        let mut a_match = false;
        loop {
            let mut c: Option<char> = None;
            for entry in &mut entries {
                a_match = true;
                if let Some(d) = entry.next() {
                    if let Some(a) = c {
                        if a != d {
                            c = None;
                            break;
                        }
                    } else {
                        c = Some(d);
                    }
                } else {
                    c = None;
                    break;
                }
            }

            if let Some(c) = c {
                if let Some(ref mut s) = accum {
                    s.push(c);
                } else {
                    let mut s = String::from(input);
                    s.push(c);
                    accum = Some(s);
                }
            } else {
                break;
            }
        }
        accum.or_else(|| a_match.then(|| String::from(input)))
    }

    fn all_lookups(&self, input: &str) -> Vec<String> {
        self.iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input))
            .map(|word| word.to_string())
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lookup_slice() {
        let a = ["abc", "abcd", "abcde"];
        assert_eq!(a[..].longest_prefix(""), Some(String::from("abc")));
        assert_eq!(a[..].longest_prefix("a"), Some(String::from("abc")));
        assert_eq!(a[..].longest_prefix("ab"), Some(String::from("abc")));
        assert_eq!(a[..].longest_prefix("abcd"), Some(String::from("abcd")));
        assert_eq!(a[..].longest_prefix("abcde"), Some(String::from("abcde")));
        assert_eq!(a[..].longest_prefix("abcdef"), None);
        assert_eq!(a[..].longest_prefix("e"), None);
    }

    #[test]
    fn lookup_array() {
        let a = ["abc", "abcd", "abcde"];
        assert_eq!(
            ["abc", "abcd", "abcde"].longest_prefix(""),
            Some(String::from("abc"))
        );
        assert_eq!(a.longest_prefix(""), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("a"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("ab"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("abcd"), Some(String::from("abcd")));
        assert_eq!(a.longest_prefix("abcde"), Some(String::from("abcde")));
        assert_eq!(a.longest_prefix("abcdef"), None);
        assert_eq!(a.longest_prefix("e"), None);
    }

    #[test]
    fn lookup_vec() {
        let a = vec!["abcd", "abc", "abcde"];
        assert_eq!(a.longest_prefix(""), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("a"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("ab"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("abcd"), Some(String::from("abcd")));
        assert_eq!(a.longest_prefix("abcde"), Some(String::from("abcde")));
        assert_eq!(a.longest_prefix("abcdef"), None);
        assert_eq!(a.longest_prefix("e"), None);
    }

    #[test]
    fn lookup_map() {
        let mut a = HashMap::new();
        a.insert("abc", 0);
        a.insert("abcd", 1);
        a.insert("abcde", 2);
        // let v: Vec<_> = a.keys().map(|s| s.to_string()).collect();
        // assert_eq!(v, vec!["abc", "abcd", "abcde"]);
        assert_eq!(a.len(), 3);
        assert_eq!(a.longest_prefix(""), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("a"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("ab"), Some(String::from("abc")));
        assert_eq!(a.longest_prefix("abcd"), Some(String::from("abcd")));
        assert_eq!(a.longest_prefix("abcde"), Some(String::from("abcde")));
        assert_eq!(a.longest_prefix("abcdef"), None);
        assert_eq!(a.longest_prefix("e"), None);
    }
}
