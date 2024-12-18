use crate::{

    acts::{Act, AddActs, ActFlags},
    input::{KeyChord, keyseq},
    event::{RunActEvent, KeyChordEvent},
    Minibuffer,
};
use std::{fmt::{self, Debug}, sync::Arc, collections::HashMap};
use bevy::prelude::*;
#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<TapeRecorder>()
        .init_resource::<Tapes>()
        .add_acts((
            Act::new(record_tape).bind(keyseq! { Q }).sub_flags(ActFlags::Record),
            Act::new(play_tape).bind(keyseq! { Shift-2 }).sub_flags(ActFlags::Record),
            Act::new(copy_tape),
            ))
        ;
}

#[derive(Resource, Debug, Default)]
pub enum TapeRecorder {
    #[default]
    Off,
    Record { tape: Tape, chord: KeyChord },
    Play,
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(HashMap<KeyChord, Tape>);

fn record_tape(mut minibuffer: Minibuffer, mut tapes: ResMut<Tapes>) {

    use TapeRecorder::*;
    match *minibuffer.tape_recorder {
        Off => {
            minibuffer.message("Record tape for key: ");
            minibuffer.get_chord()
                .observe(|mut trigger: Trigger<KeyChordEvent>, mut commands: Commands, mut minibuffer: Minibuffer| {
                    match trigger.event_mut().take() {
                        Some(chord) => {
                            minibuffer.message(format!("Recording new tape for {}", &chord));
                            *minibuffer.tape_recorder = Record { tape: Tape::default(), chord: chord };
                        }
                        None => {
                            minibuffer.message("Could not get key.");
                        }
                    }
                    commands.entity(trigger.entity()).despawn_recursive();
                });
        }
        Record { ..  } => {
            let Record { tape, chord } = std::mem::take(&mut *minibuffer.tape_recorder) else {
                unreachable!();
            };
            minibuffer.message(format!("Defined tape {}", &chord));
            tapes.insert(chord, tape);
        }
        Play => {
            warn!("Got record tape during Play.");
        }
    }
}

fn play_tape(mut minibuffer: Minibuffer) {
    minibuffer.message("Play tape for key: ");
    minibuffer.get_chord()
        .observe(|mut trigger: Trigger<KeyChordEvent>, mut commands: Commands,
                    tapes: Res<Tapes>, mut minibuffer: Minibuffer| {
            match trigger.event_mut().take() {
                Some(chord) => {
                    if let Some(tape) = tapes.get(&chord) {
                        let tape = tape.clone();
                        commands.queue(move |world: &mut World| {
                            info!("Running system.");
                            if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                                warn!("Error playing tape: {e:?}");
                            }
                        });
                    } else {
                        minibuffer.message(format!("No tape for {}", &chord));
                    }
                }
                None => {
                    minibuffer.message("Could not get key.");
                }
            }
            commands.entity(trigger.entity()).despawn_recursive();
        });
}

fn copy_tape(mut minibuffer: Minibuffer) {
    minibuffer.message("Copy tape for key: ");
    minibuffer
        .get_chord()
        .observe(|mut trigger: Trigger<KeyChordEvent>, mut commands: Commands,
                 tapes: Res<Tapes>, mut minibuffer: Minibuffer|
                 {
                     match trigger.event_mut().take() {
                         Some(chord) => {
                             if let Some(tape) = tapes.get(&chord) {
                                 info!("{}", tape);
                                 #[cfg(feature = "clipboard")]
                                 {
                                     match ClipboardContext::new() {
                                         Ok(mut ctx) => {
                                             if let Err(e) = ctx.set_contents(tape.to_string()) {
                                                 warn!("Could not set clipboard: {e}");
                                             }
                                             minibuffer.message(format!("Copy tape {} to clipboard and log:\n\n{}.", &chord, tape));
                                         }
                                         Err(e) => {
                                             minibuffer.message(format!("Log tape {}:\n\n{}", &chord, tape));
                                             warn!("Could not initialize clipboard: {e}");
                                         }
                                     }
                                 }
                                 #[cfg(not(feature = "clipboard"))]
                                 minibuffer.message(format!("Log tape {}:\n\n{}", &chord, tape));
                             } else {
                                 minibuffer.message(format!("No tape for {}", &chord));
                             }
                         }
                         None => {
                             minibuffer.message("Could not get key.");
                         }
                     }
                     commands.entity(trigger.entity()).despawn_recursive();
                 });
}

#[derive(Debug, Default, Clone)]
pub struct Tape {
    pub content: Vec<RunActEvent>,
}

impl Tape {
    pub fn append_run(&mut self, act: RunActEvent) {
        self.content.push(act);
    }

    pub fn ammend_input<I: Clone + 'static + Send + Sync + Debug>(&mut self, input: I) {
        if let Some(ref mut entry) = self.content.last_mut() {
            if entry.input.is_some() {
                warn!("Overwriting tape input for act {}", &entry.act.name);
            }
            entry.input_debug = Some(format!("{:?}", &input));
            entry.input = Some(Arc::new(input));
        } else {
            warn!("Cannot append input; no act has been run.");
        }
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "fn tape(mut minibuffer: Minibuffer) {{")?;
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

pub fn play_tape_sys(InRef(tape): InRef<Tape>,
                     mut next_prompt_state: ResMut<NextState<crate::prompt::PromptState>>,
                     mut commands: Commands,
                     mut last_act: ResMut<crate::event::LastRunAct>,
                     mut tape_recorder: ResMut<TapeRecorder>,
                     runner: Query<&crate::acts::ActRunner>) {
    let old = std::mem::replace(&mut *tape_recorder, TapeRecorder::Play);
    for e in &tape.content {
        crate::event::run_act_raw(e, runner.get(e.act.system_id).ok(), &mut next_prompt_state, &mut last_act, &mut commands, None);
    }
    let _ = std::mem::replace(&mut *tape_recorder, old);
}
