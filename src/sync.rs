//! A sync version of the Minibuffer parameter
//!
//! It uses triggers rather than promises.
use crate::{
    acts::ActArg,
    autocomplete::{AutoComplete, Completed, Lookup, LookupMap, RequireMatch},
    event::{RunActByNameEvent, RunActEvent},
    prompt::{GetKeyChord, PromptState},
    ui::PromptContainer,
    view::View,
    Error,
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        prelude::Commands,
        query::With,
        system::{EntityCommands, Query, SystemParam},
    },
    prelude::{DespawnRecursiveExt, NextState, Res, ResMut, State, Text, Trigger},
};
use bevy_asky::{prelude::*, sync::AskyCommands, Dest, Part};
use std::fmt::Debug;

/// Manipulate minibuffer synchronously with this [SystemParam].
///
/// The "synchronized" version of Minibuffer. It uses Bevy's trigger mechanism
/// to communicate outcomes. Like many `SystemParams` it cannot be passed into
/// long-lived closures like async blocks or triggers.
///
/// For async blocks, [MinibufferAsync] is available with the "async" feature
/// flag.
///
/// For trigger blocks, one cannot pass [Minibuffer] into the block, but one can
/// declare a new [Minibuffer] system parameter.
#[derive(SystemParam)]
pub struct Minibuffer<'w, 's> {
    /// The query for where the Minibuffer contents go. Expected to be singular.
    dest: Query<'w, 's, Entity, With<PromptContainer>>,
    /// Commands
    pub commands: Commands<'w, 's>,
    /// prompt_state
    prompt_state: Res<'w, State<PromptState>>,
    /// next prompt state
    next_prompt_state: ResMut<'w, NextState<PromptState>>,
}

/// An [EntityCommands] extension trait
pub trait MinibufferCommands {
    /// Add a collection of children to self.
    fn prompt_children<T: Construct + Component + Part>(
        &mut self,
        props: impl IntoIterator<Item = impl Into<T::Props>>,
    ) -> EntityCommands
    where
        <T as Construct>::Props: Send;
}

impl MinibufferCommands for EntityCommands<'_> {
    fn prompt_children<T: Construct + Component + Part>(
        &mut self,
        props: impl IntoIterator<Item = impl Into<T::Props>>,
    ) -> EntityCommands
    where
        <T as Construct>::Props: Send,
    {
        self.construct_children::<Add0<T, View>>(props);
        self.reborrow()
    }
}

impl Minibuffer<'_, '_> {
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
            .prompt::<Add0<T, View>>(props, Dest::ReplaceChildren(dest))
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
            commands.insert(Text::new(msg));
        }
    }

    /// Request an act be run.
    ///
    /// Returns true if act found and request sent. If given a name for no
    /// corresponding act, it will return false.
    pub fn run_act(&mut self, act: impl Into<ActArg>) {
        match act.into() {
            ActArg::Act(act) => {
                self.commands.trigger(RunActEvent::new(act));
                // self.commands.send_event(RunActEvent::new(act));
                // self.run_act_event.send();
            }
            ActArg::Name(name) => {
                self.commands.trigger(RunActByNameEvent::new(name));
                // self.commands.send_event(RunActByNameEvent::new(name));
                // self.lookup_and_run_act_event.send(RunActByNameEvent::new(name));
            }
        }
    }

    /// Read input from user with autocomplete provided by a [Lookup].
    pub fn prompt_lookup<L>(
        &mut self,
        prompt: impl Into<<TextField as Construct>::Props>,
        lookup: L,
    ) -> EntityCommands
    where
        L: Lookup + Send + Sync + 'static,
    {
        let dest = self.dest.single();
        let commands = Dest::ReplaceChildren(dest).entity(&mut self.commands);
        let autocomplete = AutoComplete::new(lookup);
        autocomplete.construct(commands, prompt)
    }

    /// Read input from user that maps to other another type.
    ///
    /// Instead of triggering [`Submit<String>`] it will trigger [`Completed<T>`].
    pub fn prompt_map<L>(
        &mut self,
        prompt: impl Into<<TextField as Construct>::Props>,
        lookup: L,
    ) -> EntityCommands
    where
        L: Lookup + Clone + LookupMap + Send + Sync + 'static,
        <L as LookupMap>::Item: Sync + Debug,
    {
        let dest = self.dest.single();
        let commands = Dest::ReplaceChildren(dest).entity(&mut self.commands);
        let autocomplete = AutoComplete::new(lookup.clone());
        let mut ecommands = autocomplete.construct(commands, prompt);
        ecommands
            .insert(RequireMatch)
            // TODO: We should probably return something other than submit.
            .observe(
                move |mut trigger: Trigger<Submit<String>>, mut commands: Commands| {
                    let r: Result<L::Item, Error> = trigger
                        .event_mut()
                        .take_result()
                        .map_err(Error::from)
                        .and_then(|s| {
                            // r.map(|x| (x, s))
                            lookup.resolve_res(&s).map_err(Error::from)
                        });
                    commands.trigger_targets(Completed::Unhandled(r), trigger.entity());
                },
            );
        ecommands
    }

    /// Clear the minibuffer.
    pub fn clear(&mut self) {
        let dest = self.dest.single();
        self.commands.entity(dest).despawn_descendants();
    }

    /// Return the visible state.
    pub fn visible(&self) -> bool {
        matches!(**self.prompt_state, PromptState::Visible)
    }

    /// Set the visible state.
    pub fn set_visible(&mut self, show: bool) {
        self.next_prompt_state.set(if show {
            PromptState::Visible
        } else {
            PromptState::Invisible
        });
    }

    /// Get the next key chord.
    pub fn get_chord(&mut self) -> EntityCommands {
        self.commands.spawn(GetKeyChord)
    }
}
