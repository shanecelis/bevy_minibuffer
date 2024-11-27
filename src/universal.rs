//! A universal argument, accepts a numerical prefix.
use crate::{
    act::{Act, ActFlags, Acts, PluginOnce},
    event::{RunActEvent, RunInputSequenceEvent},
    prelude::{future_sink, keyseq},
    Minibuffer, MinibufferAsync,
};
use bevy::prelude::*;
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_input_sequence::KeyChord;
use std::{fmt::Debug, future::Future};

/// Universal argument plugin
///
/// Adds act "universal_argument" and resource [UniversalArg].
pub struct UniversalPlugin {
    /// Acts
    pub acts: Acts,
}

impl Default for UniversalPlugin {
    fn default() -> Self {
        Self {
            acts: Acts::new(vec![
                Act::new(universal_argument.pipe(future_sink))
                    .named("universal_argument")
                    .hotkey(keyseq! { Ctrl-U })
                    .in_exec_act(false),
                Act::new(check_accum)
                    .named("check_accum")
                    .hotkey(keyseq! { C A }),
            ]),
        }
    }
}

impl PluginOnce for UniversalPlugin {
    fn build(mut self, app: &mut bevy::app::App) {
        app.init_resource::<UniversalArg>()
            .add_systems(bevy::app::Last, clear_arg);
        // XXX: This is kind of funky.
        self.acts.build(app);
    }
}

/// XXX: This shouldn't be here. It should be in an example.
pub fn check_accum(arg: Res<UniversalArg>, mut minibuffer: Minibuffer) {
    match arg.0 {
        Some(x) => minibuffer.message(format!("Univeral argument {x}")),
        None => minibuffer.message("No universal argument set"),
    }
}

fn clear_arg(mut event: EventReader<RunActEvent>, mut arg: ResMut<UniversalArg>) {
    if let Some(act) = event.read().next() {
        if !act.flags.contains(ActFlags::Adverb) {
            // eprintln!("clear arg for {act}");
            arg.0 = None;
        }
    }
}

/// This resources stores the last given universal argument. It is cleared after
/// any act---that is not specifically marked [ActFlags::Adverb]---runs.
#[derive(Debug, Clone, Resource, Default, Reflect)]
pub struct UniversalArg(Option<i32>);

fn universal_argument(mut minibuffer: MinibufferAsync) -> impl Future<Output = ()> {
    use bevy::prelude::KeyCode::*;
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
            eprintln!("accum {accum}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_acts() {
        let plugin = UniversalPlugin::default();
        assert_eq!(plugin.acts.len(), 2);
    }

    #[test]
    fn check_drain_read() {
        let mut plugin = UniversalPlugin::default();
        let _ = plugin.acts.drain();
        assert_eq!(plugin.acts.len(), 0);
    }
}
