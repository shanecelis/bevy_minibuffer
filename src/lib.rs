//! Minibuffer
#![forbid(missing_docs)]
pub mod act;
pub mod event;
pub mod lookup;
mod plugin;
pub mod prompt;
mod style;
pub mod task;
pub mod ui;
pub use plugin::ConsoleConfig;
pub use plugin::Error;
pub use plugin::MinibufferPlugin;

pub use prompt::Minibuffer;
pub use style::MinibufferStyle;

/// Input, mainly re-exports from [keyseq].
pub mod input {
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

/// Prelude for convenient splat importing, e.g., `use bevy_minibuffer::prelude::*`.
pub mod prelude {
    pub use super::act::{Act, AddAct};
    pub use super::event::StartActEvent;
    pub use super::input::*;
    pub use super::ConsoleConfig;
    pub use super::{Error, Minibuffer, MinibufferPlugin};
    pub use asky::bevy::future_sink;
}
