// #![feature(return_position_impl_trait_in_trait)]
// #![forbid(missing_docs)]
#![allow(incomplete_features)]
pub mod commands;
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

pub mod input {
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

pub mod prelude {
    pub use super::commands::{Act, AddAct};
    pub use super::event::StartActEvent;
    pub use super::input::*;
    pub use super::ConsoleConfig;
    pub use super::{Error, Minibuffer, MinibufferPlugin};
    pub use asky::bevy::future_sink;
}
