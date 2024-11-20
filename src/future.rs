use crate::{
    autocomplete::AutoComplete,
    event::DispatchEvent,
    lookup::Lookup,
    prompt::{GetKeyChord, KeyChordEvent},
    ui::PromptContainer,
    Dest
};
use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        prelude::Commands,
        query::With,
        system::{EntityCommands, Query, Res, Resource, SystemMeta, SystemParam, SystemState},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    },
    prelude::{Deref, Reflect, TextBundle, TextStyle, Trigger, DespawnRecursiveExt},
    utils::Duration,
};
use bevy_asky::prelude::*;
use bevy_crossbeam_event::CrossbeamEventSender;
use bevy_defer::AsyncWorld;
use bevy_input_sequence::KeyChord;
use futures::{channel::oneshot, Future};
use std::{borrow::Cow, fmt::Debug};

/// MinibufferAsync, a [SystemParam] for async.
///
/// This is distinct from the [crate::sync::Minibuffer] because it does not have
/// any lifetimes which allow it to be captured by a closure.
#[derive(Clone)]
pub struct MinibufferAsync {
    asky: Asky,
    dest: Entity,
    sender: CrossbeamEventSender<DispatchEvent>,
}

unsafe impl SystemParam for MinibufferAsync {
    type State = (
        Entity,
        CrossbeamEventSender<DispatchEvent>,
    );
    type Item<'w, 's> = MinibufferAsync;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Query<Entity, With<PromptContainer>>,
            Res<CrossbeamEventSender<DispatchEvent>>,
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
            asky: Asky::default(),
            dest: state.0,
            sender: state.1,
        }
    }
}

impl MinibufferAsync {
    /// Prompt the user for input.
    pub fn prompt<T: Construct + Component + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        self.asky
            .prompt::<T>(props, Dest::ReplaceChildren(self.dest))
    }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        self.sender.send(DispatchEvent::EmitMessage(msg.into()));
    }

    /// Read input from user that must match a [Lookup].
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
                let mut commands = Dest::ReplaceChildren(dest).entity(&mut commands);
                let autocomplete = AutoComplete::new(lookup);
                autocomplete.construct(commands, prompt).observe(
                    move |trigger: Trigger<AskyEvent<String>>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            promise.send(trigger.event().0.clone()).expect("send");
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

    /// Wait a certain duration.
    pub async fn delay(&mut self, duration: Duration) {
        let world = AsyncWorld::new();
        world.sleep(duration);
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
                    move |trigger: Trigger<KeyChordEvent>, mut commands: Commands| {
                        if let Some(promise) = promise.take() {
                            promise.send(Ok(trigger.event().0.clone())).expect("send");
                        }
                        commands.entity(trigger.entity()).despawn();
                    },
                );
            });
            waiter.await?
        }
    }
}
