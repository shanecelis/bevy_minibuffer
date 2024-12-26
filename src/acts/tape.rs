use crate::{
    acts::{Act, Acts, ActsPlugin, ActFlags, RunActMap,
           universal::UniversalArg},
    ui::IconContainer,
    input::{KeyChord, keyseq},
    event::{RunActEvent, KeyChordEvent, LastRunAct},
    Minibuffer,
};
use std::{fmt::{self, Debug, Write}, sync::Arc, collections::HashMap, borrow::Cow};
use bevy::prelude::*;
#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<TapeRecorder>()
        .init_resource::<TapeAnimate>()
        .init_resource::<RunActMap>();
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
                // Act::new(repeat).bind(keyseq! { Period }).sub_flags(ActFlags::Record | ActFlags::RunAct),
                Act::new(repeat).bind(keyseq! { Period }).sub_flags(ActFlags::Record),
                &mut Act::new(copy_tape),
                ]),
        }
    }
}

#[derive(Resource, Default)]
enum TapeAnimate {
    Speed(f32),
    Curve { curve: Box<dyn Curve<f32> + 'static + Send + Sync>, pos: f32 }, // then: Option<Box<TapeAnimate>> },
    #[default]
    Hide,
}

impl TapeAnimate {
    fn curve(curve: impl Curve<f32> + Send + Sync + 'static) -> Self {
        Self::Curve { curve: Box::new(curve), pos: 0.0 } //, then: None }
    }
}

// struct TapeAnimator(Vec<TapeAnimate>);

#[derive(Component)]
struct TapeIcon;

impl Plugin for TapeActs {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Tapes>()
            .init_resource::<LastPlayed>()
            .add_systems(Startup, setup_icon)
            .add_systems(Update, animate_icon);

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
const PADDING: Val = Val::Px(5.0);

fn setup_icon(mut commands: Commands, query: Query<Entity, With<IconContainer>>, asset_loader: Res<AssetServer>) {
    let icon_container = query.single();
    commands.entity(icon_container)
        .with_children(|parent| {
            parent.spawn((ImageNode::new(asset_loader.load("tape.png")),
                          Node {
                              width: Val::Px(25.0),
                              height: Val::Px(25.0),
                              margin: UiRect {
                                  top: PADDING,
                                  left: PADDING,
                                  right: PADDING,
                                  bottom: PADDING,
                              },
                              aspect_ratio: Some(1.0),
                              ..default()
                          },
                          // .with_mode(NodeImageMode::Stretch),
                          TapeIcon));
        });
}

// fn play_icon(mut query: Query<&mut Transform, With<TapeIcon>>, recorder: Res<TapeRecorder>, time: Res<Time>) {
//     let speed = match *recorder {
//         TapeRecorder::Off { .. } => None,
//         TapeRecorder::Record { .. } => Some(1.0),
//         TapeRecorder::Play => None,
//     };

//     if let Some(speed) = speed {
//         for mut transform in &mut query {
//             transform.rotate_local_z(speed * time.delta_secs());
//         }
//     }
// }

fn animate_icon(mut query: Query<&mut Transform, With<TapeIcon>>,
                mut animate: ResMut<TapeAnimate>,
                last_speed: Local<Option<f32>>,
                time: Res<Time>) {
    let speed = match *animate {
        TapeAnimate::Speed(speed) => Some(speed),
        TapeAnimate::Hide => None,
        TapeAnimate::Curve { ref curve, ref mut pos } => {
            let r = curve.sample(*pos);
            *pos += time.delta_secs();
            // This could be none. Should do something at that point.
            r.or_else(|| *last_speed)
        }
    };
    // *last_speed = speed;

    if let Some(speed) = speed {
        for mut transform in &mut query {
            transform.rotate_local_z(speed * time.delta_secs());
        }
    }
}

#[derive(Resource, Debug)]
pub enum TapeRecorder {
    /// Record only the last act that was run.
    Off { one_off: Tape },
    /// A Tape is being recorded.
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

