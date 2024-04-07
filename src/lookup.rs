use bevy::input::keyboard::KeyCode;
use bevy_crossbeam_event::CrossbeamEventSender;
use std::borrow::Cow;
use std::io;
use trie_rs::{iter::KeysExt, map};

use crate::event::*;
use crate::Error;
use asky::{
    bevy::KeyEvent, style::Style, utils::renderer::Renderer, OnTick, Printable, SetValue, Tick,
    Typeable, Valuable,
};

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum LookUpError {
    #[error("{0}")]
    Message(Cow<'static, str>),
    #[error("minibuffer {0}")]
    Minibuffer(#[from] Error),
    #[error("incomplete {0:?}")]
    Incomplete(Vec<String>),
}

pub trait LookUp {
    // Object-safe
    fn look_up(&self, input: &str) -> Result<(), LookUpError>;
    fn longest_prefix(&self, input: &str) -> Option<String>;
}

pub trait Resolve {
    // Not object-safe
    type Item: Send;
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

pub struct AutoComplete<T> {
    inner: T,
    look_up: Box<dyn LookUp + Send + Sync>,
    channel: CrossbeamEventSender<DispatchEvent>,
    show_completions: bool,
}

impl<T> AutoComplete<T>
where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    <T as Valuable>::Output: AsRef<str>,
{
    pub fn new<L>(inner: T, look_up: L, channel: CrossbeamEventSender<DispatchEvent>) -> Self
    where
        L: LookUp + Send + Sync + 'static,
    {
        Self {
            inner,
            look_up: Box::new(look_up),
            channel,
            show_completions: false,
        }
    }
}

impl<T> Valuable for AutoComplete<T>
where
    T: Valuable,
{
    type Output = T::Output;
    fn value(&self) -> Result<Self::Output, asky::Error> {
        self.inner.value()
    }
}

impl<T: Tick> Tick for AutoComplete<T> {
    fn tick(&mut self) -> OnTick {
        self.inner.tick()
    }
}

impl<T> Typeable<KeyEvent> for AutoComplete<T>
where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    <T as Valuable>::Output: AsRef<str>,
    // L::Item: Display
{
    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        use crate::lookup::LookUpError::*;
        // let mut hide = true;
        for code in &key.codes {
            if code == &KeyCode::Tab {
                self.show_completions = true;

                if let Ok(input) = self.inner.value() {
                    // What value does the input have?
                    if let Err(e) = self.look_up.look_up(input.as_ref()) {
                        match e {
                            Message(_s) => (), // Err(s),
                            Incomplete(_v) => {
                                if let Some(new_input) = self.look_up.longest_prefix(input.as_ref())
                                {
                                    let _ = self.inner.set_value(new_input);
                                }
                            }
                            Minibuffer(_e) => (), //Err(format!("Error: {:?}", e).into()),
                        }
                    }
                }
                // hide = false;
            }
        }
        // if hide {
        //     self.channel.send(LookUpEvent::Hide);
        // }
        let result = self.inner.handle_key(key);
        if self.show_completions {
            if let Ok(input) = self.inner.value() {
                // What value does the input have?
                match self.look_up.look_up(input.as_ref()) {
                    Ok(_) => self.channel.send(LookUpEvent::Hide),
                    Err(e) => match e {
                        Message(_s) => {
                            // TODO: message should go somewhere.
                            self.channel.send(LookUpEvent::Hide);
                        } // Err(s),
                        Incomplete(v) => self.channel.send(LookUpEvent::Completions(v)),
                        Minibuffer(_e) => (), //Err(format!("Error: {:?}", e).into()),
                    },
                }
            }
        }
        result
    }

    fn will_handle_key(&self, key: &KeyEvent) -> bool {
        for code in &key.codes {
            if code == &KeyCode::Tab {
                return true;
            }
        }
        self.inner.will_handle_key(key)
    }
}

impl<T> Printable for AutoComplete<T>
where
    T: Printable,
{
    fn draw_with_style<R: Renderer>(&self, renderer: &mut R, style: &dyn Style) -> io::Result<()> {
        self.inner.draw_with_style(renderer, style)
    }
}

// pub trait Parse: Debug + Sized {
//     fn parse(input: &str) -> Result<Self, LookUpError>;
// }

// impl Parse for () {
//     fn parse(_: &str) -> Result<Self, LookUpError> {
//         Ok(())
//     }
// }

// impl Parse for String {
//     fn parse(input: &str) -> Result<Self, LookUpError> {
//         Ok(input.to_owned())
//     }
// }

// impl Parse for i32 {
//     fn parse(input: &str) -> Result<Self, LookUpError> {
//         match input.parse::<i32>() {
//             Ok(int) => Ok(int),
//             Err(e) => Err(LookUpError::Message(format!(" expected int: {}", e).into())),
//         }
//     }
// }

// impl<T> LookUp for T
// where
//     T: Parse,
// {
//     type Item = T;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         T::parse(input)
//     }
// }
