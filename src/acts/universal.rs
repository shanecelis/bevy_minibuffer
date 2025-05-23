//! An act adverb that accepts a numerical prefix
//!
//! Can be queried by other commands using the [UniversalArg] resource.
use crate::{
    acts::{Act, ActFlags, Acts, ActsPlugin},
    event::{KeyChordEvent, LastRunAct, RunActEvent},
    prelude::keyseq,
    Minibuffer,
};
#[cfg(feature = "async")]
use crate::{sink, MinibufferAsync};
use bevy::prelude::*;
#[cfg(feature = "async")]
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::{KeyChord, KeyChordQueue};
#[cfg(feature = "async")]
use std::future::Future;
use std::{borrow::Cow, fmt::Debug};

pub(crate) fn plugin(app: &mut App) {
    // You can always rely on UniversalArg to be there.
    app.init_resource::<UniversalArg>();
}

/// Universal argument plugin and acts
///
/// Adds "universal_arg" act and resources: [UniversalArg] and [Multiplier].
pub struct UniversalArgActs {
    /// Acts
    pub acts: Acts,
}

/// Universal argument multiplier
///
/// When universal argument's key binding is invoked multiple times, its
/// multiplied by the number stored in this resource. By default that number is
/// four initially.
#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct Multiplier(i32);

impl Default for Multiplier {
    fn default() -> Self {
        Self(4)
    }
}

impl Default for UniversalArgActs {
    fn default() -> Self {
        Self {
            acts: Acts::new(vec![
                // Act::new(universal_arg.pipe(sink::future))
                Act::new(universal_arg)
                    .named("universal_arg")
                    .bind(keyseq! { Ctrl-U })
                    .sub_flags(ActFlags::RunAct | ActFlags::Record),
            ]),
        }
    }
}

impl UniversalArgActs {
    /// Include an act that prints the universal arg resource.
    pub fn include_display_act(mut self) -> Self {
        self.acts.push(
            Act::new(display_universal_arg)
                .named("display_universal_arg")
                .add_flags(ActFlags::ShowMinibuffer),
        );
        self
    }

    #[cfg(feature = "async")]
    /// Use the async version of universal_arg.
    ///
    /// NOTE: They should be identical behaviorally. Having them both present is
    /// really just to show how one can achieve the same results with the "sync"
    /// or "async" framework.
    pub fn use_async(mut self) -> Self {
        self.acts.push(
            Act::new(universal_arg_async.pipe(sink::future))
                .named("universal_arg")
                .bind(keyseq! { Ctrl-U })
                .sub_flags(ActFlags::RunAct),
        );
        self
    }
}

impl Plugin for UniversalArgActs {
    fn build(&self, app: &mut bevy::app::App) {
        app.register_type::<Multiplier>()
            .register_type::<UniversalArg>()
            .init_resource::<Multiplier>()
            .add_systems(bevy::app::Last, clear_arg);
        self.warn_on_unused_acts();
    }
}

impl ActsPlugin for UniversalArgActs {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}

fn clear_arg(
    mut event: EventReader<RunActEvent>,
    mut arg: ResMut<UniversalArg>,
    mut clear: Local<Option<Cow<'static, str>>>,
) {
    // Wait a frame to clear it.
    if let Some(_act) = clear.take() {
        arg.0 = None;
    }
    if let Some(e) = event.read().next() {
        if !e.act.flags.contains(ActFlags::Adverb) {
            // *clear = Some(e.name.clone());
            *clear = Some("something".into()); //e.name.clone());
        }
    }
}

/// Display the contents of the universal argument resource.
pub fn display_universal_arg(arg: Res<UniversalArg>, mut minibuffer: Minibuffer) {
    minibuffer.message(format!("{:?} {}", arg.0, arg.0.is_some()));
    // match arg.0 {
    //     Some(x) => minibuffer.message(format!("Univeral argument {x}")),
    //     None => minibuffer.message("No universal argument set"),
    // }
}

/// This resources stores the last given universal argument. It is cleared after
/// any act---that is not specifically marked [ActFlags::Adverb]---runs.
///
/// NOTE: the [UniversalArg] resource is always present even if the
/// [UniversalArgActs] plugin has not been added. This is to enable users to
/// opt-in to universal argument acts while allowing act writers to support universal arguments if available.
#[derive(Debug, Clone, Resource, Default, Reflect, Deref, DerefMut)]
pub struct UniversalArg(pub Option<i32>);