    pub fn process_input<I: Debug + Clone + Send + Sync + 'static>(&mut self, input: &I) {
        match self {
            TapeRecorder::Record { ref mut tape, .. } | TapeRecorder::Off { one_off: ref mut tape } => {
                tape.ammend_input(input.clone());
            }
            _ => ()
        }
    }
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(HashMap<KeyChord, Tape>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LastPlayed(Option<KeyChord>);

fn easing(amp: f32, duration: f32, func: EaseFunction) -> Option<LinearReparamCurve<f32, EasingCurve<f32>>> {
    let domain = Interval::new(0.0, duration).ok()?;
    EasingCurve::new(0.0, amp, func).reparametrize_linear(domain).ok()
}

fn record_tape(mut minibuffer: Minibuffer, mut tapes: ResMut<Tapes>, universal: Res<UniversalArg>, mut animate: ResMut<TapeAnimate>) {
    *animate = TapeAnimate::curve(easing(-3.0, 4.0, EaseFunction::Steps(3)).unwrap());//.map(|x| -x));
    let append = universal.is_some();
    match &*minibuffer.tape_recorder {
        TapeRecorder::Off { one_off: _ }=> {
            minibuffer.message("Record tape: ");
            minibuffer.get_chord()
                .observe(move |mut trigger: Trigger<KeyChordEvent>,
                         tapes: Res<Tapes>,
                         mut commands: Commands,
                         mut minibuffer: Minibuffer,
                         mut animate: ResMut<TapeAnimate>| {
                    match trigger.event_mut().take() {
                        Ok(chord) => {
                            *animate = TapeAnimate::Speed(1.0);
                            if append {
                                if let Some(tape) = tapes.get(&chord) {
                                    minibuffer.message(format!("Recording tape {}", &chord));
                                    *minibuffer.tape_recorder = TapeRecorder::Record { tape: tape.clone(), chord };
                                } else {
                                    minibuffer.message(format!("No prior tape. Recording new tape {}", &chord));
                                    *minibuffer.tape_recorder = TapeRecorder::Record { tape: Tape::default(), chord };
                                }
                            } else {
                                minibuffer.message(format!("Recording new tape {}", &chord));
                                *minibuffer.tape_recorder = TapeRecorder::Record { tape: Tape::default(), chord };
                            }
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
            minibuffer.message(format!("Stop recording tape {}", &chord));
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
    minibuffer.message("Play tape: ");
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
                 tapes: Res<Tapes>, mut minibuffer: Minibuffer, run_act_map: Res<RunActMap>, acts: Query<&Act>|
                 {
                     match trigger.event_mut().take() {
                         Ok(chord) => 'press: {
                             if let Some(tape) = tapes.get(&chord) {

                                 let script = match tape.script(&acts, &run_act_map) {
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
    if let TapeRecorder::Off { ref one_off } = *tape_recorder {
        let tape = one_off.clone();
        commands.queue(move |world: &mut World| {
            for _ in 0..count {
                if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                    warn!("Error playing tape: {e:?}");
                }
            }
        });
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

    pub fn ammend_input<I: Clone + 'static + Send + Sync + Debug>(&mut self, input: I) {
        if let Some(ref mut entry) = self.content.last_mut() {
            if entry.input.is_some() {
                warn!("Overwriting tape input for act {}", &entry.act.id);
            }
            entry.input = Some(Arc::new(input));
        } else {
            warn!("Cannot append input; no act has been run.");
        }
    }

    fn script(&self, acts: &Query<&Act>, run_act_map: &RunActMap) -> Result<String, fmt::Error> {
        let mut f = String::new();
        writeln!(f, "fn tape(mut commands: Commands) {{")?;
        for event in &self.content {
            let Ok(act) = acts.get(event.act.id) else { continue; };
            match event.input {
                Some(ref input) => {
                    let type_id = (**input).type_id();
                    // info!("try to get type_id out {type_id:?}");
                    let input_string: Cow<'static, str> = match run_act_map.get(&type_id) {
                        Some(run_act) => {
                            match run_act.debug_string(&**input) {
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
                    writeln!(f, "    commands.run_system_cached_with({}, {})", &act.system_name,
                                input_string)?;
                }
                None => {
                    writeln!(f, "    commands.run_system_cached({})", &act.system_name)?;
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
                     run_act_map: Res<crate::acts::RunActMap>) {
    let old = std::mem::replace(&mut *tape_recorder, TapeRecorder::Play);
    for e in &tape.content {
        let Ok(act) = acts.get(e.act.id) else {
            warn!("Could not get act for {:?}", e.act.id);
            continue;
        };
        let run_act = act.input.as_ref().and_then(|x| run_act_map.get(x).map(|y| &**y));
        crate::event::run_act_raw(e, Some(act), run_act, &mut next_prompt_state, &mut last_act, &mut commands, None);
    }
    let _ = std::mem::replace(&mut *tape_recorder, old);
}

#[cfg(test)]
mod test {
    use super::*;
    

    #[test]
    fn test_curve_api() {
        let curve = EasingCurve::new(0.0, 1.0, EaseFunction::BackInOut);
        assert_eq!(curve.sample(0.0), Some(0.0));
        assert_eq!(curve.sample(1.0), Some(1.0));
        assert_eq!(curve.sample(2.0), None);
        assert_eq!(curve.sample(-1.0), None);

        let curve = EasingCurve::new(0.0, 2.0, EaseFunction::BackInOut);
        assert_eq!(curve.sample(0.0), Some(0.0));
        assert_eq!(curve.sample(1.0), Some(2.0));
        assert_eq!(curve.sample(2.0), None);
        assert_eq!(curve.sample(-1.0), None);
    }

}
