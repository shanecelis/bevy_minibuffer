//! A sync version of the Minibuffer parameter
//!
//! It uses triggers rather than promises.
use crate::{
    acts::ActArg,
    autocomplete::{AutoComplete, Lookup, Resolve, Resolved},
    prompt::{GetKeyChord, PromptState},
    ui::PromptContainer,
    view::View,
    event::{LookupAndRunActEvent, RunActEvent},
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
    prelude::{DespawnRecursiveExt, NextState, Res, ResMut, State, TextBundle, TextStyle, Trigger, EventWriter},
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
    // /// Acts available
    // acts: Query<'w, 's, &'static Act>,
    run_act_event: EventWriter<'w, RunActEvent>,
    lookup_and_run_act_event: EventWriter<'w, LookupAndRunActEvent>,
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
            commands.insert(TextBundle::from_section(msg, TextStyle::default()));
            // commands.construct::<Message>(msg);
        }
    }

    /// Request an act be run.
    ///
    /// Returns true if act found and request sent. If given a name for no
    /// corresponding act, it will return false.
    pub fn run_act(&mut self, act: impl Into<ActArg>) {
        match act.into() {
            ActArg::Act(act) => {
                self.run_act_event.send(RunActEvent::new(act));
            }
            ActArg::Name(name) => {
                self.lookup_and_run_act_event.send(LookupAndRunActEvent::new(name));
            }
        }
    }

    /// Read input from user with autocomplete provided by a [Lookup].
    pub fn read<L>(
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
    /// Instead of triggering [`Submit<String>`] it will trigger [`Resolved<T>`].
    pub fn resolve<L>(
        &mut self,
        prompt: impl Into<<TextField as Construct>::Props>,
        lookup: L,
    ) -> EntityCommands
    where
        L: Lookup + Clone + Resolve + Send + Sync + 'static,
        <L as Resolve>::Item: Sync,
    {
        let dest = self.dest.single();
        let commands = Dest::ReplaceChildren(dest).entity(&mut self.commands);
        let autocomplete = AutoComplete::new(lookup.clone());
        let mut ecommands = autocomplete.construct(commands, prompt);
        ecommands
            // .insert(RequireMatch)
            // TODO: We should probably return something other than submit.
            .observe(
                move |mut trigger: Trigger<Submit<String>>, mut commands: Commands| {
                    let mut resolved = Resolved::empty();
                    let r: Result<L::Item, Error> = trigger
                        .event_mut()
                        .take_result()
                        .map_err(Error::from)
                        .and_then(|s| {
                            let r = lookup.resolve_res(&s).map_err(Error::from);
                            resolved.input = Some(s);
                            r
                        });
                    resolved.result = Some(r);
                    commands.trigger_targets(resolved, trigger.entity());
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
