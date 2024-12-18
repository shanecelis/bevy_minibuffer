use crate::{
    acts::{Act, AddActs, ActFlags},
    event::run_script,
    input::keyseq,
    event::RunActEvent,
    Minibuffer,
};
use std::sync::Arc;
use bevy::prelude::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<Tape>()
        .init_resource::<Tapes>()
        .add_acts((
            Act::new(record_macro).bind(keyseq! { Q }).sub_flags(ActFlags::Record),
            Act::new(play_macro).bind(keyseq! { Shift-2 }).sub_flags(ActFlags::Record),
            ))
        ;
}

#[derive(Resource, Debug, Default)]
pub enum Tape {
    #[default]
    Off,
    Record(Script),
    Play,
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(Vec<Script>);

fn record_macro(mut minibuffer: Minibuffer, mut tapes: ResMut<Tapes>) {
    use Tape::*;
    let new_value = match *minibuffer.tape {
        Off => {
            minibuffer.message("Recording new macro");
            Record(Script::default())
        }
        Record(ref mut script) => {
            tapes.push(std::mem::take(script));
            minibuffer.message("Defined macro");
            Off
        }
        Play => {
            warn!("Got record macro during Play.");
            Play
        }
    };
    *minibuffer.tape = new_value;
}

fn play_macro(world: &mut World) {
    let tape = {
        world.resource::<Tapes>().iter().cloned().last()
    };
    if let Some(tape) = tape {
        info!("Running system.");
        if let Err(e) = world.run_system_cached_with(run_script, &tape) {
            warn!("Error playing macro: {e:?}");
        }
    } else {
        warn!("No macro on tape.");
    }
}

#[derive(Debug, Default, Clone)]
pub struct Script {
    pub content: Vec<RunActEvent>,
}

impl Script {
    pub fn append_run(&mut self, act: RunActEvent) {
        self.content.push(act);
    }

    pub fn ammend_input<I: Clone + 'static + Send + Sync>(&mut self, input: I) {
        if let Some(ref mut entry) = self.content.last_mut() {
            if entry.input.is_some() {
                warn!("Overwriting script input for act {}", &entry.act.name);
            }
            entry.input = Some(Arc::new(input));
        } else {
            warn!("Cannot append input; no act has been run.");
        }
    }
}
