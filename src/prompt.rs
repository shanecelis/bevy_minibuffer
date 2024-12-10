//! Kinds of queries for user: Toggle, TextField, etc.

use crate::{
    event::LookupEvent,
    ui::{completion_item, ScrollingList},
    Config,
};
use bevy::{prelude::*, window::RequestRedraw};
use bevy_asky::prelude::*;
use bevy_input_sequence::{KeyChord, Modifiers};
use std::collections::VecDeque;
use std::fmt::Debug;

pub use bevy_asky::{prompt::*, Submit};

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<MinibufferState>()
        .register_type::<PromptState>()
        .register_type::<CompletionState>()
        .register_type::<HideTime>()
        .register_type::<GetKeyChord>();
}

/// Is the minibuffer visible?
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub enum MinibufferState {
    /// Inactive
    #[default]
    Inactive,
    /// Active
    Active,
}

/// Is the prompt active?
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub(crate) enum PromptState {
    // Uninit,
    /// Invisible
    #[default]
    Invisible,
    /// Visible
    Visible,
}

/// Is the autocomplete panel visible?
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Reflect)]
pub(crate) enum CompletionState {
    // Uninit,
    /// Invisible
    #[default]
    Invisible,
    /// Visible
    Visible,
}

/// Hides an entity after the timer finishes.
#[derive(Debug, Component, Reflect)]
pub(crate) struct HideTime {
    /// Timer
    pub timer: Timer,
}

// /// Get a key chord.
#[derive(Component, Debug, Reflect)]
pub(crate) struct GetKeyChord;

#[derive(Event, Debug, Reflect)]
pub(crate) enum KeyChordEvent {
    Unhandled(KeyChord),
    Handled,
}

impl KeyChordEvent {
    pub(crate) fn new(chord: KeyChord) -> Self {
        Self::Unhandled(chord)
    }

    pub(crate) fn take(&mut self) -> Option<KeyChord> {
        match std::mem::replace(self, KeyChordEvent::Handled) {
            KeyChordEvent::Unhandled(chord) => Some(chord),
            KeyChordEvent::Handled => None,
        }
    }
}

// impl GetKeyChord {
//     pub(crate) fn new(promise: Sender<Result<KeyChord, Error>>) -> Self {
//         GetKeyChord(Some(promise))
//     }
// }

/// Make component visible.
pub(crate) fn show<T: Component>(
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
    let is_active = query.iter().any(|x| focus.is_focused(x)) || key_chords.iter().next().is_some();

    next_minibuffer_state.set(if is_active {
        MinibufferState::Active
    } else {
        MinibufferState::Inactive
    });
}

/// Returns true if [KeyCode] is a modifier key.
pub(crate) fn is_modifier(key: KeyCode) -> bool {
    let mods = Modifiers::from(key);
    !mods.is_empty()
}

pub(crate) fn get_key_chords(
    mut query: Query<Entity, With<GetKeyChord>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut buffer: Local<VecDeque<KeyChord>>,
    mut commands: Commands,
) {
    let mods = Modifiers::from(&keys);
    let mut chords: VecDeque<KeyChord> = keys
        .get_just_pressed()
        .filter(|key| !is_modifier(**key))
        .map(move |key| KeyChord(mods, *key))
        .collect();

    if let Some(chord) = buffer.pop_front().or_else(|| chords.pop_front()) {
        for id in query.iter_mut() {
            commands.trigger_targets(KeyChordEvent::new(chord.clone()), id);
            // NOTE: Don't remove this here. Let the consumer decide when they're done.
            //
            // commands.entity(id).remove::<GetKeyChord>();
            // commands.entity(id).despawn();
        }
    }
    buffer.extend(chords);
}

/// Hide entities with component [HideTime].
pub(crate) fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<Config>,
    // redraw: EventWriter<RequestRedraw>,
    mut query: Query<Entity, With<T>>,
) {
    if !config.auto_hide {
        return;
    }
    for id in query.iter_mut() {
        commands.entity(id).insert(HideTime {
            timer: Timer::new(config.hide_delay, TimerMode::Once),
        });
    }
}

/// Hide the prompt if the timer is finished.
pub(crate) fn hide_prompt_maybe(
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
pub(crate) fn hide<T: Component>(
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
    commands: &mut Commands,
) {
    let new_children = labels
        .into_iter()
        .map(|label| commands.spawn(completion_item(label)).id())
        .collect::<Vec<Entity>>();
    commands.entity(completion).replace_children(&new_children);
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(*child).despawn();
        }
    }
}

pub(crate) fn lookup_events(
    mut lookup_events: EventReader<LookupEvent>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut commands: Commands,
    mut last_hash: Local<Option<u64>>,
) {
    for e in lookup_events.read() {
        // info!("look up event: {e:?}");
        match e {
            LookupEvent::Completions(v) => {
                let rnd_state = bevy::utils::RandomState::with_seed(0);
                let hash = rnd_state.hash_one(v);
                // eprintln!("hash {hash}");
                if last_hash.unwrap_or(0) != hash {
                    let (completion_node, children) = completion.single();
                    completion_set(
                        completion_node,
                        children,
                        v.clone(),
                        &mut commands,
                    );
                    next_completion_state.set(CompletionState::Visible);
                    redraw.send(RequestRedraw);
                }
                *last_hash = Some(hash);
            }
            LookupEvent::Hide => {
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
        if let Some(MinibufferState::Active) = transition.entered {
            next_prompt_state.set(PromptState::Visible)
        }
        redraw.send(RequestRedraw);
    }
}

#[cfg(test)]
mod tests {
    use crate::autocomplete::Lookup;
    // use super::Parse;

    // #[derive(Debug)]
    // struct TomDickHarry(String);

    // impl Parse for TomDickHarry {
    //     fn parse(input: &str) -> Result<Self, LookupError> {
    //         match input {
    //             "Tom" => Ok(TomDickHarry(input.into())),
    //             "Dick" => Ok(TomDickHarry(input.into())),
    //             "Harry" => Ok(TomDickHarry(input.into())),
    //             _ => Err(LookupError::Incomplete(vec![
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
        use trie_rs::Trie;
        let trie: Trie<u8> = ["ask_name", "ask_name2", "asky_age"].into_iter().collect();
        assert_eq!(trie.longest_prefix::<String, _>("a").unwrap(), "ask");
        let lookup: &dyn Lookup = &trie;
        assert_eq!(lookup.longest_prefix("a").unwrap(), "ask");
        assert_eq!(lookup.longest_prefix("b"), None);
        // let lookup: &dyn Lookup = &trie;
    }
}
