//! A sync version of the Minibuffer parameter.
use crate::{
    Message,
    Dest,
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
    prelude::{Deref, Reflect, Trigger, TextBundle, TextStyle, DespawnRecursiveExt},
    utils::Duration,
};
use bevy_input_sequence::KeyChord;
use std::{borrow::Cow, fmt::Debug};
use bevy_asky::{prelude::*, sync::AskyCommands};

// #[derive(Resource, Debug, Reflect, Deref)]
// pub struct MinibufferDest(Entity);

/// Minibuffer, a [SystemParam]
#[derive(SystemParam)]
pub struct Minibuffer<'w, 's> {
    /// The query for where the Minibuffer contents go. Expected to be singular.
    pub dest: Query<'w, 's, Entity, With<PromptContainer>>,
    /// Commands
    pub commands: Commands<'w, 's>
}

/// I don't know the entity without a query or something.
// pub trait MinibufferCommands {

//     fn prompt<T: Construct + Component + Submitter> (
//         &mut self,
//         props: impl Into<T::Props>,
//     ) -> EntityCommands
//     where
//         <T as Construct>::Props: Send,
//         <T as Submitter>::Out: Clone + Debug + Send + Sync;
// }

// impl<'w, 's> MinibufferCommands for Commands<'w, 's> {
//     fn prompt<T: Construct + Component + Submitter> (
//         &mut self,
//         props: impl Into<T::Props>,
//     ) -> EntityCommands
//     where
//         <T as Construct>::Props: Send,
//         <T as Submitter>::Out: Clone + Debug + Send + Sync {

//     }
// }

impl<'w, 's> Minibuffer<'w, 's> {

    /// Prompt the user for input.
    pub fn prompt<T: Construct + Component + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> EntityCommands
    where
        <T as Construct>::Props: Send,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        let dest = self.dest.single();
        self.commands.prompt::<T, bevy_asky::view::color::View>(props, Dest::ReplaceChildren(dest))
    }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        let msg = msg.into();

        let dest = self.dest.single();
        Dest::ReplaceChildren(dest).entity_commands(&mut self.commands)
            .construct::<Message>(msg);
    }

    /// Read input from user that must match a [LookUp].
    // pub fn read<L>(
    //     &mut self,
    //     prompt: String,
    //     lookup: L,
    // ) -> impl Future<Output = Result<String, Error>> + '_
    // where
    //     L: LookUp + Clone + Send + Sync + 'static,
    // {
    //     use crate::lookup::LookUpError::*;
    //     let mut text = asky::Text::new(prompt);
    //     let l = lookup.clone();
    //     text.validate(move |input| match l.look_up(input) {
    //         Ok(_) => Ok(()),
    //         Err(e) => match e {
    //             Message(s) => Err(s),
    //             // Incomplete(_v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
    //             Incomplete(_v) => Err("Incomplete".into()),
    //             Minibuffer(e) => Err(format!("Error: {:?}", e).into()),
    //         },
    //     });
    //     let text = AutoComplete::new(text, lookup, self.channel.clone());
    //     self.prompt_styled(text, self.style.clone().into())
    // }

    /// Clear the minibuffer.
    pub fn clear(&mut self) {
        let dest = self.dest.single();
        self.commands.entity(dest).despawn_descendants();
    }

    // Wait a certain duration.
    // pub fn delay(&mut self, duration: Duration) -> impl Future<Output = ()> {
    //     self.asky.delay(duration)
    // }

    /// Get the next key chord.
    pub fn get_chord(&mut self) -> EntityCommands {
        self.commands.spawn(GetKeyChord)
    }
}
