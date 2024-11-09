//! Prompt

use crate::{
    event::{DispatchEvent, LookUpEvent, RunActEvent},
    ui::{completion_item, ScrollingList},
    Config,
};
use bevy::{prelude::*, utils::Duration, window::RequestRedraw};
use bevy_input_sequence::{KeyChord, Modifiers};
use std::collections::VecDeque;
use std::fmt::Debug;
use bevy_asky::prelude::*;
use bevy_defer::sync::oneshot::Sender;

/// The state of... something???
/// XXX: What is this?
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum MinibufferState {
    /// Inactive
    #[default]
    Inactive,
    /// Active
    Active,
}

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

// /// Get a key chord.
#[derive(Component, Debug)]
pub(crate) struct GetKeyChord;

#[derive(Event, Debug)]
pub(crate) struct KeyChordEvent(pub(crate) KeyChord);

// impl GetKeyChord {
//     pub(crate) fn new(promise: Sender<Result<KeyChord, Error>>) -> Self {
//         GetKeyChord(Some(promise))
//     }
// }

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

/// Check components to determine MinibufferState's state.
pub(crate) fn set_minibuffer_state(
    query: Query<Entity, With<Focusable>>,
    focus: Focus,
    key_chords: Query<&GetKeyChord>,
    mut next_minibuffer_state: ResMut<NextState<MinibufferState>>,
) {
    let is_active = query.iter().any(|x| focus.is_focused(x));
        //|| key_chords.iter().next().is_some();

    next_minibuffer_state.set(if is_active {
        MinibufferState::Active
    } else {
        MinibufferState::Inactive
    });
}

/// Returns true if [KeyCode] is a modifier key.
pub fn is_modifier(key: KeyCode) -> bool {
    let mods = Modifiers::from(key);
    !mods.is_empty()
}

pub(crate) fn get_key_chords(
    mut query: Query<Entity, With<GetKeyChord>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut buffer: Local<VecDeque<KeyChord>>,
    mut commands: Commands,
) {
    let mods = Modifiers::from_input(&keys);
    let mut chords: VecDeque<KeyChord> = keys
        .get_just_pressed()
        .filter(|key| !is_modifier(**key))
        .map(move |key| KeyChord(mods, *key))
        .collect();

    if let Some(chord) = buffer.pop_front().or_else(|| chords.pop_front()) {
        for id in query.iter_mut() {
            commands.trigger_targets(KeyChordEvent(chord.clone()), id);
            // if let Some(promise) = get_key_chord.0.take() {
            //     promise.resolve(chord.clone());
            // }
            commands.entity(id).remove::<GetKeyChord>();
            // commands.entity(id).despawn();
        }
    }
    buffer.extend(chords);
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
    state: Res<State<MinibufferState>>,
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
            if *state == MinibufferState::Inactive {
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
            RunActEvent(s) => {
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

/// Listen for [MinibufferState] transitions.
pub(crate) fn listen_prompt_active(
    mut transitions: EventReader<StateTransitionEvent<MinibufferState>>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut redraw: EventWriter<RequestRedraw>,
) {
    for transition in transitions.read() {
        match transition.entered {
            Some(MinibufferState::Active) => next_prompt_state.set(PromptState::Visible),
            Some(MinibufferState::Inactive) => next_prompt_state.set(PromptState::Finished),
            _ => (),
        }
        redraw.send(RequestRedraw);
    }
}

#[cfg(test)]
mod tests {
    // use crate::lookup::LookUp;
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

    // #[test]
    // fn test_lookup() {
    //     use trie_rs::Trie;
    //     let trie: Trie<u8> = ["ask_name", "ask_name2", "asky_age"].into_iter().collect();
    //     assert_eq!(trie.longest_prefix::<String, _>("a").unwrap(), "ask");
    //     let lookup: &dyn LookUp = &trie;
    //     assert_eq!(lookup.longest_prefix("a").unwrap(), "ask");
    //     assert_eq!(lookup.longest_prefix("b"), None);
    //     // let lookup: &dyn LookUp = &trie;
    // }
}
