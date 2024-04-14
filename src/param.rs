use crate::{
    event::DispatchEvent,
    lookup::{AutoComplete, LookUp},
    prompt::GetKeyChord,
    ui::PromptContainer,
    MinibufferStyle,
};
use asky::{
    bevy::{Asky, AskyStyle, KeyEvent},
    Error, Typeable, Valuable,
};
use bevy::{
    ecs::{
        component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, SystemMeta, SystemParam, SystemState},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    },
    hierarchy::Children,
    utils::Duration,
};
use bevy_crossbeam_event::CrossbeamEventSender;
use bevy_defer::world;
use bevy_input_sequence::KeyChord;
use promise_out::{pair::Producer, Promise};
use std::future::Future;

/// Minibuffer, a [SystemParam]
#[derive(Clone)]
pub struct Minibuffer {
    asky: Asky,
    dest: Entity,
    style: MinibufferStyle,
    channel: CrossbeamEventSender<DispatchEvent>,
}

unsafe impl SystemParam for Minibuffer {
    type State = (
        Asky,
        Entity,
        Option<MinibufferStyle>,
        CrossbeamEventSender<DispatchEvent>,
    );
    type Item<'w, 's> = Minibuffer;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Asky,
            Query<Entity, With<PromptContainer>>,
            Option<Res<MinibufferStyle>>,
            Res<CrossbeamEventSender<DispatchEvent>>,
        )> = SystemState::new(world);
        let (asky, query, res, channel) = state.get_mut(world);
        (
            asky,
            query.single(),
            res.map(|x| x.clone()),
            channel.clone(),
        )
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: component::Tick,
    ) -> Self::Item<'w, 's> {
        let state = state.clone();
        Minibuffer {
            asky: state.0,
            dest: state.1,
            style: state.2.unwrap_or_default(),
            channel: state.3,
        }
    }
}

impl Minibuffer {
    /// Prompt the user for input.
    pub fn prompt<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static>(
        &mut self,
        prompt: T,
    ) -> impl Future<Output = Result<T::Output, Error>> + '_ {
        self.prompt_styled(prompt, self.style.clone().into())
    }

    /// Read input from user that must match a [LookUp].
    pub fn read<L>(
        &mut self,
        prompt: String,
        lookup: L,
    ) -> impl Future<Output = Result<String, Error>> + '_
    where
        L: LookUp + Clone + Send + Sync + 'static,
    {
        use crate::lookup::LookUpError::*;
        let mut text = asky::Text::new(prompt);
        let l = lookup.clone();
        text.validate(move |input| match l.look_up(input) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                Message(s) => Err(s),
                // Incomplete(_v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
                Incomplete(_v) => Err("Incomplete".into()),
                Minibuffer(e) => Err(format!("Error: {:?}", e).into()),
            },
        });
        let text = AutoComplete::new(text, lookup, self.channel.clone());
        self.prompt_styled(text, self.style.clone().into())
    }

    /// Prompt the user for input using a particular style.
    pub async fn prompt_styled<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static>(
        &mut self,
        prompt: T,
        style: AskyStyle,
    ) -> Result<T::Output, Error> {
        let _ = self.asky.clear(self.dest).await;
        self.asky
            .prompt_styled(prompt, self.dest, style)
            .await
            .map_err(Error::from)
    }

    /// Clear the minibuffer.
    pub fn clear(&mut self) -> impl Future<Output = ()> {
        self.asky.clear(self.dest)
    }

    /// Wait a certain duration.
    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = ()> {
        self.asky.delay(duration)
    }

    pub fn get_chord(&mut self) -> impl Future<Output = Result<Vec<KeyChord>, asky::Error>> {
        async {
            let (promise, waiter) = Producer::<Vec<KeyChord>, asky::Error>::new();
            let world = world();
            world.spawn_bundle(GetKeyChord::new(promise)).await;
            waiter.await
        }
    }
}
