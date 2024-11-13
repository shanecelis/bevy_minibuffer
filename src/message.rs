use crate::{
    event::DispatchEvent,
    // lookup::{AutoComplete, LookUp},
    prompt::{KeyChordEvent, GetKeyChord},
    ui::PromptContainer,
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Query, Res, SystemMeta, SystemParam, SystemState, Resource, EntityCommands},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
        prelude::Commands,
    },
    prelude::{Deref, Reflect, Trigger, TextBundle, TextStyle},
    utils::Duration,
};
use bevy_defer::AsyncWorld;
use bevy_input_sequence::KeyChord;
use std::{borrow::Cow, fmt::Debug};
use bevy_asky::prelude::*;
use futures::{channel::oneshot, Future};

/// A message marker to put a text message in the minibuffer.
#[derive(Component, Debug, Reflect)]
pub struct Message;

impl Construct for Message {
    type Props = Cow<'static, str>;

    fn construct(
        context: &mut ConstructContext,
        props: Self::Props,
    ) -> Result<Self, ConstructError> {
        // Our requirements.
        let mut commands = context.world.commands();
        commands
            .entity(context.id)
            .insert(TextBundle::from_section(props, TextStyle::default()));
        context.world.flush();
        Ok(Message)
    }
}
