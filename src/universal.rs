use crate::{
    act::{Act, AddAct},
    event::{RunActEvent, RunInputSequenceEvent},
    lookup::{LookUp, LookUpError, Resolve},
    prompt::{CompletionState, PromptState},
    Minibuffer,
    prelude::{future_sink, keyseq},
};
use asky::Message;
use bevy::{ecs::system::SystemId, prelude::*, window::RequestRedraw};
use bevy_defer::{world, AsyncAccess};
use bevy_input_sequence::{KeyChord, input_sequence::KeySequence};
use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display, Write},
    future::Future,
};
use tabular::{Row, Table};
use trie_rs::map::{Trie, TrieBuilder};

pub struct UniversalPlugin;

impl Plugin for UniversalPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .init_resource::<UniversalArg>()
            .add_systems(bevy::app::Last, clear_arg)
            .add_act(Act::new(universal_argument.pipe(future_sink))
                     .named("universal_argument")
                     .hotkey(keyseq! { ctrl-U })
                     .in_exec_act(false))
            .add_act(Act::new(check_accum.pipe(future_sink))
                     .named("check_accum")
                     .hotkey(keyseq! { C A }));
    }
}

pub fn check_accum(arg: Res<UniversalArg>,
                   mut minibuffer: Minibuffer) -> impl Future<Output = ()> {
    let accum = arg.0.clone();
    async move {
        let _ = match accum {
            Some(x) => minibuffer.prompt(Message::new(format!("Univeral argument {x}"))).await,
            None => minibuffer.prompt(Message::new("No universal argument set")).await,
        };
    }
}

fn clear_arg(mut event: EventReader<RunActEvent>,
             mut arg: ResMut<UniversalArg>) {
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
            let Ok(KeyChord(mods, key)) = minibuffer.get_chord().await else { break };
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
                    let _ = world.resource::<UniversalArg>().set(move |r| {r.0 = Some(accum)}).await;
                    let _ = world.send_event(RunInputSequenceEvent).await;
                    return;
                },
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
