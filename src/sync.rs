//! A sync version of the Minibuffer parameter.
use crate::{
    Message,
    Dest,
    event::DispatchEvent,
    lookup::LookUp,
    autocomplete::AutoComplete,
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
#[cfg(feature = "async")]
use futures::{channel::oneshot, Future};
#[cfg(feature = "async")]
use bevy_defer::AsyncWorld;

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

    #[cfg(feature = "async")]
    /// Prompt the user for input.
    pub fn prompt_async<T: Construct + Component + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        let dest = self.dest.single();
        Asky::default().prompt::<T, bevy_asky::view::color::View>(props, Dest::ReplaceChildren(dest))
    }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        let msg = msg.into();

        let dest = self.dest.single();
        if let Some(mut commands) = Dest::ReplaceChildren(dest).get_entity(&mut self.commands) {
            commands
                .construct::<Message>(msg);
        }
    }

    /// Read input from user that must match a [LookUp].
    pub fn read<L>(
        &mut self,
        prompt: impl Into<<TextField as Construct>::Props>,
        lookup: L,
    ) -> EntityCommands
    where
        L: LookUp + Clone + Send + Sync + 'static,
    {
        use crate::lookup::LookUpError::*;
        let mut commands = self.prompt::<TextField>(prompt);
        commands
            .insert(AutoComplete::new(lookup));
        commands

        // // let mut text = asky::Text::new(prompt);
        // let l = lookup.clone();
        // text.validate(move |input| match l.look_up(input) {
        //     Ok(_) => Ok(()),
        //     Err(e) => match e {
        //         Message(s) => Err(s),
        //         // Incomplete(_v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
        //         Incomplete(_v) => Err("Incomplete".into()),
        //         Minibuffer(e) => Err(format!("Error: {:?}", e).into()),
        //     },
        // });
        // let text = AutoComplete::new(text, lookup, self.channel.clone());
        // self.prompt_styled(text, self.style.clone().into())
    }

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

    #[cfg(feature = "async")]
    /// Get the next key chord.
    pub fn get_chord_async(&mut self) -> impl Future<Output = Result<KeyChord, Error>> {
        async {
            let (promise, waiter) = oneshot::channel::<Result<KeyChord, Error>>();
            let mut promise = Some(promise);
            let async_world = AsyncWorld::new();
            async_world.apply_command(move |world: &mut World| {
                let mut commands = world.commands();
                commands.spawn(GetKeyChord)
                    .observe(move |trigger: Trigger<KeyChordEvent>| {
                        if let Some(promise) = promise.take() {
                            promise.send(Ok(trigger.event().0.clone())).expect("send");
                        }
                    });
            });
            waiter.await?
        }
    }
}
