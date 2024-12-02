#![doc(html_root_url = "https://docs.rs/bevy_minibuffer/0.1.0")]
#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]
pub mod acts;
pub mod autocomplete;
pub mod event;
#[cfg(feature = "async")]
mod future;
mod plugin;
pub mod prompt;
mod sync;
mod ui;
pub use plugin::Config;
pub use plugin::Error;
pub use plugin::MinibufferPlugin;
pub use plugin::MinibufferPlugins;
pub use autocomplete::Resolved;
pub mod view;
#[cfg(feature = "async")]
pub mod sink;
#[cfg(feature = "async")]
pub use future::MinibufferAsync;
pub use sync::Minibuffer;
mod hotkey;

/// Input, mainly re-exports from [keyseq]
pub mod input {
    pub use bevy_input_sequence::KeyChord;
    pub use super::hotkey::*;
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

/// Prelude for convenient splat importing, e.g., `use bevy_minibuffer::prelude::*`.
pub mod prelude {
    pub use super::acts::{self, Act, ActBuilder, ActFlags, Acts, ActsPlugin, AddActs};
    pub use super::event::RunActEvent;
    pub use super::input::{key, keyseq, Modifiers};
    pub use super::autocomplete::*;
    pub use super::sync::MinibufferCommands;
    #[cfg(feature = "async")]
    pub use super::acts::universal::*;
    pub use super::acts::basic::BasicActs;
    pub use super::Config;
    pub use super::Minibuffer;
    #[cfg(feature = "async")]
    pub use super::MinibufferAsync;
    #[cfg(feature = "async")]
    pub use super::sink::{future_result_sink, future_sink};
    pub use super::{Error, MinibufferPlugin, MinibufferPlugins, Resolved};
    pub use super::prompt::*;
    pub use std::time::Duration;
}
