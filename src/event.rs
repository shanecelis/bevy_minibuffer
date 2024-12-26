//! Events
use crate::{
    Error,
    acts::{Act, ActRef, ActFlags, RunAct, ActSystem, RunActMap, tape::{TapeRecorder, Tape}},
    input::{KeyChord, Hotkey},
    prompt::PromptState,
    Minibuffer
};
use bevy::{
    ecs::{
        event::{Event, EventReader},
        system::{QueryLens, Commands},
    },
    prelude::*,
};
#[cfg(feature = "async")]
use bevy_channel_trigger::ChannelTriggerApp;
// #[cfg(feature = "async")]
// use bevy_crossbeam_event::CrossbeamEventApp;
use std::{borrow::Cow, fmt::{self, Debug}, any::Any, sync::Arc};

pub(crate) fn plugin(app: &mut App) {
    // #[cfg(feature = "async")]
    // app.add_crossbeam_event::<DispatchEvent>();
    #[cfg(feature = "async")]
    {
        let sender = app.add_channel_trigger::<DispatchEvent>();
        app.insert_resource(sender);
    }
    app.add_event::<DispatchEvent>()
        .add_event::<RunActEvent>()
        .add_event::<RunActByNameEvent>()
        // .add_systems(Startup, setup_observers)
        .init_resource::<LastRunAct>();
}

// fn setup_observers(root: Res<MinibufferRoot>,
//                    mut commands: Commands) {
//     commands.entity(root.0)
//         .with_children(|parent| {
//             parent.spawn(Observer::new(crate::event::dispatch_trigger));
//             parent.spawn(Observer::new(crate::event::run_acts_trigger));
//             parent.spawn(Observer::new(crate::event::run_acts_by_name_trigger));
//         });
// }

pub type Input = Arc<dyn Any + 'static + Send + Sync>;

/// Requests an act to be run
#[derive(Clone, Event, Debug)]
pub struct RunActEvent {
    /// The act to run
    pub(crate) act: ActRef,
    /// Which if any of its hotkeys started it
    pub hotkey: Option<usize>,
    pub(crate) input: Option<Input>,
    // pub(crate) universal_arg: Option<i32>,
}

/// Requests an act by name to be run
#[derive(Clone, Event, Debug)]
pub struct RunActByNameEvent {
    /// Name of the act to run
    pub name: Cow<'static, str>,
    input: Option<Input>,
}

impl RunActByNameEvent {
    /// Lookup and run act with given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into(), input: None}
    }

    pub fn new_with_input<I: 'static + Send + Sync + Debug>(name: impl Into<Cow<'static, str>>, input: I) -> Self {
        Self { name: name.into(), input: Some(Arc::new(input))}
    }

}

/// This holds the last act run.
///
/// It is set prior to the command being run, so a command can look up its own
/// run event and act using this resource.
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct LastRunAct(Option<RunActEvent>);

impl LastRunAct {
    /// Return the hotkey associated with this run.
    pub fn hotkey(&self, acts: &mut QueryLens<&Act>) -> Option<Hotkey> {
        self.0.as_ref().and_then(|run_act| run_act.hotkey(acts))
    }
}

impl RunActEvent {
    /// Make a new run act event.
    pub fn new(act: ActRef) -> Self {
        Self { act, hotkey: None, input: None}
    }

    pub fn from_act(act: &Act, id: Entity) -> Self {
        Self { act: ActRef { id, flags: act.flags.clone() }, hotkey: None, input: None}
    }

    pub fn new_with_input<I: 'static + Send + Sync + Debug>(act: ActRef, input: I) -> Self {
        Self { act, hotkey: None, input: Some(Arc::new(input))}
    }

    pub fn from_act_with_input<I: 'static + Send + Sync + Debug>(act: &Act, id: Entity, input: I) -> Self {
        Self { act: ActRef { id, flags: act.flags.clone() }, hotkey: None, input: Some(Arc::new(input))}
    }

    // pub fn from_name(name: impl Into<Cow<'static, str>>) -> Self {
    //     Self { act: ActArg::from(name.into()), hotkey: None }
    // }

    /// Set the hotkey index.
    pub fn with_hotkey(mut self, index: usize) -> Self {
        self.hotkey = Some(index);
        self
    }

    /// Return the hotkey associated with this run.
    pub fn hotkey(&self, acts: &mut QueryLens<&Act>) -> Option<Hotkey> {
        acts.query().get(self.act.id).ok().and_then(|act|
                                        self.hotkey.map(|index| act.hotkeys[index].clone()))
    }
}

// impl RunActEvent<ActArg> {
//     /// Make a new run act event.
//     pub fn from_arg(act: impl Into<ActArg>) -> Self {
//         Self { act: act.into(), hotkey: None }
//     }
// }

