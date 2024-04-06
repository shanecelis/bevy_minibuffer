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


use crate::{Error, ConsoleConfig, lookup::{LookUp, AutoComplete}};
use crate::event::{StartActEvent, DispatchEvent, LookUpEvent};
use crate::MinibufferStyle;
use bevy_crossbeam_event::CrossbeamEventSender;


use crate::ui::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum PromptState {
    #[default]
    // Uninit,
    Invisible,
    Finished,
    Visible,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum CompletionState {
    // Uninit,
    #[default]
    Invisible,
    Visible,
}


// impl<T> LookUp for T
// where
//     T: Parse,
// {
//     type Item = T;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         T::parse(input)
//     }
// }

pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

#[derive(Component, Reflect)]
pub struct HideTime {
    pub timer: Timer,
}

pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<ConsoleConfig>,
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
        // let asky_param_config = world
        //     .get_resource_mut::<AskyParamConfig>()
        //     .expect("No AskyParamConfig setup.")
        //     .clone();
        // (asky, query.single(), res.map(|x| x), channel.clone())
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
    pub fn prompt<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static>(
        &mut self,
        prompt: T,
    ) -> impl Future<Output = Result<T::Output, Error>> + '_ {
        self.prompt_styled(prompt, self.style.clone().into())
    }

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

    pub fn clear(&mut self) -> impl Future<Output = Result<(), asky::Error>> {
        self.asky.clear(self.dest)
    }

    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = Result<(), asky::Error>> {
        self.asky.delay(duration)
    }
}

// async fn read_crit<T>(
//     &mut self,
//     prompt: impl Into<PromptBuf>,
//     look_up: &impl LookUp<Item = T>,
// ) -> Result<T, NanoError> {
//     let mut buf = prompt.into();
//     loop {
//         match self.read_raw(buf.clone()).await {
//             Ok(mut new_buf) => match look_up.look_up(&new_buf.input) {
//                 Ok(v) => {
//                     if new_buf.flags.contains(Requests::Submit) {
//                         return Ok(v);
//                     } else {
//                         buf = new_buf
//                     }
//                 }
//                 Err(LookUpError::Message(m)) => {
//                     new_buf.completion.clear();
//                     new_buf.message = m.to_string();
//                     buf = new_buf;
//                 }
//                 Err(LookUpError::Incomplete(v)) => {
//                     if new_buf.flags.contains(Requests::AutoComplete) {
//                         new_buf.completion.clear();
//                         new_buf.completion.extend_from_slice(&v[..]);

//                         if !new_buf.completion.is_empty() {
//                             let prefix = longest_common_prefix(&new_buf.completion);
//                             if prefix.len() > new_buf.input.len() {
//                                 new_buf.input = prefix;
//                             }
//                             new_buf.message.clear();
//                         }
//                     }
//                     buf = new_buf;
//                 }
//                 Err(LookUpError::NanoError(e)) => return Err(e),
//             },
//             Err(e) => return Err(e),
//         }
//         buf.flags = Requests::empty();
//     }
// }

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

pub(crate) fn handle_dispatch_event(
    mut dispatch_events: EventReader<DispatchEvent>,
    mut look_up_events: EventWriter<LookUpEvent>,
    mut request_act_events: EventWriter<StartActEvent>,
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
pub(crate) fn handle_look_up_event(
    mut look_up_events: EventReader<LookUpEvent>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut commands: Commands,
    mut last_hash: Local<Option<u64>>,
    config: Res<ConsoleConfig>,
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

pub fn listen_prompt_active(
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
