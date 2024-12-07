//! An async version of the Minibuffer parameter
//!
//! It uses promises rather than triggers.
use crate::{
    acts::ActArg,
    autocomplete::{AutoComplete, Lookup},
    event::{DispatchEvent, RunActEvent, RunActByNameEvent},
    prompt::{GetKeyChord, KeyChordEvent, PromptState},
    ui::PromptContainer,
    view::View,
    Error,
};
use bevy::{
    ecs::{
        entity::Entity,
        prelude::Commands,
        query::With,
        system::{EntityCommands, Query, Res, SystemMeta, SystemParam, SystemState},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    },
    prelude::{Bundle, DespawnRecursiveExt, State, Trigger},
    utils::Duration,
};
use bevy_asky::{
    construct::{Add0, Construct},
    AskyAsync, Dest, Submit, Submitter,
};
// use bevy_crossbeam_event::CrossbeamEventSender;
use bevy_channel_trigger::ChannelSender;
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::KeyChord;
use futures::{channel::oneshot, future::Either, pin_mut, Future, TryFutureExt};
use std::{borrow::Cow, fmt::Debug};

/// MinibufferAsync, a [SystemParam] for async.
///
/// This is distinct from the [crate::sync::Minibuffer] because it does not have
/// any lifetimes which allow it to be captured by closures.
#[derive(Clone)]
pub struct MinibufferAsync {
    asky: AskyAsync,
    dest: Entity,
    trigger: ChannelSender<DispatchEvent>,
}

unsafe impl SystemParam for MinibufferAsync {
    type State = (
        Entity,
        // CrossbeamEventSender<DispatchEvent>,
        ChannelSender<DispatchEvent>,
    );
    type Item<'w, 's> = MinibufferAsync;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Query<Entity, With<PromptContainer>>,
            Res<ChannelSender<DispatchEvent>>,
        )> = SystemState::new(world);
        let (query, channel) = state.get_mut(world);
        (query.single(), channel.clone())
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: bevy::ecs::component::Tick,
    ) -> Self::Item<'w, 's> {
        let state = state.clone();
        MinibufferAsync {
            asky: AskyAsync,
            dest: state.0,
            trigger: state.1,
        }
    }
}

impl MinibufferAsync {
    /// Prompt the user for input.
    pub fn prompt<T: Construct + Bundle + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send + Sync,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        self.asky
            .prompt::<Add0<T, View>>(props, Dest::ReplaceChildren(self.dest))
            .map_err(Error::from)
    }

    /// Request an act be run.
    ///
    /// Returns true if act found and request sent. If given a name for no
    /// corresponding act, it will return false.
    pub fn run_act(&mut self, act: impl Into<ActArg>) {
        match act.into() {
            ActArg::Act(act) => {
                self.trigger.send(DispatchEvent::RunActEvent(RunActEvent::new(act)));
            }
            ActArg::Name(name) => {
                self.trigger.send(DispatchEvent::RunActByNameEvent(RunActByNameEvent::new(name)));
            }
        }
    }

    /// Builds a prompt and accepts a closure that may alter that entity.
    pub fn prompt_with<T: Submitter + Construct + Bundle>(
        &mut self,
        props: impl Into<T::Props>,
        f: impl FnOnce(&mut EntityCommands) + Send + 'static,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send + Sync,
        <T as Submitter>::Out: Clone + Debug + Send + Sync + 'static,
    {
        let p = props.into();
        self.asky
            .prompt_with::<Add0<T, View>>(p, Dest::ReplaceChildren(self.dest), f)
            .map_err(Error::from)
    }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        self.trigger.send(DispatchEvent::EmitMessage(msg.into()));
    }

    /// Read input from user that must match a [Lookup].
    pub fn prompt_with_lookup<L>(
        &mut self,
        prompt: impl Into<Cow<'static, str>>,
        lookup: L,
    ) -> impl Future<Output = Result<String, Error>> + '_
    where
        L: Lookup + Clone + Send + Sync + 'static,
    {
        let prompt = prompt.into();
        async {
            let dest = self.dest;
            let (promise, waiter) = oneshot::channel::<Result<String, Error>>();
            let mut promise = Some(promise);
            let async_world = AsyncWorld::new();
            async_world.apply_command(move |world: &mut World| {
                let mut commands = world.commands();
                let commands = Dest::ReplaceChildren(dest).entity(&mut commands);
                let autocomplete = AutoComplete::new(lookup);
                autocomplete.construct(commands, prompt).observe(
                    move |mut trigger: Trigger<Submit<String>>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            promise
                                .send(trigger.event_mut().take_result().map_err(Error::from))
                                .expect("send");
                        }
                        commands.entity(trigger.entity()).despawn_recursive();
                    },
                );
            });
            waiter.await?
        }
    }

    /// Clear the minibuffer.
    pub fn clear(&mut self) {
        self.trigger.send(DispatchEvent::Clear);
    }

    /// Hide the minibuffer.
    pub fn set_visible(&mut self, show: bool) {
        self.trigger.send(DispatchEvent::SetVisible(show));
    }

    /// Show the minibuffer.
    pub fn is_visible(&mut self) -> impl Future<Output = Result<bool, Error>> {
        async move {
            let async_world = AsyncWorld::new();

            async_world
                .resource::<State<PromptState>>()
                .get(|res| matches!(**res, PromptState::Visible))
                .map_err(Error::from)
        }
    }

    /// Wait a certain duration.
    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = ()> {
        AsyncWorld::new().sleep(duration)
    }

    /// Wait for a certain duration or a key chord, whichever comes first.
    pub async fn delay_or_chord(&mut self, duration: Duration) -> Option<KeyChord> {
        const SMALL_DURATION: Duration = Duration::from_millis(250);
        let sleep = AsyncWorld::new().sleep(duration);
        let get_key = async move {
            // We sleep a tiny bit at the beginning so that we don't accept a
            // key press that happened right as the sleep timer died.
            AsyncWorld::new().sleep(SMALL_DURATION.min(duration / 4)).await;
            self.get_chord().await
        };
        pin_mut!(sleep);
        pin_mut!(get_key);
        match futures::future::select(sleep, get_key).await {
            Either::Left((_, _)) => None,
            Either::Right((chord, _)) => chord.ok(),
        }
    }

    /// Get the next key chord.
    pub fn get_chord(&mut self) -> impl Future<Output = Result<KeyChord, Error>> {
        async {
            let (promise, waiter) = oneshot::channel::<Result<KeyChord, Error>>();
            let mut promise = Some(promise);
            let async_world = AsyncWorld::new();
            async_world.apply_command(move |world: &mut World| {
                let mut commands = world.commands();
                commands.spawn(GetKeyChord).observe(
                    move |mut trigger: Trigger<KeyChordEvent>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            let _ = promise.send(trigger.event_mut().take().ok_or(Error::Message("no key chord".into())));
                        }
                        commands.entity(trigger.entity()).despawn();
                    },
                );
            });
            waiter.await?
        }
    }
}