// impl fmt::Display for RunActEvent {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         // write!(f, "RunAct({})", self.0)
//         write!(f, "{}", self.act)
//     }
// }
// impl fmt::Debug for RunActEvent {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let rnd_state = bevy::utils::RandomState::with_seed(0);
//         let hash = rnd_state.hash_one(self.0);
//         write!(f, "StartActEvent({:04})", hash % 10000)
//     }
// }

/// Look up event fires when autocomplete panel is shown or hidden.
#[derive(Debug, Clone, Event)]
pub(crate) enum LookupEvent {
    /// Hide the autocomplete panel
    Hide,
    /// Show completions
    Completions(Vec<String>),
}

/// Dispatch an event
///
/// This event relays another event to fire.
///
/// Allows minibuffer to use one channel to dispatch multiple kinds of events.
#[doc(hidden)]
#[derive(Debug, Clone, Event)]
#[allow(private_interfaces)]
pub enum DispatchEvent {
    /// Send a look up event.
    LookupEvent(LookupEvent),
    /// Send a run act event.
    RunActEvent(RunActEvent),
    /// Send a lookup and run act event.
    RunActByNameEvent(RunActByNameEvent),
    /// Emit a message.
    EmitMessage(String),
    /// Clear the buffer.
    Clear,
    /// Show the buffer.
    SetVisible(bool),
    /// This event has been "taken" already.
    Taken,
}

impl From<LookupEvent> for DispatchEvent {
    fn from(e: LookupEvent) -> Self {
        Self::LookupEvent(e)
    }
}
impl From<RunActEvent> for DispatchEvent {
    fn from(e: RunActEvent) -> Self {
        Self::RunActEvent(e)
    }
}

pub(crate) fn dispatch_events(
    mut dispatch_events: EventReader<DispatchEvent>,
    mut lookup_events: EventWriter<LookupEvent>,
    mut minibuffer: Minibuffer,
    acts: Query<&Act>,
) {
    use crate::event::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookupEvent(l) => {
                lookup_events.send(l.clone());
            }
            RunActEvent(e) => {
                minibuffer.run_act(e.act);
            }
            RunActByNameEvent(e) => {
                minibuffer.run_act(e.clone().name);
            }
            EmitMessage(s) => {
                minibuffer.message(s.to_string());
            }
            Clear => {
                minibuffer.clear();
            }
            SetVisible(show) => {
                minibuffer.set_visible(*show);
            }
            Taken => {}
        }
    }
}

pub(crate) fn dispatch_trigger(
    mut dispatch_events: Trigger<DispatchEvent>,
    mut lookup_events: EventWriter<LookupEvent>,
    mut minibuffer: Minibuffer,
) {
    use crate::event::DispatchEvent::*;
    let event = std::mem::replace(dispatch_events.event_mut(), DispatchEvent::Taken);
    match event {
        LookupEvent(l) => {
            lookup_events.send(l);
        }
        RunActEvent(e) => {
            minibuffer.run_act(e.act);
        }

        RunActByNameEvent(e) => {
            minibuffer.run_act(e.name);
        }
        EmitMessage(s) => {
            minibuffer.message(s);
        }
        Clear => {
            minibuffer.clear();
        }
        SetVisible(show) => {
            minibuffer.set_visible(show);
        }
        Taken => {}
    }
}

#[derive(Event, Debug, Reflect)]
pub enum KeyChordEvent {
    Unhandled(KeyChord),
    Canceled,
    Handled,
}

impl KeyChordEvent {
    pub fn new(chord: KeyChord) -> Self {
        Self::Unhandled(chord)
    }

    pub fn take(&mut self) -> Result<KeyChord, Error> {
        match std::mem::replace(self, KeyChordEvent::Handled) {
            KeyChordEvent::Unhandled(chord) => Ok(chord),
            KeyChordEvent::Handled => Err(Error::Message("Event already handled".into())),
            KeyChordEvent::Canceled => Err(bevy_asky::Error::Cancel.into()),
        }
    }
}


