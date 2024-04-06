// #![feature(return_position_impl_trait_in_trait)]
// #![forbid(missing_docs)]
#![allow(incomplete_features)]
pub mod commands;
pub mod prompt;
mod style;
pub mod tasks;
pub mod ui;
mod plugin;
pub mod event;
pub mod lookup;
pub use plugin::MinibufferPlugin;
pub use plugin::ConsoleConfig;
pub use plugin::Error;

pub use prompt::Minibuffer;
pub use style::MinibufferStyle;

pub mod input {
    pub use keyseq::{
        bevy::{pkey as key, pkeyseq as keyseq},
        Modifiers,
    };
}

pub mod prelude {
    pub use super::{Minibuffer, Error, MinibufferPlugin};
    pub use super::input::*;
    pub use super::event::StartActEvent;
    pub use super::commands::{Act, AddAct};
    pub use super::ConsoleConfig;
    pub use asky::bevy::future_sink;
}
