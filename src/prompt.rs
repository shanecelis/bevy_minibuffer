//! Prompt
#![allow(async_fn_in_trait)]

use std::fmt::Debug;

use bevy::ecs::{
    component,
    system::{SystemMeta, SystemParam, SystemState},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use asky::bevy::{Asky, AskyPrompt, AskyStyle, KeyEvent};
use asky::{Typeable, Valuable};

use std::future::Future;

use crate::event::{DispatchEvent, LookUpEvent, RunActEvent};
use crate::MinibufferStyle;
use crate::{
    lookup::{AutoComplete, LookUp},
    Config, Error,
};
use bevy_crossbeam_event::CrossbeamEventSender;

use crate::ui::*;

/// The state of the minibuffer
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum PromptState {
    // Uninit,
    /// Invisible
    #[default]
    Invisible,
    /// Finished prompt, start auto hide timer.
    Finished,
    /// Visible
    Visible,
}

/// The state of the autocomplete panel
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum CompletionState {
    // Uninit,
    /// Invisible
    #[default]
    Invisible,
    /// Visible
    Visible,
}

/// Hides an entity after the timer finishes.
#[derive(Component, Reflect)]
pub struct HideTime {
    /// Timer
    pub timer: Timer,
}


/// Make component visible.
pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

/// Hide entities with component [HideTime].
pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<Config>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(Entity, &mut Visibility, Option<&mut HideTime>), With<T>>,
) {
    if !config.auto_hide {
        return;
    }
    for (id, mut visibility, hide_time_maybe) in query.iter_mut() {
        match config.hide_delay {
            Some(hide_delay) => match hide_time_maybe {
                Some(mut hide_time) => {
                    hide_time.timer =
                        Timer::new(Duration::from_millis(hide_delay), TimerMode::Once);
                }
                None => {
                    commands.entity(id).insert(HideTime {
                        timer: Timer::new(Duration::from_millis(hide_delay), TimerMode::Once),
                    });
                }
            },
            None => {
                *visibility = Visibility::Hidden;
                redraw.send(RequestRedraw);
            }
        }
    }
}

/// Hide the prompt if the timer is finished.
pub fn hide_prompt_maybe(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<AskyPrompt>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut query: Query<(Entity, &mut HideTime)>,
) {
    for (id, mut hide) in query.iter_mut() {
        // eprintln!("checking hide {:?}", time.delta());
        redraw.send(RequestRedraw); // Force ticks to happen when a timer is present.
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == AskyPrompt::Inactive {
                next_prompt_state.set(PromptState::Invisible);
                next_completion_state.set(CompletionState::Invisible);
                // eprintln!("hiding after delay.");
                // *visibility = Visibility::Hidden;
            }
            commands.entity(id).remove::<HideTime>();
        }
    }
}

/// Hide the entity whose component matches.
#[allow(dead_code)]
pub fn hide<T: Component>(
    mut query: Query<&mut Visibility, With<T>>,
    mut redraw: EventWriter<RequestRedraw>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
        redraw.send(RequestRedraw);
    }
}

/// Minibuffer, a [bevy::ecs::system::SystemParam]
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
    pub fn clear(&mut self) -> impl Future<Output = Result<(), asky::Error>> {
        self.asky.clear(self.dest)
    }

    /// Wait a certain duration.
    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = Result<(), asky::Error>> {
        self.asky.delay(duration)
    }
}

fn completion_set(
    completion: Entity,
    children: Option<&Children>,
    labels: Vec<String>,
    style: TextStyle,
    commands: &mut Commands,
) {
    let new_children = labels
        .into_iter()
        .map(|label| commands.spawn(completion_item(label, style.clone())).id())
        .collect::<Vec<Entity>>();
    commands.entity(completion).replace_children(&new_children);
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(*child).despawn();
        }
    }
}

pub(crate) fn dispatch_events(
    mut dispatch_events: EventReader<DispatchEvent>,
    mut look_up_events: EventWriter<LookUpEvent>,
    mut request_act_events: EventWriter<RunActEvent>,
) {
    use crate::event::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookUpEvent(l) => {
                look_up_events.send(l.clone());
            }
            StartActEvent(s) => {
                request_act_events.send(s.clone());
            }
        }
    }
}
pub(crate) fn look_up_events(
    mut look_up_events: EventReader<LookUpEvent>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut commands: Commands,
    mut last_hash: Local<Option<u64>>,
    config: Res<Config>,
) {
    let text_style = &config.text_style;
    for e in look_up_events.read() {
        // info!("look up event: {e:?}");
        match e {
            LookUpEvent::Completions(v) => {
                let rnd_state = bevy::utils::RandomState::with_seed(0);
                let hash = rnd_state.hash_one(v);
                // eprintln!("hash {hash}");
                if last_hash.unwrap_or(0) != hash {
                    let (completion_node, children) = completion.single();
                    completion_set(
                        completion_node,
                        children,
                        v.clone(),
                        text_style.clone(),
                        &mut commands,
                    );
                    next_completion_state.set(CompletionState::Visible);
                    redraw.send(RequestRedraw);
                }
                *last_hash = Some(hash);
            }
            LookUpEvent::Hide => {
                // eprintln!("hide");
                *last_hash = None;
                next_completion_state.set(CompletionState::Invisible);
                redraw.send(RequestRedraw);
            }
        }
    }
}

/// Listen for [AskyPrompt] transitions.
pub(crate) fn listen_prompt_active(
    mut transitions: EventReader<StateTransitionEvent<AskyPrompt>>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut redraw: EventWriter<RequestRedraw>,
) {
    for transition in transitions.read() {
        match transition.after {
            AskyPrompt::Active => next_prompt_state.set(PromptState::Visible),
            AskyPrompt::Inactive => next_prompt_state.set(PromptState::Finished),
        }
        redraw.send(RequestRedraw);
    }
}

#[cfg(test)]
mod tests {
    // use crate::prompt::LookUpError;
    // use crate::prompt::Parse;

    // #[derive(Debug)]
    // struct TomDickHarry(String);

    // impl Parse for TomDickHarry {
    //     fn parse(input: &str) -> Result<Self, LookUpError> {
    //         match input {
    //             "Tom" => Ok(TomDickHarry(input.into())),
    //             "Dick" => Ok(TomDickHarry(input.into())),
    //             "Harry" => Ok(TomDickHarry(input.into())),
    //             _ => Err(LookUpError::Incomplete(vec![
    //                 "Tom".into(),
    //                 "Dick".into(),
    //                 "Harry".into(),
    //             ])),
    //         }
    //     }
    // }

    // #[test]
    // fn test_tom_dick_parse() {
    //     let a = TomDickHarry::parse("Tom").unwrap();
    //     assert_eq!(a.0, "Tom");
    // }

    #[test]
    fn test_lookup() {
        use super::LookUp;
        use trie_rs::Trie;
        let trie: Trie<u8> = ["ask_name", "ask_name2", "asky_age"].into_iter().collect();
        assert_eq!(trie.longest_prefix::<String, _>("a").unwrap(), "ask");
        let lookup: &dyn LookUp = &trie;
        assert_eq!(lookup.longest_prefix("a").unwrap(), "ask");
        assert_eq!(lookup.longest_prefix("b"), None);
        // let lookup: &dyn LookUp = &trie;
    }
}
