//! A sync version of the Minibuffer parameter.
use crate::{
    autocomplete::AutoComplete, lookup::Lookup, prompt::GetKeyChord, ui::PromptContainer, Dest,
    Message, prompt::PromptState, view::View,
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        prelude::Commands,
        query::With,
        system::{EntityCommands, Query, SystemParam},
    },
    prelude::{Res, ResMut, NextState, State, DespawnRecursiveExt},
};
use bevy_asky::{prelude::*, sync::AskyCommands, Part};
use std::fmt::Debug;

// #[derive(Resource, Debug, Reflect, Deref)]
// pub struct MinibufferDest(Entity);

/// Minibuffer, a [SystemParam]
#[derive(SystemParam)]
pub struct Minibuffer<'w, 's> {
    /// The query for where the Minibuffer contents go. Expected to be singular.
    pub dest: Query<'w, 's, Entity, With<PromptContainer>>,
    /// Commands
    pub commands: Commands<'w, 's>,
    /// prompt_state
    prompt_state: Res<'w, State<PromptState>>,
    /// next prompt state
    next_prompt_state: ResMut<'w, NextState<PromptState>>,

}

/// I don't know the entity without a query or something.
pub trait MinibufferCommands {

    fn prompt_children<T: Construct + Component + Part> (
        &mut self,
        props: impl IntoIterator<Item = impl Into<T::Props>>,
    ) -> EntityCommands
    where
        <T as Construct>::Props: Send;
}

impl<'w> MinibufferCommands for EntityCommands<'w> {
    fn prompt_children<T: Construct + Component + Part> (
        &mut self,
        props: impl IntoIterator<Item = impl Into<T::Props>>,
    ) -> EntityCommands
    where
        <T as Construct>::Props: Send {
        self.construct_children::<Add<T, View>>(props);
        self.reborrow()
    }
}

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
        self.commands
            .prompt::<Add<T, View>>(props, Dest::ReplaceChildren(dest))
    }

    // pub fn with_prompt<T: Submitter>(
    //     &mut self,
    //     props: impl Into<T::Props>,
    //     f: impl FnOnce(EntityCommands) -> Asyncable<T::Out> + Sync + Send + 'static
    // ) -> Asyncable<T::Out>
    // where
    //     <T as Submitter>::Out: Clone + Debug + Send + Sync + 'static,
    // {



    // }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        let dest = self.dest.single();
        if let Some(mut commands) = Dest::ReplaceChildren(dest).get_entity(&mut self.commands) {
            commands.construct::<Message>(msg);
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

    pub fn visible(&self) -> bool {
        matches!(**self.prompt_state, PromptState::Visible)
    }

    pub fn set_visible(&mut self, show: bool) {
        self.next_prompt_state.set(if show { PromptState::Visible } else { PromptState::Invisible });
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