fn universal_arg(
    mut minibuffer: Minibuffer,
    multiplier: Res<Multiplier>,
    last_act: Res<LastRunAct>,
    mut acts: Query<&Act>,
) {
    use bevy::prelude::KeyCode::*;

    let mut bindkey: Option<KeyChord> = None;
    let multiplier: i32 = multiplier.0;
    let mut accum = 0;
    let mut accumulated = false;
    let prompt: Cow<'static, str> = last_act
        .hotkey(&mut acts.as_query_lens())
        .map(|hotkey| {
            if hotkey.chords.len() == 1 {
                bindkey = Some(hotkey.chords[0].clone());
            }
            format!("{}", hotkey).into()
        })
        .unwrap_or("universal_arg".into());
    minibuffer.message(prompt.clone());
    minibuffer.get_chord().observe(
        move |mut trigger: Trigger<KeyChordEvent>,
              mut universal_arg: ResMut<UniversalArg>,
              mut chord_queue: ResMut<KeyChordQueue>,
              mut minibuffer: Minibuffer,
              mut commands: Commands| {
            let abort: bool = 'body: {
                let Ok(chord @ KeyChord(mods, key)) = trigger.event_mut().take() else {
                    break 'body true;
                };
                if let Some(ref bindkey) = bindkey {
                    if chord == *bindkey {
                        if accum == 0 {
                            accum = multiplier * multiplier;
                        } else {
                            accum *= multiplier;
                        }
                        accumulated = true;
                        minibuffer.message(format!("{prompt} {accum}"));
                        break 'body false;
                    }
                }
                if !mods.is_empty() {
                    universal_arg.0 = (!accumulated).then_some(multiplier).or(Some(accum));
                    chord_queue.push_back(chord);
                    break 'body true;
                }

                let digit = match key {
                    Digit0 => Some(0),
                    Digit1 => Some(1),
                    Digit2 => Some(2),
                    Digit3 => Some(3),
                    Digit4 => Some(4),
                    Digit5 => Some(5),
                    Digit6 => Some(6),
                    Digit7 => Some(7),
                    Digit8 => Some(8),
                    Digit9 => Some(9),
                    Minus => Some(-1),
                    Backspace => {
                        accum /= 10;
                        None
                    }
                    _ => {
                        universal_arg.0 = (!accumulated).then_some(multiplier).or(Some(accum));
                        chord_queue.push_back(chord);
                        break 'body true;
                    }
                };
                if let Some(digit) = digit {
                    if digit >= 0 {
                        if accum >= 0 {
                            accum = accum * 10 + digit;
                        } else {
                            accum = accum * 10 - digit;
                        }
                    } else {
                        accum *= digit;
                    }
                }
                accumulated = true;
                minibuffer.message(format!("{prompt} {accum}"));
                false
            };
            if abort {
                minibuffer.clear();
                commands.entity(trigger.target()).despawn();
            }
        },
    );
}

#[cfg(feature = "async")]
fn universal_arg_async(
    mut minibuffer: MinibufferAsync,
    multiplier: Res<Multiplier>,
    last_act: Res<LastRunAct>,
    mut acts: Query<&Act>,
) -> impl Future<Output = ()> {
    use bevy::prelude::KeyCode::*;

    let mut bindkey: Option<KeyChord> = None;
    let multiplier: i32 = multiplier.0;

    let prompt: Cow<'static, str> = last_act
        .hotkey(&mut acts.as_query_lens())
        .map(|hotkey| {
            if hotkey.chords.len() == 1 {
                bindkey = Some(hotkey.chords[0].clone());
            }
            format!("{} ", hotkey).into()
        })
        .unwrap_or("univeral_arg: ".into());
    minibuffer.message(prompt.clone());
    async move {
        let mut accum = 0;
        let mut accumulated = false;
        loop {
            let Ok(chord @ KeyChord(_mods, key)) = minibuffer.get_chord().await else {
                break;
            };
            if let Some(ref bindkey) = bindkey {
                if chord == *bindkey {
                    if accum == 0 {
                        accum = multiplier * multiplier;
                    } else {
                        accum *= multiplier;
                    }
                    accumulated = true;
                    minibuffer.message(format!("{prompt} {accum}"));
                    continue;
                }
            }
            let digit = match key {
                Digit0 => 0,
                Digit1 => 1,
                Digit2 => 2,
                Digit3 => 3,
                Digit4 => 4,
                Digit5 => 5,
                Digit6 => 6,
                Digit7 => 7,
                Digit8 => 8,
                Digit9 => 9,
                Minus => -1,
                _ => {
                    let world = AsyncWorld::new();
                    let _ = world.resource::<UniversalArg>().get_mut(move |r| {
                        r.0 = (!accumulated).then_some(multiplier).or(Some(accum))
                    });
                    // This last chord isn't what we expected. Send it back for
                    // processing.
                    let _ = world
                        .resource::<KeyChordQueue>()
                        .get_mut(move |r| r.push_back(chord));
                    return;
                }
            };
            if digit >= 0 {
                if accum >= 0 {
                    accum = accum * 10 + digit;
                } else {
                    accum = accum * 10 - digit;
                }
            } else {
                accum *= digit;
            }
            accumulated = true;
            minibuffer.message(format!("{prompt} {accum}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_acts() {
        let plugin = UniversalArgActs::default();
        assert_eq!(plugin.acts.len(), 1);
    }

    #[test]
    fn check_drain_read() {
        let mut plugin = UniversalArgActs::default();
        let _ = plugin.acts.drain();
        assert_eq!(plugin.acts.len(), 0);
    }
}
