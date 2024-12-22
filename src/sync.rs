//! A sync version of the Minibuffer parameter
//!
//! It uses triggers rather than promises.
use crate::{
    acts::{ActArg, tape::{DebugMap, TapeRecorder}},
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
    prelude::{DespawnRecursiveExt, NextState, Res, ResMut, State, Text, Trigger, TextLayout, LineBreak, default},
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
    /// macro state
    pub(crate) tape_recorder: ResMut<'w, TapeRecorder>,
    pub(crate) debug_map: ResMut<'w, DebugMap>,
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
            commands
                .insert(Text::new(msg))
                .insert(TextLayout {
                    linebreak: LineBreak::WordOrCharacter,
                    ..default()
                });
        }
    }

    /// Request an act be run.
    pub fn run_act(&mut self, act: impl Into<ActArg>) {
        match act.into() {
            ActArg::ActRef(act) => {
                self.commands.trigger(RunActEvent::new(act));
            }
            ActArg::Name(name) => {
                self.commands.trigger(RunActByNameEvent::new(name));
                // self.commands.send_event(RunActByNameEvent::new(name));
                // self.lookup_and_run_act_event.send(RunActByNameEvent::new(name));
            }
        }
    }

    /// Request an act be run.
    ///
    /// The input type cannot be determined at compile-time, so _you_ must
    /// ensure it is correct. Consider using type suffixes for numerical
    /// literals. For instance `Some(2.0)` is an `Option<f64>` without any
    /// further indication not an `Option<f32>`; use `Some(2.0f32)` if you want
    /// the latter.
    pub fn run_act_with_input<I: Send + Sync + Debug + 'static>(&mut self, act: impl Into<ActArg>, input: I) {
        match act.into() {
            ActArg::ActRef(act) => {
                self.commands.trigger(RunActEvent::new_with_input(act, input));
            }
            ActArg::Name(name) => {
                self.commands.trigger(RunActByNameEvent::new_with_input(name, input));
                // self.commands.send_event(RunActByNameEvent::new(name));
                // self.lookup_and_run_act_event.send(RunActByNameEvent::new(name));
            }
        }
    }

    pub fn log_input<I: Debug + Clone + Send + Sync + 'static>(&mut self, input: &I) {
        self.tape_recorder.process_input(input, &mut *self.debug_map);
        match *self.tape_recorder {
            TapeRecorder::Record { ref mut tape, .. } => {
                tape.ammend_input(input.clone(), &mut *self.debug_map);
            }
            _ => ()
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
            // TODO: We should probably return the input string in either case.
            .observe(
                move |mut trigger: Trigger<Submit<String>>, mut commands: Commands| {
                    let mut input = None;
                    let result: Result<L::Item, Error> = trigger
                        .event_mut()
                        .take_result()
                        .map_err(Error::from)
                        .and_then(|s| {
                            let r = lookup.resolve_res(&s).map_err(Error::from);
                            input = Some(s);
                            r
                        });
                    commands.trigger_targets(Completed::Unhandled { result, input }, trigger.entity());
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
