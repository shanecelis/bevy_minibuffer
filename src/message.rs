use bevy::{
    ecs::component::Component,
    prelude::{Reflect, TextBundle, TextStyle},
};
use bevy_asky::prelude::*;
use std::{borrow::Cow, fmt::Debug};

/// A message marker to put a text message in the minibuffer.
#[derive(Component, Debug, Reflect)]
pub(crate) struct Message;

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
