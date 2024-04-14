//! Minibuffer
// #![forbid(missing_docs)]
pub mod act;
pub mod event;
pub mod lookup;
mod param;
mod plugin;
pub mod prompt;
mod style;
pub mod ui;
pub use param::Minibuffer;
pub use plugin::Config;
pub use plugin::Error;
pub use plugin::MinibufferPlugin;
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
    pub use super::act::{self, Act, AddAct};
    pub use super::event::RunActEvent;
    pub use super::input::{key, keyseq, Modifiers};
    pub use super::Config;
    pub use super::{Error, Minibuffer, MinibufferPlugin};
    pub use asky::bevy::future_sink;
}
