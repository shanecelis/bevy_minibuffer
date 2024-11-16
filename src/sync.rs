//! A sync version of the Minibuffer parameter.
use crate::{
    Message,
    Dest,
    lookup::Lookup,
    autocomplete::AutoComplete,
    prompt::GetKeyChord,
    ui::PromptContainer,
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Query, SystemParam, EntityCommands},
        prelude::Commands,
    },
    prelude::DespawnRecursiveExt,
};
use std::fmt::Debug;
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
        if let Some(mut commands) = Dest::ReplaceChildren(dest).get_entity(&mut self.commands) {
            commands
                .construct::<Message>(msg);
        }
    }

    /// Read input from user that must match a [Lookup].
    pub fn read<L>(
        &mut self,
        prompt: impl Into<<TextField as Construct>::Props>,
        lookup: L,
    ) -> EntityCommands
    where
        L: Lookup + Clone + Send + Sync + 'static,
    {
        let dest = self.dest.single();
        let commands = Dest::ReplaceChildren(dest).entity(&mut self.commands);
        let autocomplete = AutoComplete::new(lookup);
        autocomplete.construct(commands, prompt)
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

}
