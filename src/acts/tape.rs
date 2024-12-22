use crate::{
    acts::{Act, Acts, ActsPlugin, AddActs, ActFlags, ActRunner, universal::UniversalArg},
    input::{KeyChord, keyseq},
    event::{RunActEvent, KeyChordEvent, LastRunAct},
    Minibuffer,
};
use std::{fmt::{self, Debug, Write}, sync::Arc, collections::HashMap, any::{Any, TypeId}, borrow::Cow};
use bevy::prelude::*;
#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<TapeRecorder>()
        .init_resource::<DebugMap>();
}

pub struct TapeActs {
    acts: Acts,
}

impl Default for TapeActs {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                Act::new(record_tape).bind(keyseq! { Q }).sub_flags(ActFlags::Record),
                Act::new(play_tape).bind_aliased(keyseq! { Shift-2 }, "@").sub_flags(ActFlags::Record),
                Act::new(repeat).bind(keyseq! { Period }).sub_flags(ActFlags::Record | ActFlags::RunAct),
                &mut Act::new(copy_tape),
                ]),
        }
    }
}

impl Plugin for TapeActs {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Tapes>()
            .init_resource::<LastPlayed>();

        self.warn_on_unused_acts();
    }
}

impl ActsPlugin for TapeActs {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}

#[derive(Resource, Debug)]
pub enum TapeRecorder {
    /// Record only the last act that was run.
    Off { one_off: Tape },
    Record { tape: Tape, chord: KeyChord },
    Play,
}

impl Default for TapeRecorder {
    fn default() -> Self {
        TapeRecorder::Off { one_off: Tape::default() }
    }
}

impl TapeRecorder {
    pub fn process_event(&mut self, event: &RunActEvent) {
        if event.act.flags.contains(ActFlags::Record) {
            match self {
                TapeRecorder::Off { one_off: ref mut tape } => {
                    tape.content.clear();
                    tape.append_run(event.clone());
                }
                TapeRecorder::Record { ref mut tape, .. } => {
                    tape.append_run(event.clone());
                }
                _ => ()
            }
        }
    }

