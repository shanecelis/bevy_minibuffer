//! Events
use bevy::{
    prelude::*,
    ecs::{
        event::{Event, EventReader},
        system::Commands,
    }
};
use crate::Minibuffer;
use std::fmt;
#[cfg(feature = "async")]
use bevy_crossbeam_event::CrossbeamEventApp;

pub(crate) fn plugin(app: &mut App) {
#[cfg(feature = "async")]
    app
        .add_crossbeam_event::<DispatchEvent>();
#[cfg(not(feature = "async"))]
    app
        .add_event::<DispatchEvent>();
}

/// Request a one-shot system be run.
#[derive(Clone, Event, Debug, Deref)]
// pub struct RunActEvent(pub SystemId);
pub struct RunActEvent(pub super::act::Act);

/// Run the input sequence system even if the minibuffer is set to inactive.
#[derive(Clone, Event, Debug)]
pub struct RunInputSequenceEvent;

impl fmt::Display for RunActEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write!(f, "RunAct({})", self.0)
        write!(f, "{}", self.0)
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
pub enum LookUpEvent {
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
#[derive(Debug, Clone, Event)]
pub enum DispatchEvent {
    /// Send a look up event.
    LookUpEvent(LookUpEvent),
    /// Send a start act event.
    RunActEvent(RunActEvent),
    /// Emit a message.
    EmitMessage(String),
}

impl From<LookUpEvent> for DispatchEvent {
    fn from(e: LookUpEvent) -> Self {
        Self::LookUpEvent(e)
    }
}
impl From<RunActEvent> for DispatchEvent {
    fn from(e: RunActEvent) -> Self {
        Self::RunActEvent(e)
    }
}

pub(crate) fn dispatch_events(
    mut dispatch_events: EventReader<DispatchEvent>,
    mut look_up_events: EventWriter<LookUpEvent>,
    mut request_act_events: EventWriter<RunActEvent>,
    mut minibuffer: Minibuffer,
) {
    use crate::event::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookUpEvent(l) => {
                look_up_events.send(l.clone());
            }
            RunActEvent(s) => {
                request_act_events.send(s.clone());
            }
            EmitMessage(s) => {
                minibuffer.message(s);
            }
        }
    }
}

/// Run act for any [RunActEvent].
pub fn run_acts(mut events: EventReader<RunActEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.run_system(e.0.system_id);
    }
}