pub fn run_act_raw(e: &RunActEvent,
                   act: Option<&Act>,
                   run_act: Option<&dyn RunAct>,
                   mut next_prompt_state: &mut NextState<PromptState>,
                   mut last_act: &mut LastRunAct,
                   commands: &mut Commands,
                   tape_recorder: Option<&mut TapeRecorder>,
) {
    if e.act.flags.contains(ActFlags::ShowMinibuffer) {
        next_prompt_state.set(PromptState::Visible);
    }
    let Some(act) = act else {
        warn!("Could not find act at {:?}", e.act.id);
        return;
    };
    last_act.0 = Some(e.clone());
    let run_act = run_act.unwrap_or(ActSystem);
    if let Some(ref input) = e.input {
        let input = input.clone();
        if let Err(error) = run_act.run_with_input(&*input, commands) {
            warn!("Error running act with input '{}': {:?}", act.name, error);
        }
    } else {
        if let Err(error) = run_act.run(commands) {
            warn!("Error running act '{}': {:?}", act.name, error);
        }
    }
    if let Some(tape_recorder) = tape_recorder {
        tape_recorder.process_event(e);
    }
    // } else {
    //     warn!("Could not find RunActMap for act '{}'.", act.name);
    // }
}

// fn act_and_runner(act_id: Entity, acts: &Query<&Act>, runners: &Query<&RunActMap>) -> Option<(&Act, &RunActMap)>{
//     acts.get(act_id).ok().and_then(|act| runners.get(act.system_id).ok().map(|run_act| (act, run_act)))
// }


/// Run act for any [RunActEvent].
pub(crate) fn run_acts(
    mut events: EventReader<RunActEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    mut tape: ResMut<TapeRecorder>,
    acts: Query<&Act>,
    run_act: Res<RunActMap>,
) {
    for e in events.read() {
        let act = acts.get(e.act.id).ok();
        let run_act = act.as_ref().and_then(|a|run_act.get(a.system_id).ok());
        run_act_raw(e, act, run_act, &mut next_prompt_state, &mut last_act, &mut commands, Some(&mut tape));
    }
}

/// Run act for any [RunActEvent].
pub(crate) fn run_acts_trigger(
    trigger: Trigger<RunActEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    run_act: Res<RunActMap>,
    acts: Query<&Act>,
    mut tape: ResMut<TapeRecorder>,
) {
    let e = trigger.event();
    let act = acts.get(e.act.id).ok();
    let run_act = act.as_ref().and_then(|a|run_act.get(a.system_id).ok());
    run_act_raw(e, act, run_act, &mut next_prompt_state, &mut last_act, &mut commands, Some(&mut tape));
}

/// Lookup and run act for any [RunActByNameEvent].
pub(crate) fn run_acts_by_name(
    mut events: EventReader<RunActByNameEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    run_act: Res<RunActMap>,
    acts: Query<(Entity, &Act)>,
    mut tape: ResMut<TapeRecorder>,
) {
    for e in events.read() {
        if let Some((id, act)) = acts.iter().find(|(_, a)| a.name == e.name) {
            let e = match &e.input {
                Some(input) => RunActEvent { act: ActRef::from_act(act, id), hotkey: None, input: Some(input.clone())},
                None => RunActEvent::from_act(act, id),
            };
            let run_act = run_act.get(act.system_id).ok();
            run_act_raw(&e, Some(act), run_act, &mut next_prompt_state, &mut last_act, &mut commands, Some(&mut tape));
        } else {
            warn!("No act named '{}' found.", e.name);
        }
    }
}

pub(crate) fn run_acts_by_name_trigger(
    trigger: Trigger<RunActByNameEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    run_act: Res<RunActMap>,
    acts: Query<(Entity, &Act)>,
    mut tape: ResMut<TapeRecorder>,
) {
    let e = trigger.event();
    if let Some((id, act)) = acts.iter().find(|(_, a)| a.name == e.name) {
        let e = match &e.input {
            Some(input) => RunActEvent { act: ActRef::from_act(act, id), hotkey: None, input: Some(input.clone())},
            None => RunActEvent::from_act(act, id),
        };
        let run_act = run_act.get(act.system_id).ok();
        run_act_raw(&e, Some(act), run_act, &mut next_prompt_state, &mut last_act, &mut commands, Some(&mut tape));
    } else {
        warn!("No act named '{}' found.", e.name);
    }
}


#[cfg(test)]
mod test {
    use std::{
        any::{Any, TypeId},
        sync::Arc,
    };

    #[test]
    fn test_arc_typeid() {
        let boxed: Arc<dyn Any> = Arc::new(2.0f32);

        let actual_id = (&*boxed).type_id();
        let boxed_id = boxed.type_id();

        assert_eq!(actual_id, TypeId::of::<f32>());
        assert_eq!(boxed_id, TypeId::of::<Arc<dyn Any>>());
        // assert_eq!(actual_id, boxed_id);
    }

    #[test]
    fn test_arc_downcast() {
        let boxed: Arc<dyn Any> = Arc::new(2.0f32);

        match boxed.downcast_ref::<f32>() {
            Some(value) => {
                assert_eq!(value, &2.0f32);
            }
            None => {
                panic!("Could not downcast.");
            }
        }
    }

}
