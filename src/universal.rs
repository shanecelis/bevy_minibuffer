//! A universal argument, accepts a numerical prefix.
//!
//! Can be queried by other commands using the [UniversalArg] resource.
use crate::{
    act::{Act, ActFlags, Acts, ActsPlugin},
    event::{LastRunAct, RunActEvent, RunInputSequenceEvent},
    prelude::{future_sink, keyseq},
    MinibufferAsync,
};
use bevy::prelude::*;
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::KeyChord;
use std::{borrow::Cow, fmt::Debug, future::Future};

/// Universal argument plugin
///
/// Adds act "universal_argument" and resource [UniversalArg].
pub struct UniversalArgActs {
    /// Acts
    pub acts: Acts,
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

impl Plugin for UniversalArgActs {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<UniversalArg>()
            .add_systems(bevy::app::Last, clear_arg);
        if !self.acts.is_empty() {
            warn!(
                "universal plugin has {} that acts were not added; consider using add_acts() on the plugin.",
                self.acts.len()
            );
        }
    }
}

impl ActsPlugin for UniversalArgActs {
    fn take_acts(&mut self) -> Acts {
        self.acts.take()
    }
}

fn clear_arg(
    mut event: EventReader<RunActEvent>,
    mut arg: ResMut<UniversalArg>,
    mut clear: Local<Option<Cow<'static, str>>>,
) {
    // Wait a frame to clear it.
    if let Some(_act) = clear.take() {
        // info!("clear arg for {act}");
        arg.0 = None;
    }
    if let Some(act) = event.read().next() {
        if !act.flags.contains(ActFlags::Adverb) {
            *clear = Some(act.name.clone());
        }
    }
}

/// This resources stores the last given universal argument. It is cleared after
/// any act---that is not specifically marked [ActFlags::Adverb]---runs.
#[derive(Debug, Clone, Resource, Default, Reflect)]
pub struct UniversalArg(pub Option<i32>);

fn universal_argument(
    mut minibuffer: MinibufferAsync,
    last_act: Res<LastRunAct>,
) -> impl Future<Output = ()> {
    use bevy::prelude::KeyCode::*;

    let prompt: Cow<'static, str> = (*last_act)
        .as_ref()
        .and_then(|run_act| {
            run_act
                .hotkey
                .map(|index| format!("{}", run_act.act.hotkeys[index]).into())
        })
        .unwrap_or("universal_argument ".into());

    minibuffer.message(format!("{prompt}"));
    async move {
        let mut accum = 0;
        loop {
            let Ok(KeyChord(_mods, key)) = minibuffer.get_chord().await else {
                break;
            };
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
                    // info!("set universal arg to {accum}");
                    let _ = world
                        .resource::<UniversalArg>()
                        .set(move |r| r.0 = Some(accum));
                    let _ = world.send_event(RunInputSequenceEvent);
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
