//! Events
use bevy::ecs::{
    event::{Event, EventReader},
    system::Commands,
};
use std::fmt;

/// Request a one-shot system be run.
#[derive(Clone, Event, Debug)]
// pub struct RunActEvent(pub SystemId);
pub struct RunActEvent(pub super::act::Act);

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

/// Run act for any [RunActEvent].
pub fn run_acts(mut events: EventReader<RunActEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.run_system(e.0.system_id);
    }
}
