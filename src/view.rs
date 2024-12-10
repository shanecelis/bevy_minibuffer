//! Implements Asky's views for Minibuffer prompts
#![allow(clippy::type_complexity)]
use bevy::{
    ecs::{query::QueryEntityError, system::SystemParam},
    prelude::*,
};
use bevy_asky::{construct::*, prelude::*, string_cursor::*, view::color};

pub use color::View;

pub use color::plugin_no_focus as plugin;
pub use color::text_view;
