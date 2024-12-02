//! An act adverb that accepts a numerical prefix
//!
//! Can be queried by other commands using the [UniversalArg] resource.
use crate::{
    acts::{Act, ActFlags, Acts, ActsPlugin},
    event::{LastRunAct, RunActEvent},
    prelude::{future_sink, keyseq},
    Minibuffer, MinibufferAsync,
};
use bevy::prelude::*;
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::{KeyChord, KeyChordQueue};
use std::{borrow::Cow, fmt::Debug, future::Future};

/// Universal argument plugin
///
/// Adds act "universal_argument" and resource [UniversalArg].
pub struct UniversalArgActs {
    /// Acts
    pub acts: Acts,
}

/// Universal multiplier
///
/// When universal argument's key binding is invoked multiple times, its
/// multiplied by the number stored in this resource. By default that number is
/// four initially.
#[derive(Resource)]
pub struct UniversalMultiplier(i32);

impl Default for UniversalMultiplier {
    fn default() -> Self {
        Self(4)
    }
}

impl Default for UniversalArgActs {
    fn default() -> Self {
        Self {
            acts: Acts::new(vec![Act::new(universal_argument.pipe(future_sink))
                .named("universal_argument")
                .bind(keyseq! { Ctrl-U })
                .sub_flags(ActFlags::ExecAct)]),
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
}

impl Plugin for UniversalArgActs {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<UniversalMultiplier>()
            .init_resource::<UniversalArg>()
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
    if let Some(act) = event.read().next() {
        if !act.flags.contains(ActFlags::Adverb) {
            *clear = Some(act.name.clone());
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
#[derive(Debug, Clone, Resource, Default, Reflect)]
pub struct UniversalArg(pub Option<i32>);

fn universal_argument(
    mut minibuffer: MinibufferAsync,
    multiplier: Res<UniversalMultiplier>,
    last_act: Res<LastRunAct>,
) -> impl Future<Output = ()> {
    use bevy::prelude::KeyCode::*;

    let mut bindkey: Option<KeyChord> = None;
    let multiplier: i32 = multiplier.0;
    let prompt: Cow<'static, str> = (*last_act)
        .as_ref()
        .and_then(|run_act| {
            run_act.hotkey.map(|index| {
                let keyseq = &run_act.act.hotkeys[index];
                if keyseq.chords.len() == 1 {
                    bindkey = Some(keyseq.chords[0].clone());
                }
                format!("{}", keyseq).into()
            })
        })
        .unwrap_or("universal_argument ".into());

    minibuffer.message(format!("{prompt}"));
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
                    minibuffer.message(format!("{prompt}{accum}"));
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
                // TODO: If the key chord is hit multiple times, we should
                // add some multiple to it like 4 or 8.
                //
                // KeyU => {
                // }
                _ => {
                    let world = AsyncWorld::new();
                    let _ = world
                        .resource::<UniversalArg>()
                        .set(move |r| r.0 = (!accumulated).then_some(multiplier).or(Some(accum)));
                    // This last chord isn't what we expected. Send it back for
                    // processing.
                    let _ = world
                        .resource::<KeyChordQueue>()
                        .set(move |r| r.push_back(chord));
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
            minibuffer.message(format!("{prompt}{accum}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_acts() {
        let plugin = UniversalArgActs::default();
        assert_eq!(plugin.acts.len(), 2);
    }

    #[test]
    fn check_drain_read() {
        let mut plugin = UniversalArgActs::default();
        let _ = plugin.acts.drain();
        assert_eq!(plugin.acts.len(), 0);
    }
}
