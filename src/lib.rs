//! Minibuffer
#![forbid(missing_docs)]
pub mod act;
pub mod event;
// pub mod lookup;
#[cfg(feature = "async")]
mod future;
mod message;
pub mod sync;
mod plugin;
pub mod prompt;
pub mod ui;
pub use plugin::Config;
pub use plugin::Error;
pub use plugin::MinibufferPlugin;
pub use plugin::MinibufferPlugins;
#[cfg(feature = "async")]
mod sink;
#[cfg(feature = "async")]
pub mod universal;
#[cfg(feature = "async")]
mod builtin;
#[cfg(feature = "async")]
pub use builtin::Builtin;
#[cfg(feature = "async")]
pub use sink::{future_sink, future_result_sink};
pub use message::Message;
pub use bevy_asky::Dest;

#[cfg(feature = "async")]
pub use future::Minibuffer;
#[cfg(not(feature = "async"))]
pub use sync::Minibuffer;
/// Input, mainly re-exports from [keyseq].
pub mod input {
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

/// Prelude for convenient splat importing, e.g., `use bevy_minibuffer::prelude::*`.
pub mod prelude {
    pub use bevy_asky as asky;
    pub use asky::{prompt::*, AskyEvent};
    pub use super::act::{self, Act, ActBuilder, ActsPlugin, PluginOnce};
    pub use super::event::RunActEvent;
    #[cfg(feature = "async")]
    pub use super::{future_sink, future_result_sink};
    pub use super::input::{key, keyseq, Modifiers};
    #[cfg(feature = "async")]
    pub use super::Builtin;
    pub use super::Config;
    pub use super::{Error, MinibufferPlugin, MinibufferPlugins};
    pub use super::Minibuffer;
    pub use super::Dest;
}