    pub fn process_input<I: Debug + Clone + Send + Sync + 'static>(&mut self, input: &I, debug_map: &mut DebugMap) {
        match self {
            TapeRecorder::Record { ref mut tape, .. } | TapeRecorder::Off { one_off: ref mut tape } => {
                tape.ammend_input(input.clone(), debug_map);
            }
            _ => ()
        }
    }
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(HashMap<KeyChord, Tape>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DebugMap(HashMap<TypeId, Box<dyn Fn(&dyn Any) -> Option<String> + 'static + Send + Sync>>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LastPlayed(Option<KeyChord>);

fn record_tape(mut minibuffer: Minibuffer, mut tapes: ResMut<Tapes>) {
    match &*minibuffer.tape_recorder {
        TapeRecorder::Off { one_off }=> {
            minibuffer.message("Record tape for key: ");
            minibuffer.get_chord()
                .observe(|mut trigger: Trigger<KeyChordEvent>, mut commands: Commands, mut minibuffer: Minibuffer| {
                    match trigger.event_mut().take() {
                        Ok(chord) => {
                            minibuffer.message(format!("Recording new tape for {}", &chord));
                            *minibuffer.tape_recorder = TapeRecorder::Record { tape: Tape::default(), chord: chord };
                        }
                        Err(e) => {
                            minibuffer.message(format!("{e}"));
                        }
                    }
                    commands.entity(trigger.entity()).despawn_recursive();
                });
        }
        TapeRecorder::Record { .. } => {
            let TapeRecorder::Record { tape, chord } = std::mem::take(&mut *minibuffer.tape_recorder) else {
                unreachable!();
            };
            minibuffer.message(format!("Defined tape {}", &chord));
            tapes.insert(chord, tape);
        }
        TapeRecorder::Play => {
            warn!("Got record tape during Play.");
        }
    }
}

fn play_tape(mut minibuffer: Minibuffer, mut acts: Query<&Act>, last_act: Res<LastRunAct>, universal_arg: Res<UniversalArg>) {
    let this_keychord = last_act.hotkey(&mut acts.as_query_lens());
    let count = universal_arg.unwrap_or(1);
    minibuffer.message("Play tape for key: ");
    minibuffer.get_chord()
        .observe(move |mut trigger: Trigger<KeyChordEvent>, mut commands: Commands,
                 tapes: Res<Tapes>, mut minibuffer: Minibuffer, mut last_played: ResMut<LastPlayed>| {
            match trigger.event_mut().take() {
                Ok(mut chord) => 'body: {
                    if this_keychord.as_ref().map(|x| x.chords.len() == 1 && x.chords[0] == chord).unwrap_or(false) {
                        // We want to play the same chord as last time.
                        if let Some(ref last_played) = **last_played {
                            chord.clone_from(last_played);
                        } else {
                            minibuffer.message("Tried to play last tape but no tape has been played.");
                            break 'body;
                        }
                    }
                    if let Some(tape) = tapes.get(&chord) {
                        let tape = tape.clone();
                        commands.queue(move |world: &mut World| {
                            for _ in 0..count {
                                if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                                    warn!("Error playing tape: {e:?}");
                                }
                            }
                        });
                        **last_played = Some(chord);
                    } else {
                        minibuffer.message(format!("No tape for {}", &chord));
                    }
                }
                Err(e) => {
                    minibuffer.message(format!("{e}"));
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
                 tapes: Res<Tapes>, mut minibuffer: Minibuffer, runner: Query<&ActRunner>, acts: Query<&Act>|
                 {
                     match trigger.event_mut().take() {
                         Ok(chord) => 'press: {
                             if let Some(tape) = tapes.get(&chord) {

                                 let script = match tape.script(&acts, &runner, &minibuffer.debug_map) {
                                     Ok(s) => s,
                                     Err(e) => {
                                         minibuffer.message(format!("Could not generate script: {e}"));
                                         break 'press;
                                     }
                                 };
                                 info!("{}", script);
                                 #[cfg(feature = "clipboard")]
                                 {
                                     match ClipboardContext::new() {
                                         Ok(mut ctx) => {
                                             if let Err(e) = ctx.set_contents(script.clone()) {
                                                 warn!("Could not set clipboard: {e}");
                                             }
                                             minibuffer.message(format!("Copy tape {} to clipboard and log:\n\n{}.", &chord, &script));
                                         }
                                         Err(e) => {
                                             minibuffer.message(format!("Log tape {}:\n\n{}", &chord, &script));
                                             warn!("Could not initialize clipboard: {e}");
                                         }
                                     }
                                 }
                                 #[cfg(not(feature = "clipboard"))]
                                 minibuffer.message(format!("Log tape {}:\n\n{}", &chord, &script));
                             } else {
                                 minibuffer.message(format!("No tape for {}", &chord));
                             }
                         }
                         Err(e) => {
                             minibuffer.message(format!("{e}"));
                         }
                     }
                     commands.entity(trigger.entity()).despawn_recursive();
                 });
}

fn repeat(tape_recorder: Res<TapeRecorder>, universal_arg: Res<UniversalArg>, mut commands: Commands) {
    let count = universal_arg.unwrap_or(1);
    match *tape_recorder {
        TapeRecorder::Off { ref one_off } => {
            let tape = one_off.clone();
            commands.queue(move |world: &mut World| {
                for _ in 0..count {
                    if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                        warn!("Error playing tape: {e:?}");
                    }
                }
            });
        }
        _ => ()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Tape {
    pub content: Vec<RunActEvent>,
}

impl Tape {
    pub fn append_run(&mut self, act: RunActEvent) {
        self.content.push(act);
    }

    pub fn ammend_input<I: Clone + 'static + Send + Sync + Debug>(&mut self, input: I, debug_map: &mut DebugMap) {
        if let Some(ref mut entry) = self.content.last_mut() {
            if entry.input.is_some() {
                warn!("Overwriting tape input for act {}", &entry.act.id);
            }
            let type_id = TypeId::of::<I>();
            // info!("put type_id in {type_id:?}");
            debug_map.entry(type_id).or_insert_with(|| Box::new(|boxed_input: &dyn Any| {

                // info!("debug_str_fn type_id in {:?}", boxed_input.type_id());
                boxed_input.downcast_ref::<I>().map(|input: &I| format!("{:?}", input))
            }));
            entry.input = Some(Arc::new(input));
        } else {
            warn!("Cannot append input; no act has been run.");
        }
    }

    fn script(&self, acts: &Query<&Act>, runner: &Query<&ActRunner>, debug_map: &DebugMap) -> Result<String, fmt::Error> {
        let mut f = String::new();
        writeln!(f, "fn tape(mut commands: Commands) {{")?;
        for event in &self.content {
            let Ok(act) = acts.get(event.act.id) else { continue; };
            let Ok(act_runner) = runner.get(act.system_id) else {
                warn!("Cannot add act {:?} to script: no act runner.", &act.name);
                writeln!(f, "    // Skipping {:?}; no act runner.", &act.name)?;
                continue;
            };
            match event.input {
                Some(ref input) => {
                    let type_id = (&**input).type_id();
                    // info!("try to get type_id out {type_id:?}");
                    let input_string: Cow<'static, str> = match debug_map.get(&type_id) {
                        Some(debug_str_fn) => {
                            match debug_str_fn(&**input) {
                                Some(s) => s.into(),
                                None => {
                                    warn!("Debug string function failed for act {:?}", &act.name);
                                    "???".into()
                                }
                            }
                        }
                        None => {
                            warn!("No debug string function for act {:?}", &act.name);
                            "!!!".into()
                        }
                    };
                    writeln!(f, "    commands.run_system_cached_with({}, {})", &act_runner.system_name(),
                                input_string)?;
                }
                None => {
                    writeln!(f, "    commands.run_system_cached({})", &act_runner.system_name())?;
                }
            }
        }
        write!(f, "}}")?;
        Ok(f)
    }
}

// impl fmt::Display for Tape {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         writeln!(f, "fn tape(mut commands: Minibuffer) {{")?;
//         for event in &self.content {
//             match event.input {
//                 Some(ref input) => {
//                     writeln!(f, "    minibuffer.run_act_with_input({:?}, {})", &event.act.name, event.input_debug.as_deref().unwrap_or_else(|| "???"))?;
//                 }
//                 None => {
//                     writeln!(f, "    minibuffer.run_act({:?})", &event.act.name)?;
//                 }
//             }
//         }
//         write!(f, "}}")
//     }
// }

pub fn play_tape_sys(InRef(tape): InRef<Tape>,
                     mut next_prompt_state: ResMut<NextState<crate::prompt::PromptState>>,
                     mut commands: Commands,
                     mut last_act: ResMut<crate::event::LastRunAct>,
                     mut tape_recorder: ResMut<TapeRecorder>,
                     acts: Query<&Act>,
                     runner: Query<&crate::acts::ActRunner>) {
    let old = std::mem::replace(&mut *tape_recorder, TapeRecorder::Play);
    for e in &tape.content {
        let Ok(act) = acts.get(e.act.id) else {
            warn!("Could not get act for {:?}", e.act.id);
            continue;
        };
        let act = acts.get(e.act.id).ok();
        let runner = act.as_ref().and_then(|a|runner.get(a.system_id).ok());
        crate::event::run_act_raw(e, act, runner, &mut next_prompt_state, &mut last_act, &mut commands, None);
    }
    let _ = std::mem::replace(&mut *tape_recorder, old);
}
