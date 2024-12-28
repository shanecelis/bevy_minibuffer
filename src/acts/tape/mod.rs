use crate::{
    acts::{universal::UniversalArg, Act, ActFlags, Acts, ActsPlugin, RunActMap},
    event::{KeyChordEvent, LastRunAct, RunActEvent},
    input::{keyseq, KeyChord},
    Minibuffer,
};
use bevy::prelude::*;
#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Debug, Write},
    sync::Arc,
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<TapeRecorder>()
        .init_resource::<RunActMap>()
        .init_state::<SoundState>()
        ;
#[cfg(feature = "fun")]
    app
        .add_plugins(fun::plugin);
}

pub struct TapeActs {
    acts: Acts,
}

impl Default for TapeActs {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                Act::new(record_tape)
                    .bind(keyseq! { Q })
                    .sub_flags(ActFlags::Record),
                Act::new_with_input(play_tape)
                    .bind_aliased(keyseq! { Shift-2 }, "@")
                    .sub_flags(ActFlags::Record),
                // Act::new(repeat).bind(keyseq! { Period }).sub_flags(ActFlags::Record | ActFlags::RunAct),
                Act::new(repeat)
                    .bind(keyseq! { Period })
                    .sub_flags(ActFlags::Record),
                &mut Act::new(copy_tape),
            ]),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum SoundState {
    #[default]
    Off,
    Stop,
    Record,
    Play,
    Rewind,
    Load,
    Squeak,
    // Unload,
}

#[cfg(feature = "fun")]
mod fun {
    use super::*;
    use bevy::asset::embedded_asset;
    use crate::{
        ui::IconContainer,
        prompt::show,
    };
    use std::time::Duration;

    pub(super) fn plugin(app: &mut App) {
        embedded_asset!(app, "tape.png");
        embedded_asset!(app, "record-start.ogg");
        embedded_asset!(app, "record-loop.ogg");
        embedded_asset!(app, "record-stop.ogg");
        embedded_asset!(app, "play-loop.ogg");
        embedded_asset!(app, "tape-rewind.ogg");
        embedded_asset!(app, "tape-load.ogg");
        embedded_asset!(app, "squeak.ogg");
        app.init_resource::<TapeSoundSource>()
            .init_resource::<TapeAnimate>()
            .add_systems(Startup, setup_icon)
            .add_systems(Update, (after, play_for))
            .add_systems(OnEnter(SoundState::Record), record)
            .add_systems(OnEnter(SoundState::Stop), (stop_all_sound, stop).chain())
            .add_systems(OnEnter(SoundState::Load), (load, show::<TapeIcon>))
            .add_systems(OnEnter(SoundState::Play), (stop_all_sound, play).chain())
            .add_systems(OnEnter(SoundState::Rewind), rewind)
            .add_systems(OnEnter(SoundState::Squeak), squeak)
            .add_systems(Update, animate_icon);
            ;
    }


    #[derive(Resource, Default)]
    enum TapeAnimate {
        Speed(f32),
        Curve {
            curve: Box<dyn Curve<f32> + 'static + Send + Sync>,
            pos: f32,
        }, // then: Option<Box<TapeAnimate>> },
        #[default]
        Hide,
    }

    impl TapeAnimate {
        fn curve(curve: impl Curve<f32> + Send + Sync + 'static) -> Self {
            Self::Curve {
                curve: Box::new(curve),
                pos: 0.0,
            }
        }
    }

    fn squeak(mut animate: ResMut<TapeAnimate>, mut commands: Commands, tape_sound: Res<TapeSoundSource>) {
        *animate = TapeAnimate::curve(easing(-3.0, 0.5, EaseFunction::Steps(3)).unwrap()); //.map(|x| -x));
        commands.spawn((AudioPlayer::new(tape_sound.squeak.clone_weak()),
                        PlaybackSettings::DESPAWN,
                        TapeSoundSink));
    }

    fn load(mut animate: ResMut<TapeAnimate>, mut commands: Commands, tape_sound: Res<TapeSoundSource>) {
        *animate = TapeAnimate::curve(easing(-3.0, 4.0, EaseFunction::Steps(3)).unwrap()); //.map(|x| -x));
        commands.spawn((AudioPlayer::new(tape_sound.load.clone_weak()),
                        PlaybackSettings::DESPAWN,
                        TapeSoundSink));
    }

