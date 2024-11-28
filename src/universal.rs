//! A universal argument, accepts a numerical prefix.
//!
//! Can be queried by other commands using the [UniversalArg] resource.
use crate::{
    act::{Act, ActFlags, Acts, ActsPlugin},
    event::{LastRunAct, RunActEvent, RunInputSequenceEvent},
    prelude::{future_sink, keyseq},
    Minibuffer, MinibufferAsync,
};
use bevy::prelude::*;
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::KeyChord;
use std::{borrow::Cow, fmt::Debug, future::Future};

/// Universal argument plugin
///
/// Adds act "universal_argument" and resource [UniversalArg].
pub struct UniversalArgPlugin {
    /// Acts
    pub acts: Acts,
}

impl Default for UniversalArgPlugin {
    fn default() -> Self {
        Self {
            acts: Acts::new(vec![
                Act::new(universal_argument.pipe(future_sink))
                    .named("universal_argument")
                    .hotkey(keyseq! { Ctrl-U })
                    .sub_flags(ActFlags::ExecAct),
                Act::new(check_accum)
                    .named("check_accum")
                    .hotkey(keyseq! { C A }),
            ]),
        }
    }
}

impl Plugin for UniversalArgPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<UniversalArg>()
            .add_systems(bevy::app::Last, clear_arg);
        if !self.acts.is_empty() {
            warn!(
                "universal plugin has {} that acts were not added.",
                self.acts.len()
            );
        }
    }
}

impl ActsPlugin for UniversalArgPlugin {
    fn take_acts(&mut self) -> Acts {
        self.acts.take()
    }
}

/// XXX: This shouldn't be here. It should be in an example.
pub fn check_accum(arg: Res<UniversalArg>, mut minibuffer: Minibuffer) {
    eprintln!("BEGIN: check_accum");
    match arg.0 {
        Some(x) => minibuffer.message(format!("Univeral argument {x}")),
        None => minibuffer.message("No universal argument set"),
    }
    eprintln!("END: check_accum");
}

fn clear_arg(
    mut event: EventReader<RunActEvent>,
    mut arg: ResMut<UniversalArg>,
    mut clear: Local<Option<Cow<'static, str>>>,
) {
    // Wait a frame to clear it.
    if let Some(act) = clear.take() {
        eprintln!("clear arg for {act}");
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
pub struct UniversalArg(Option<i32>);

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
        .unwrap_or("universal_argument".into());

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
                _ => {
                    let world = AsyncWorld::new();
                    eprintln!("set accum {accum}");
                    let _ = world
                        .resource::<UniversalArg>()
                        .set(move |r| r.0 = Some(accum));
                    let _ = world.send_event(RunInputSequenceEvent);
                    return;
                }
            };
            if digit >= 0 {
                accum = accum * 10 + digit;
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
        let plugin = UniversalArgPlugin::default();
        assert_eq!(plugin.acts.len(), 2);
    }

    #[test]
    fn check_drain_read() {
        let mut plugin = UniversalArgPlugin::default();
        let _ = plugin.acts.drain();
        assert_eq!(plugin.acts.len(), 0);
    }
}
