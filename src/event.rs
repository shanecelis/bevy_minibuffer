//! Events
use crate::{acts::Act, acts::ActFlags, input::Hotkey, prompt::PromptState, Minibuffer};
use bevy::{
    ecs::{
        event::{Event, EventReader},
        system::Commands,
    },
    prelude::*,
};
#[cfg(feature = "async")]
use bevy_channel_trigger::ChannelTriggerApp;
// #[cfg(feature = "async")]
// use bevy_crossbeam_event::CrossbeamEventApp;
use std::{borrow::Cow, fmt};

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

/// Requests an act to be run
#[derive(Clone, Event, Debug, Deref)]
pub struct RunActEvent {
    #[deref]
    /// The act to run
    pub act: Act,
    /// Which one if any of its hotkeys started it
    pub hotkey: Option<usize>,
}

/// Requests an act by name to be run
#[derive(Clone, Event, Debug)]
pub struct RunActByNameEvent {
    /// Name of the act to run
    pub name: Cow<'static, str>,
}

impl RunActByNameEvent {
    /// Lookup and run act with given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
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
    pub fn hotkey(&self) -> Option<&Hotkey> {
        self.0.as_ref().and_then(|run_act| run_act.hotkey())
    }
}

impl RunActEvent {
    /// Make a new run act event.
    pub fn new(act: Act) -> Self {
        Self { act, hotkey: None }
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
    pub fn hotkey(&self) -> Option<&Hotkey> {
        self.hotkey.map(|index| &self.act.hotkeys[index])
    }
}

// impl RunActEvent<ActArg> {
//     /// Make a new run act event.
//     pub fn from_arg(act: impl Into<ActArg>) -> Self {
//         Self { act: act.into(), hotkey: None }
//     }
// }

impl fmt::Display for RunActEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write!(f, "RunAct({})", self.0)
        write!(f, "{}", self.act)
    }
}
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

// #[derive(Debug, Clone)]
// pub enum MsgDest<T> {
//     Replace(T),
//     Append(T)
// }

// impl<T> MsgDest<T> {
//     pub fn map<F: Fn(T) -> X, X>(self, f: F) -> MsgDest<X> {
//         use MsgDest::*;
//         match self {
//             Replace(x) => Replace(f(x)),
//             Append(x) => Append(f(x)),
//         }
//     }
// }

// impl<T> From<T> for MsgDest<T> {
//     fn from(x: T) -> Self {
//         MsgDest::Replace(x)
//     }
// }

// impl<T, X:Into<T>> From<X> for MsgDest<T> {
//     fn from(x: X) -> Self {
//         MsgDest::Replace(x.into())
//     }
// }
//

// impl<X: Into<String>> From<X> for MsgDest<String> {
//     fn from(x: X) -> Self {
//         MsgDest::Replace(x.into())
//     }
// }

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
) {
    use crate::event::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookupEvent(l) => {
                lookup_events.send(l.clone());
            }
            RunActEvent(e) => {
                minibuffer.run_act(e.clone().act);
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

/// Run act for any [RunActEvent].
pub(crate) fn run_acts(
    mut events: EventReader<RunActEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
) {
    for e in events.read() {
        if e.act.flags.contains(ActFlags::ShowMinibuffer) {
            next_prompt_state.set(PromptState::Visible);
        }
        last_act.0 = Some(e.clone());
        commands.run_system(e.act.system_id);
    }
}

/// Run act for any [RunActEvent].
pub(crate) fn run_acts_trigger(
    trigger: Trigger<RunActEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
) {
    let e = trigger.event();
    if e.act.flags.contains(ActFlags::ShowMinibuffer) {
        next_prompt_state.set(PromptState::Visible);
    }
    last_act.0 = Some(e.clone());
    commands.run_system(e.act.system_id);
}

/// Lookup and run act for any [RunActByNameEvent].
pub(crate) fn run_acts_by_name(
    mut events: EventReader<RunActByNameEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    acts: Query<&Act>,
) {
    for e in events.read() {
        if let Some(act) = acts.iter().find(|a| a.name == e.name) {
            if act.flags.contains(ActFlags::ShowMinibuffer) {
                next_prompt_state.set(PromptState::Visible);
            }
            let system_id = act.system_id;
            last_act.0 = Some(RunActEvent::new(act.clone()));
            commands.run_system(system_id);
        }
    }
}

pub(crate) fn run_acts_by_name_trigger(
    trigger: Trigger<RunActByNameEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut commands: Commands,
    mut last_act: ResMut<LastRunAct>,
    acts: Query<&Act>,
) {
    let e = trigger.event();
    if let Some(act) = acts.iter().find(|a| a.name == e.name) {
        if act.flags.contains(ActFlags::ShowMinibuffer) {
            next_prompt_state.set(PromptState::Visible);
        }
        let system_id = act.system_id;
        last_act.0 = Some(RunActEvent::new(act.clone()));
        commands.run_system(system_id);
    }
}
