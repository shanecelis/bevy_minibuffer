use crate::{
    acts::{Act, AddActs, ActFlags},
    input::keyseq,
    event::RunActEvent,
    Minibuffer,
};
use std::{fmt::{self, Debug}, sync::Arc};
use bevy::prelude::*;
#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<TapeRecorder>()
        .init_resource::<Tapes>()
        .add_acts((
            Act::new(record_macro).bind(keyseq! { Q }).sub_flags(ActFlags::Record),
            Act::new(play_macro).bind(keyseq! { Shift-2 }).sub_flags(ActFlags::Record),
            Act::new(copy_macro),
            ))
        ;
}

#[derive(Resource, Debug, Default)]
pub enum TapeRecorder {
    #[default]
    Off,
    Record(Script),
    Play,
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(Vec<Script>);

fn record_macro(mut minibuffer: Minibuffer, mut tapes: ResMut<Tapes>) {
    use TapeRecorder::*;
    let new_value = match *minibuffer.tape_recorder {
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
    *minibuffer.tape_recorder = new_value;
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

#[cfg(feature = "clipboard")]
fn copy_macro(mut minibuffer: Minibuffer, mut tapes: Res<Tapes>) {
    match ClipboardContext::new() {
        Ok(mut ctx) => {
            if let Some(ref tapes) = tapes.last() {
                info!("{}", tapes);
                ctx.set_contents(tapes.to_string()).expect("copy script to clipboard");
                minibuffer.message("Copied script to clipboard.");
            } else {
                minibuffer.message("No tapes available.");
            }
        }
        Err(e) => {
            minibuffer.message("Could not initialize clipboard: {e:?}");
        }
    }
}

#[cfg(not(feature = "clipboard"))]
fn copy_macro(mut minibuffer: Minibuffer, mut tapes: Res<Tapes>) {
    if let Some(ref tapes) = tapes.last() {
        info!("{}", tapes);
        minibuffer.message("Print script on stdout.");
    } else {
        minibuffer.message("No tapes available.");
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

    pub fn ammend_input<I: Clone + 'static + Send + Sync + Debug>(&mut self, input: I) {
        if let Some(ref mut entry) = self.content.last_mut() {
            if entry.input.is_some() {
                warn!("Overwriting script input for act {}", &entry.act.name);
            }
            entry.input_debug = Some(format!("{:?}", &input));
            entry.input = Some(Arc::new(input));
        } else {
            warn!("Cannot append input; no act has been run.");
        }
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "fn script(mut minibuffer: Minibuffer) {{")?;
        for event in &self.content {
            match event.input {
                Some(ref input) => {
                    writeln!(f, "    minibuffer.run_act_with_input({:?}, {})", &event.act.name, event.input_debug.as_deref().unwrap_or_else(|| "???"))?;
                }
                None => {
                    writeln!(f, "    minibuffer.run_act({:?})", &event.act.name)?;
                }
            }
        }
        write!(f, "}}")
    }
}

pub fn run_script(InRef(script): InRef<Script>,
    mut next_prompt_state: ResMut<NextState<crate::prompt::PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<crate::event::LastRunAct>,
    runner: Query<&crate::acts::ActRunner>) {
    for e in &script.content {
        crate::event::run_act_raw(e, runner.get(e.act.system_id).ok(), &mut next_prompt_state, &mut last_act, &mut commands, None);
    }
}