    fn rewind(mut animate: ResMut<TapeAnimate>, mut commands: Commands, tape_sound: Res<TapeSoundSource>) {
        commands.spawn((AudioPlayer::new(tape_sound.rewind.clone_weak()),
                        TapeSoundSink,
                        PlaybackSettings::LOOP,
                        PlayFor(Timer::new(Duration::from_secs_f32(2.0), TimerMode::Once), After::State(SoundState::Stop))));
        *animate = TapeAnimate::Speed(-4.0);
    }

    fn play(mut commands: Commands, tape_sound: Res<TapeSoundSource>, mut animate: ResMut<TapeAnimate>) {
        commands.spawn((AudioPlayer::new(tape_sound.play.clone_weak()),
                        TapeSoundSink,
                        PlayFor(Timer::new(Duration::from_secs_f32(2.0), TimerMode::Once), After::State(SoundState::Stop)),
                        // After::State(SoundState::Stop),
                        PlaybackSettings::LOOP));

        *animate = TapeAnimate::Speed(1.0);
    }

    fn record(mut commands: Commands, tape_sound: Res<TapeSoundSource>, mut animate: ResMut<TapeAnimate>) {
        commands.spawn((AudioPlayer::new(tape_sound.record_start.clone_weak()),
                        After::Play(AudioBundle {
                            source: AudioPlayer::new(tape_sound.record_loop.clone_weak()),
                            settings: PlaybackSettings::LOOP,
                        }),
                        TapeSoundSink,
                        PlaybackSettings::ONCE));

        *animate = TapeAnimate::Speed(1.0);
    }

    fn stop_all_sound(query: Query<Entity, With<TapeSoundSink>>, mut commands: Commands) {
        for id in &query {
            commands.entity(id).despawn();
        }
    }

    fn stop(mut commands: Commands, tape_sound: Res<TapeSoundSource>, mut animate: ResMut<TapeAnimate>) {
        commands.spawn((AudioPlayer::new(tape_sound.record_stop.clone_weak()),
                        TapeSoundSink,
                        PlaybackSettings::DESPAWN));

        *animate = TapeAnimate::Speed(0.0);
    }


    #[derive(Component)]
    struct TapeSoundSink;

    #[derive(Component, Default)]
    enum After {
        #[default]
        Done,
        State(SoundState),
        Play(AudioBundle)
    }

    #[derive(Component)]
    struct PlayFor(Timer, After);

    #[derive(Component)]
    struct PlayNext(Option<AudioBundle>);
    impl PlayNext {
        fn new(bundle: AudioBundle) -> Self {
            Self(Some(bundle))
        }
    }

    fn play_for(mut query: Query<(Entity, &AudioSink, &mut PlayFor), With<TapeSoundSink>>, mut commands: Commands, time: Res<Time>,
                 mut tape_state: ResMut<NextState<SoundState>>) {
        for (id, sink, mut play_for) in &mut query {
            let PlayFor(ref mut timer, ref mut after) = *play_for;
            timer.tick(time.delta());
            if timer.just_finished() {
                match std::mem::take(after) {
                    After::Done => {
                        warn!("After::DONE unexpected.");
                    },
                    After::Play(bundle) => {
                        commands.spawn((bundle,
                                        TapeSoundSink));
                        commands.entity(id).despawn();
                    }
                    After::State(state) => {
                        tape_state.set(state);
                        commands.entity(id).despawn();
                    }
                }
            }
        }
    }

