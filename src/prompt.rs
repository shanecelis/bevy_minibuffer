#![allow(async_fn_in_trait)]
use bitflags::bitflags;
use std::borrow::Cow;
use std::fmt::Debug;

use bevy::ecs::{component::Tick, prelude::Commands, system::{SystemParam, SystemMeta, SystemState}, world::unsafe_world_cell::UnsafeWorldCell};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use promise_out::{pair::Producer, Promise};
use asky::{Typeable, Valuable, Error, bevy::{Asky, KeyEvent, AskyPrompt, AskyParamConfig}, style::Style};

use std::future::Future;
// use futures_lite::future;
use crate::MinibufferStyle;

use crate::ui::*;

pub type CowStr = Cow<'static, str>;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum PromptState {
    #[default]
    // Uninit,
    Invisible,
    Visible,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum CompletionState {
    // Uninit,
    #[default]
    Invisible,
    Visible,
}

#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(CowStr),
}

#[allow(dead_code)]
pub enum LookUpError {
    Message(Cow<'static, str>),
    NanoError(NanoError),
    Incomplete(Vec<String>),
}

/// Handles arrays of &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> LookUp for &[T] {
    type Item = String;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.

        // let matches: Vec<&str> = self
        //     .iter()
        //     .map(|word| word.as_ref())
        //     .filter(|word| word.starts_with(input))
        //     .collect();
        // match matches[..] {
        //     [a] => Ok(a.to_string()),
        //     [_a, _b, ..] => Err(LookUpError::Incomplete(
        //         matches.into_iter().map(|s| s.to_string()).collect(),
        //     )),
        //     [] => Err(LookUpError::Message(" no matches".into())),
        // }

        let mut matches = self
            .iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input));

        if let Some(first) = matches.next() {
            if let Some(second) = matches.next() {
                let mut result = vec![first.to_string(), second.to_string()];
                for item in matches {
                    result.push(item.to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else {
                if input == first {
                    Ok(first.to_string())
                } else {
                    Err(LookUpError::Incomplete(vec![first.to_string()]))
                }
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }
}

pub trait LookUp: Sized {
    type Item;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<T> LookUp for T
where
    T: Parse,
{
    type Item = T;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        T::parse(input)
    }
}

pub trait Parse: Debug + Sized {
    fn parse(input: &str) -> Result<Self, LookUpError>;
}

impl Parse for () {
    fn parse(_: &str) -> Result<Self, LookUpError> {
        Ok(())
    }
}

impl Parse for String {
    fn parse(input: &str) -> Result<Self, LookUpError> {
        Ok(input.to_owned())
    }
}

impl Parse for i32 {
    fn parse(input: &str) -> Result<Self, LookUpError> {
        match input.parse::<i32>() {
            Ok(int) => Ok(int),
            Err(e) => Err(LookUpError::Message(format!(" expected int: {}", e).into())),
        }
    }
}

pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

#[derive(Component)]
pub struct HideTime {
    pub timer: Timer,
}

#[derive(Debug, Resource, Clone)]
pub struct ConsoleConfig {
    // pub(crate) state: Arc<Mutex<ConsoleState>>,
    pub hide_delay: Option<u64>,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            // state: Arc::new(Mutex::new(ConsoleState::new())),
            hide_delay: Some(2000), /* milliseconds */
        }
    }
}

pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<ConsoleConfig>,
    mut query: Query<(Entity, &mut Visibility, Option<&mut HideTime>), With<T>>,
) {
    for (id, mut visibility, hide_time_maybe) in query.iter_mut() {
        match config.hide_delay {
            Some(hide_delay) => {
                match hide_time_maybe {
                    Some(mut hide_time) => {
                        hide_time
                            .timer = Timer::new(Duration::from_millis(hide_delay),
                                                TimerMode::Once);
                    }
                    None => {
                        commands.entity(id).insert(HideTime {
                            timer: Timer::new(Duration::from_millis(hide_delay),
                                            TimerMode::Once),
                        });
                    }
                }
            }
            None => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

pub fn hide_prompt_maybe(
    mut commands: Commands,
    // mut tasks: Query<(Entity, &mut TaskSink<T>)>,
    time: Res<Time>,
    state: Res<State<AskyPrompt>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(Entity, &mut Visibility, &mut HideTime)>,
) {
    for (id, mut visibility, mut hide) in query.iter_mut() {
        // eprintln!("checking hide {:?}", time.delta());
        redraw.send(RequestRedraw); // Force ticks to happen when a timer is present.
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == AskyPrompt::Inactive {
                eprintln!("hiding after delay.");
                *visibility = Visibility::Hidden;
            }
            commands.entity(id).remove::<HideTime>();
        }
    }
}

#[allow(dead_code)]
pub fn hide<T: Component>(mut query: Query<&mut Visibility, With<T>>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
    }
}

#[cfg(test)]
mod tests {
    use crate::prompt::LookUpError;
    use crate::prompt::Parse;

    #[derive(Debug)]
    struct TomDickHarry(String);

    impl Parse for TomDickHarry {
        fn parse(input: &str) -> Result<Self, LookUpError> {
            match input {
                "Tom" => Ok(TomDickHarry(input.into())),
                "Dick" => Ok(TomDickHarry(input.into())),
                "Harry" => Ok(TomDickHarry(input.into())),
                _ => Err(LookUpError::Incomplete(vec![
                    "Tom".into(),
                    "Dick".into(),
                    "Harry".into(),
                ])),
            }
        }
    }

    // #[allow(unused_must_use)]
    // #[test]
    // fn test_option_default() {
    //     let a: Option<PromptCel> = default();
    // }
}

pub struct Minibuffer {
    asky: Asky,
    dest: Entity,
    style: MinibufferStyle,
}

unsafe impl SystemParam for Minibuffer {
    type State = (Asky, Entity, Option<MinibufferStyle>);
    type Item<'w, 's> = Minibuffer;

    fn init_state(mut world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Asky,
            Query<Entity, With<PromptContainer>>,
            Option<Res<MinibufferStyle>>,
        )> = SystemState::new(&mut world);
        let (asky, query, res) = state.get_mut(&mut world);
        // let asky_param_config = world
        //     .get_resource_mut::<AskyParamConfig>()
        //     .expect("No AskyParamConfig setup.")
        //     .clone();
        (asky, query.single(), res.map(|x| x.clone()))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let state = state.clone();
        Minibuffer {
            asky: state.0,
            dest: state.1,
            style: state.2.unwrap_or_default(),
        }
    }
}

impl Minibuffer {
    pub fn prompt<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static>(
        &mut self,
        prompt: T
    ) -> impl Future<Output = Result<T::Output, Error>> + '_ {
        self.prompt_styled(prompt, self.style)
        // async move {
        //     let _ = self.asky.clear(self.dest).await;
        //     self.asky.prompt_styled(prompt, self.dest, self.style.clone()).await
        // }
    }

    pub fn prompt_styled<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static, S>(
        &mut self,
        prompt: T,
        style: S
    ) -> impl Future<Output = Result<T::Output, Error>> + '_
    where S: Style + Send + Sync + 'static {
        async move {
            let _ = self.asky.clear(self.dest).await;
            self.asky.prompt_styled(prompt, self.dest, style).await
        }
    }

    pub fn clear(&mut self) -> impl Future<Output = Result<(), Error>> {
        self.asky.clear(self.dest)
    }

    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = Result<(), Error>> {
        self.asky.delay(duration)
    }
}
