use crate::{
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
    prelude::{Deref, Reflect, Trigger, TextBundle, TextStyle},
    utils::Duration,
};
use bevy_defer::AsyncWorld;
use bevy_input_sequence::KeyChord;
use std::{borrow::Cow, fmt::Debug};
use bevy_asky::prelude::*;
use futures::{channel::oneshot, Future};

// #[derive(Resource, Debug, Reflect, Deref)]
// pub struct MinibufferDest(Entity);

/// Minibuffer, a [SystemParam]
#[derive(Clone)]
pub struct Minibuffer {
    asky: Asky,
    dest: Entity,
    // dest: Res<'w, MinibufferDest>,
    // channel: CrossbeamEventSender<DispatchEvent>,
}

unsafe impl SystemParam for Minibuffer {
    type State = (
        // Asky,
        Entity,
        // CrossbeamEventSender<DispatchEvent>,
    );
    type Item<'w, 's> = Minibuffer;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            // Asky,
            Query<Entity, With<PromptContainer>>,
            // Option<Res<MinibufferStyle>>,
            // Res<CrossbeamEventSender<DispatchEvent>>,
        )> = SystemState::new(world);
        let (//asky,
             query,
             //res,
             //channel
        ) = state.get_mut(world);
        (
            // asky,
            query.single(),
            // res.map(|x| x.clone()),
            // channel.clone(),
        )
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: bevy::ecs::component::Tick,
    ) -> Self::Item<'w, 's> {
        let state = state.clone();
        Minibuffer {
            asky: Asky::default(),
            dest: state.0,
            // style: state.2.unwrap_or_default(),
            // channel: state.3,
        }
    }
}

#[derive(Component, Debug, Reflect)]
struct Message;
impl Construct for Message {
    type Props = Cow<'static, str>;

    fn construct(
        context: &mut ConstructContext,
        props: Self::Props,
    ) -> Result<Self, ConstructError> {
        // Our requirements.
        let mut commands = context.world.commands();
        commands
            .entity(context.id)
            .insert(TextBundle::from_section(props, TextStyle::default()));
        context.world.flush();
        Ok(Message)
    }
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

impl Minibuffer {

    /// Prompt the user for input.
    pub fn prompt<T: Construct + Component + Submitter>(
        &mut self,
        props: impl Into<T::Props>,
    ) -> impl Future<Output = Result<T::Out, Error>>
    where
        <T as Construct>::Props: Send,
        <T as Submitter>::Out: Clone + Debug + Send + Sync,
    {
        self.asky.prompt::<T, bevy_asky::view::color::View>(props, Dest::ReplaceChildren(self.dest))
    }

    /// Leave a message in the minibuffer.
    pub fn message(&mut self, msg: impl Into<String>) {
        let msg = msg.into();

        let dest = self.dest;
        let async_world = AsyncWorld::new();
        async_world.apply_command(move |world: &mut World| {
            let mut commands = world.commands();
            Dest::ReplaceChildren(dest).entity_commands(&mut commands)
                .construct::<Message>(msg);
        });
        // self.dest
        // self.asky.prompt::<Message, bevy_asky::view::color::View>(msg.as_ref(), Dest::ReplaceChildren(self.dest))
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
        let world = AsyncWorld::new();
        world.entity(self.dest).despawn_descendants()
    }

    // Wait a certain duration.
    // pub fn delay(&mut self, duration: Duration) -> impl Future<Output = ()> {
    //     self.asky.delay(duration)
    // }

    /// Get the next key chord.
    pub fn get_chord(&mut self) -> impl Future<Output = Result<KeyChord, Error>> {
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
