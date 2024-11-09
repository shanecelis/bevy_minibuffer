//! Minibuffer
#![forbid(missing_docs)]
pub mod act;
pub mod event;
// pub mod lookup;
mod param;
mod plugin;
pub mod prompt;
pub mod ui;
pub use param::Minibuffer;
pub use plugin::Config;
pub use plugin::Error;
pub use plugin::MinibufferPlugin;
mod builtin;
mod sink;
pub mod universal;
pub use builtin::Builtin;
pub use sink::{future_sink, future_result_sink};

/// Input, mainly re-exports from [keyseq].
pub mod input {
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

/// Prelude for convenient splat importing, e.g., `use bevy_minibuffer::prelude::*`.
pub mod prelude {
    pub use super::act::{self, Act, ActBuilder, ActsPlugin};
    pub use super::event::RunActEvent;
    pub use super::{future_sink, future_result_sink};
    pub use super::input::{key, keyseq, Modifiers};
    pub use super::Builtin;
    pub use super::Config;
    pub use super::{Error, Minibuffer, MinibufferPlugin};
}
