use crate::{
    act::{Act, ActsPlugin},
    event::{RunActEvent, RunInputSequenceEvent},
    prelude::{future_sink, keyseq},
    Minibuffer,
};
use asky::Message;
use bevy::prelude::*;
use bevy_defer::{world, AsyncAccess};
use bevy_input_sequence::KeyChord;
use std::{
    fmt::Debug,
    future::Future,
};

pub struct UniversalPlugin {
    pub acts: ActsPlugin,
}

impl Default for UniversalPlugin {
    fn default() -> Self {
        Self {
            acts: ActsPlugin::new(vec![
                Act::new(universal_argument.pipe(future_sink))
                    .named("universal_argument")
                    .hotkey(keyseq! { ctrl-U })
                    .in_exec_act(false),
                Act::new(check_accum.pipe(future_sink))
                    .named("check_accum")
                    .hotkey(keyseq! { C A }),
            ]),
        }
    }
}

impl Plugin for UniversalPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<UniversalArg>()
            .add_systems(bevy::app::Last, clear_arg);
        // XXX: This is kind of funky.
        self.acts.build(app);
    }
}

pub fn check_accum(arg: Res<UniversalArg>, mut minibuffer: Minibuffer) -> impl Future<Output = ()> {
    let accum = arg.0;
    async move {
        let _ = match accum {
            Some(x) => {
                minibuffer
                    .prompt(Message::new(format!("Univeral argument {x}")))
                    .await
            }
            None => {
                minibuffer
                    .prompt(Message::new("No universal argument set"))
                    .await
            }
        };
    }
}

fn clear_arg(mut event: EventReader<RunActEvent>, mut arg: ResMut<UniversalArg>) {
    if let Some(act) = event.read().next() {
        if act.0.name != "exec_act" {
            eprintln!("clear arg for {act}");
            arg.0 = None;
        }
    }
}

#[derive(Debug, Clone, Resource, Default, Reflect)]
pub struct UniversalArg(Option<i32>);

pub fn universal_argument(mut minibuffer: Minibuffer) -> impl Future<Output = ()> {
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
                    let world = world();
                    eprintln!("set accum {accum}");
                    let _ = world
                        .resource::<UniversalArg>()
                        .set(move |r| r.0 = Some(accum))
                        .await;
                    let _ = world.send_event(RunInputSequenceEvent).await;
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
        assert_eq!(plugin.acts.get().len(), 2);
    }

    #[test]
    fn check_drain_read() {
        let plugin = UniversalPlugin::default();
        let _ = plugin.acts.get_mut().drain(..);
        assert_eq!(plugin.acts.get().len(), 0);
    }
}
