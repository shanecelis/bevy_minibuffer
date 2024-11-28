use crate::{
    autocomplete::AutoComplete,
    event::DispatchEvent,
    lookup::Lookup,
    prompt::{GetKeyChord, KeyChordEvent, PromptState},
    ui::PromptContainer,
    view::View,
    Dest,
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
    prelude::{Trigger, State, DespawnRecursiveExt, Bundle},
    utils::Duration,
};
use bevy_asky::{
    AskyAsync, AskyEvent, sync::{AskyEntityCommands, AskyCommands},
    construct::{Add, Construct}, Submitter,
};
// use bevy_crossbeam_event::CrossbeamEventSender;
use bevy_channel_trigger::ChannelSender;
use bevy_defer::{AsyncWorld, AsyncAccess};
use bevy_input_sequence::KeyChord;
use futures::{channel::oneshot, Future, future::Either, pin_mut, TryFutureExt};
use std::{borrow::Cow, fmt::Debug};

/// MinibufferAsync, a [SystemParam] for async.
///
/// This is distinct from the [crate::sync::Minibuffer] because it does not have
/// any lifetimes which allow it to be captured by a closure.
#[derive(Clone)]
pub struct MinibufferAsync {
    asky: AskyAsync,
    dest: Entity,
    sender: ChannelSender<DispatchEvent>,
}

unsafe impl SystemParam for MinibufferAsync {
    type State = (
        Entity,
        // CrossbeamEventSender<DispatchEvent>,
        ChannelSender<DispatchEvent>
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
            sender: state.1,
        }
    }
}

impl MinibufferAsync {
    /// Prompt the user for input.
    #[must_use]
    pub fn prompt<T: Construct + Bundle + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send + Sync,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        self.asky
            .prompt::<Add<T, View>>(props, Dest::ReplaceChildren(self.dest))
            .map_err(Error::from)
    }

    pub fn prompt_with<T: Submitter + Construct + Bundle>(
        &mut self,
        props: impl Into<T::Props>,
        f: impl FnOnce(&mut EntityCommands) + Send + 'static,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send + Sync,
        <T as Submitter>::Out: Clone + Debug + Send + Sync + 'static {
        let p = props.into();
        self.asky.prompt_with::<Add<T, View>>(p, Dest::ReplaceChildren(self.dest), f)
            .map_err(Error::from)

    }

    // #[must_use]
    // pub fn prompt_group<T: Construct + Component + Part>(
    //     &mut self,
    //     group_prop: impl Into<<<T as Part>::Group as Construct>::Props>,
    //     props: impl IntoIterator<Item = impl Into<T::Props>>,
    // ) -> impl Future<Output = Result<<<T as Part>::Group as Submitter>::Out, Error>>
    // where
    //     <T as Construct>::Props: Send,
    //     <<T as Part>::Group as Construct>::Props: Send,
    //     <T as Part>::Group: Component + Construct + Send + Sync + Submitter,
    //     <<T as Part>::Group as Submitter>::Out: Clone + Debug + Send + Sync {
    //     self.asky
    //         .prompt_group::<T, View>(group_prop, props, Dest::ReplaceChildren(self.dest))
    //         .map_err(Error::from)
    // }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        self.sender.send(DispatchEvent::EmitMessage(msg.into()));
    }

    /// Read input from user that must match a [Lookup].
    #[must_use]
    pub fn read<L>(
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
                    move |trigger: Trigger<AskyEvent<String>>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            promise.send(trigger.event().0.clone().map_err(Error::from)).expect("send");
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
        self.sender.send(DispatchEvent::Clear);
    }

    /// Hide the minibuffer.
    pub fn set_visible(&mut self, show: bool) {
        self.sender.send(DispatchEvent::SetVisible(show));
    }

    /// Show the minibuffer.
    pub fn is_visible(&mut self) -> impl Future<Output = Result<bool, Error>> {
        async move {
            let async_world = AsyncWorld::new();

            async_world
            .resource::<State<PromptState>>()
            .get(|res| matches!(**res, PromptStateVisible))
                .map_err(Error::from)

        }
    }

    /// Wait a certain duration.
    #[must_use]
    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = ()> {
        AsyncWorld::new().sleep(duration)
    }

    #[must_use]
    pub async fn delay_or_chord(&mut self, duration: Duration) -> Option<KeyChord> {
        let sleep = AsyncWorld::new().sleep(duration);
        let get_key = self.get_chord();
        pin_mut!(sleep);
        pin_mut!(get_key);
        match futures::future::select(sleep, get_key).await {
            Either::Left((_, _)) => None,
            Either::Right((chord, _)) => chord.ok()
        }
    }

    /// Get the next key chord.
    #[must_use]
    pub fn get_chord(&mut self) -> impl Future<Output = Result<KeyChord, Error>> {
        async {
            let (promise, waiter) = oneshot::channel::<Result<KeyChord, Error>>();
            let mut promise = Some(promise);
            let async_world = AsyncWorld::new();
            async_world.apply_command(move |world: &mut World| {
                let mut commands = world.commands();
                commands.spawn(GetKeyChord).observe(
                    move |trigger: Trigger<KeyChordEvent>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            let _ = promise.send(Ok(trigger.event().0.clone()));//.expect("send");
                        }
                        commands.entity(trigger.entity()).despawn();
                    },
                );
            });
            waiter.await?
        }
    }
}