    fn after(mut query: Query<(Entity, &AudioSink, &mut After), With<TapeSoundSink>>, mut commands: Commands, time: Res<Time>,
                 mut tape_state: ResMut<NextState<SoundState>>) {
        for (id, sink, mut after) in &mut query {
            if sink.empty() {
                match std::mem::take(&mut *after) {
                    After::Done => {
                        warn!("After::DONE unexpected.");
                    },
                    After::Play(bundle) => {
                        commands.spawn((bundle,
                                        TapeSoundSink));
                        commands.entity(id).despawn();
                    }
                    After::State(state) => {
                        tape_state.set(state);
                        commands.entity(id).despawn();
                    }
                }
            }
        }
    }

fn animate_icon(
    mut query: Query<&mut Transform, With<TapeIcon>>,
    mut animate: ResMut<TapeAnimate>,
    last_speed: Local<Option<f32>>,
    time: Res<Time>,
) {
    let speed = match *animate {
        TapeAnimate::Speed(speed) => Some(speed),
        TapeAnimate::Hide => None,
        TapeAnimate::Curve {
            ref curve,
            ref mut pos,
        } => {
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

    // fn react_on_removal(mut removed: RemovedComponents<PlayNext>, mut commands: Commands) {
    //     for after in removed.read() {
    //         if let Some(bundle) = after.0.take() {
    //             commands.spawn(bundle);
    //         }
    //     }
    // }

    #[derive(Resource)]
    pub(super) struct TapeSoundSource {
        pub(super) record_start: Handle<AudioSource>,
        pub(super) record_loop: Handle<AudioSource>,
        pub(super) record_stop: Handle<AudioSource>,
        pub(super) play: Handle<AudioSource>,
        pub(super) rewind: Handle<AudioSource>,
        pub(super) load: Handle<AudioSource>,
        pub(super) squeak: Handle<AudioSource>,
    }

    impl FromWorld for TapeSoundSource {
        fn from_world(world: &mut World) -> Self {
            let asset_server = world.resource::<AssetServer>();
            Self {
                record_start: asset_server.load("embedded://bevy_minibuffer/acts/tape/record-start.ogg"),
                record_loop: asset_server.load("embedded://bevy_minibuffer/acts/tape/record-loop.ogg"),
                record_stop: asset_server.load("embedded://bevy_minibuffer/acts/tape/record-stop.ogg"),
                play: asset_server.load("embedded://bevy_minibuffer/acts/tape/play-loop.ogg"),
                rewind: asset_server.load("embedded://bevy_minibuffer/acts/tape/tape-rewind.ogg"),
                load: asset_server.load("embedded://bevy_minibuffer/acts/tape/tape-load.ogg"),
                squeak: asset_server.load("embedded://bevy_minibuffer/acts/tape/squeak.ogg"),
            }
        }
    }

#[derive(Component)]
struct TapeIcon;

const PADDING: Val = Val::Px(5.0);

fn setup_icon(
    mut commands: Commands,
    query: Query<Entity, With<IconContainer>>,
    asset_loader: Res<AssetServer>,
) {
    let icon_container = query.single();
    commands.entity(icon_container).with_children(|parent| {
        parent.spawn((
            ImageNode::new(asset_loader.load("embedded://bevy_minibuffer/acts/tape/tape.png")),
            Visibility::Hidden,
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
            TapeIcon,
        ));
    });
}

fn easing(
    amp: f32,
    duration: f32,
    func: EaseFunction,
) -> Option<LinearReparamCurve<f32, EasingCurve<f32>>> {
    let domain = Interval::new(0.0, duration).ok()?;
    EasingCurve::new(0.0, amp, func)
        .reparametrize_linear(domain)
        .ok()
}


}

impl Plugin for TapeActs {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tapes>()
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

#[derive(Resource, Debug)]
pub enum TapeRecorder {
    /// Record only the last act that was run.
    Off {
        one_off: Tape,
    },
    /// A Tape is being recorded.
    Record {
        tape: Tape,
        chord: KeyChord,
    },
    Play,
}

impl Default for TapeRecorder {
    fn default() -> Self {
        TapeRecorder::Off {
            one_off: Tape::default(),
        }
    }
}

impl TapeRecorder {
    pub fn process_event(&mut self, event: &RunActEvent) {
        if event.act.flags.contains(ActFlags::Record) {
            match self {
                TapeRecorder::Off {
                    one_off: ref mut tape,
                } => {
                    tape.content.clear();
                    tape.append_run(event.clone());
                }
                TapeRecorder::Record { ref mut tape, .. } => {
                    tape.append_run(event.clone());
                }
                _ => (),
            }
        }
    }

    pub fn process_input<I: Debug + Clone + Send + Sync + 'static>(&mut self, input: &I) {
        match self {
            TapeRecorder::Record { ref mut tape, .. }
            | TapeRecorder::Off {
                one_off: ref mut tape,
            } => {
                tape.ammend_input(input.clone());
            }
            _ => (),
        }
    }
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct Tapes(HashMap<KeyChord, Tape>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LastPlayed(Option<KeyChord>);

fn record_tape(
    mut minibuffer: Minibuffer,
    mut tapes: ResMut<Tapes>,
    universal: Res<UniversalArg>,
    mut next_tape_state: ResMut<NextState<SoundState>>,
) {
    next_tape_state.set(SoundState::Load);
    let append = universal.is_some();
    match &*minibuffer.tape_recorder {
        TapeRecorder::Off { one_off: _ } => {
            minibuffer.message("Record tape: ");
            minibuffer.get_chord().observe(
                move |mut trigger: Trigger<KeyChordEvent>,
                      tapes: Res<Tapes>,
                      mut commands: Commands,
                      mut minibuffer: Minibuffer,
                mut tape_state: ResMut<NextState<SoundState>>| {
                    match trigger.event_mut().take() {
                        Ok(chord) => {
                            tape_state.set(SoundState::Record);
                            if append {
                                if let Some(tape) = tapes.get(&chord) {
                                    minibuffer.message(format!("Recording tape {}", &chord));
                                    *minibuffer.tape_recorder = TapeRecorder::Record {
                                        tape: tape.clone(),
                                        chord,
                                    };
                                } else {
                                    minibuffer.message(format!(
                                        "No prior tape. Recording new tape {}",
                                        &chord
                                    ));
                                    *minibuffer.tape_recorder = TapeRecorder::Record {
                                        tape: Tape::default(),
                                        chord,
                                    };
                                }
                            } else {
                                minibuffer.message(format!("Recording new tape {}", &chord));
                                *minibuffer.tape_recorder = TapeRecorder::Record {
                                    tape: Tape::default(),
                                    chord,
                                };
                            }
                        }
                        Err(e) => {
                            tape_state.set(SoundState::Squeak);
                            minibuffer.message(format!("{e}"));
                        }
                    }
                    commands.entity(trigger.entity()).despawn_recursive();
                },
            );
        }
        TapeRecorder::Record { .. } => {
            let TapeRecorder::Record { tape, chord } =
                std::mem::take(&mut *minibuffer.tape_recorder)
            else {
                unreachable!();
            };
            next_tape_state.set(SoundState::Stop);
            minibuffer.message(format!("Stop recording tape {}", &chord));
            tapes.insert(chord, tape);
        }
        TapeRecorder::Play => {
            warn!("Got record tape during Play.");
        }
    }
}

fn play_tape(In(chord): In<Option<KeyChord>>,
    mut minibuffer: Minibuffer,
    mut acts: Query<&Act>,
    last_act: Res<LastRunAct>,
    universal_arg: Res<UniversalArg>,
             tapes: Res<Tapes>,
    mut next_tape_state: ResMut<NextState<SoundState>>,
    tape_state: Res<State<SoundState>>,
             mut commands: Commands,
) {
    // Non-interactive case
    if let Some(chord) = chord {
        if let Some(tape) = tapes.get(&chord) {
            let tape = tape.clone();
            let count = 1; // TODO: Need to store universal somewhere.
            // next_tape_state.set(SoundState::Play);
            commands.queue(move |world: &mut World| {
                for _ in 0..count {
                    if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                        warn!("Error playing tape: {e:?}");
                    }
                }
            });
        } else {
            // next_tape_state.set(SoundState::Load);
            minibuffer.message(format!("No tape for {}", &chord));
        }
        return;
    }
    // Interactive case
    if *tape_state.get() != SoundState::Play {
        next_tape_state.set(SoundState::Load);
    }
    let this_keychord = last_act.hotkey(&mut acts.as_query_lens());
    let count = universal_arg.unwrap_or(1);
    minibuffer.message("Play tape: ");
    minibuffer.get_chord().observe(
        move |mut trigger: Trigger<KeyChordEvent>,
              mut commands: Commands,
              tapes: Res<Tapes>,
              mut minibuffer: Minibuffer,
        mut tape_state: ResMut<NextState<SoundState>>,
              mut last_played: ResMut<LastPlayed>| {
            match trigger.event_mut().take() {
                Ok(mut chord) => 'body: {
                    if this_keychord
                        .as_ref()
                        .map(|x| x.chords.len() == 1 && x.chords[0] == chord)
                        .unwrap_or(false)
                    {
                        // We want to play the same chord as last time.
                        if let Some(ref last_played) = **last_played {
                            chord.clone_from(last_played);
                        } else {
                            minibuffer
                                .message("Tried to play last tape but no tape has been played.");
                            break 'body;
                        }
                    }
                    minibuffer.log_input(&Some(chord.clone()));
                    if let Some(tape) = tapes.get(&chord) {
                        let tape = tape.clone();
                        tape_state.set(SoundState::Play);
                        commands.queue(move |world: &mut World| {
                            for _ in 0..count {
                                if let Err(e) = world.run_system_cached_with(play_tape_sys, &tape) {
                                    warn!("Error playing tape: {e:?}");
                                }
                            }
                        });
                        **last_played = Some(chord);
                    } else {
                        tape_state.set(SoundState::Load);
                        minibuffer.message(format!("No tape for {}", &chord));
                    }
                }
                Err(e) => {
                    minibuffer.message(format!("{e}"));
                    tape_state.set(SoundState::Squeak);
                }
            }
            commands.entity(trigger.entity()).despawn_recursive();
        },
    );
}

fn copy_tape(mut minibuffer: Minibuffer,
    mut tape_state: ResMut<NextState<SoundState>>) {
    tape_state.set(SoundState::Load);
    minibuffer.message("Copy tape for key: ");
    minibuffer.get_chord().observe(
        |mut trigger: Trigger<KeyChordEvent>,
         mut commands: Commands,
         tapes: Res<Tapes>,
         mut minibuffer: Minibuffer,
         run_act_map: Res<RunActMap>,
        mut tape_state: ResMut<NextState<SoundState>>,
         acts: Query<&Act>| {
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
                        tape_state.set(SoundState::Rewind);
                        info!("{}", script);
                        #[cfg(feature = "clipboard")]
                        {
                            match ClipboardContext::new() {
                                Ok(mut ctx) => {
                                    if let Err(e) = ctx.set_contents(script.clone()) {
                                        warn!("Could not set clipboard: {e}");
                                    }
                                    minibuffer.message(format!(
                                        "Copy tape {} to clipboard and log:\n\n{}.",
                                        &chord, &script
                                    ));
                                }
                                Err(e) => {
                                    minibuffer
                                        .message(format!("Log tape {}:\n\n{}", &chord, &script));
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
                    tape_state.set(SoundState::Squeak);
                    minibuffer.message(format!("{e}"));
                }
            }
            commands.entity(trigger.entity()).despawn_recursive();
        },
    );
}

fn repeat(
    tape_recorder: Res<TapeRecorder>,
    universal_arg: Res<UniversalArg>,
    mut commands: Commands,
) {
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
            let Ok(act) = acts.get(event.act.id) else {
                continue;
            };
            match event.input {
                Some(ref input) => {
                    let type_id = (**input).type_id();
                    // info!("try to get type_id out {type_id:?}");
                    let input_string: Cow<'static, str> = match run_act_map.get(&type_id) {
                        Some(run_act) => match run_act.debug_string(&**input) {
                            Some(s) => s.into(),
                            None => {
                                warn!("Debug string function failed for act {:?}", &act.name);
                                "???".into()
                            }
                        },
                        None => {
                            warn!("No debug string function for act {:?}", &act.name);
                            "!!!".into()
                        }
                    };
                    writeln!(
                        f,
                        "    commands.run_system_cached_with({}, {})",
                        &act.system_name, input_string
                    )?;
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

pub fn play_tape_sys(
    InRef(tape): InRef<Tape>,
    mut next_prompt_state: ResMut<NextState<crate::prompt::PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<crate::event::LastRunAct>,
    mut tape_recorder: ResMut<TapeRecorder>,
    acts: Query<&Act>,
    run_act_map: Res<crate::acts::RunActMap>,
) {
    let old = std::mem::replace(&mut *tape_recorder, TapeRecorder::Play);
    for e in &tape.content {
        let Ok(act) = acts.get(e.act.id) else {
            warn!("Could not get act for {:?}", e.act.id);
            continue;
        };
        let run_act = act
            .input
            .as_ref()
            .and_then(|x| run_act_map.get(x).map(|y| &**y));
        crate::event::run_act_raw(
            e,
            Some(act),
            run_act,
            &mut next_prompt_state,
            &mut last_act,
            &mut commands,
            None,
        );
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
