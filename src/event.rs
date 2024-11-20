//! Events
use crate::{Minibuffer, act::Act, act::ActFlags, prompt::PromptState};
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
use std::fmt;

pub(crate) fn plugin(app: &mut App) {
    // #[cfg(feature = "async")]
    // app.add_crossbeam_event::<DispatchEvent>();
    #[cfg(feature = "async")]
    {
        let sender = app.add_channel_trigger::<DispatchEvent>();
        app.insert_resource(sender);
    }
    // #[cfg(not(feature = "async"))]
    app.add_event::<DispatchEvent>();
    app.init_resource::<LastRunAct>();
}

/// Request a one-shot system be run.
#[derive(Clone, Event, Debug, Deref)]
// pub struct RunActEvent(pub SystemId);
pub struct RunActEvent {
    #[deref]
    pub act: Act,
    pub hotkey: Option<usize>
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub struct LastRunAct(Option<RunActEvent>);

impl RunActEvent {
    pub fn new(act: Act) -> Self {
        Self {
            act,
            hotkey: None
        }
    }

    pub fn hotkey(mut self, index: usize) -> Self {
        self.hotkey = Some(index);
        self
    }
}

/// Run the input sequence system even if the minibuffer is set to inactive.
#[derive(Clone, Event, Debug)]
pub struct RunInputSequenceEvent;

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
pub enum LookupEvent {
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
    LookupEvent(LookupEvent),
    /// Send a start act event.
    RunActEvent(RunActEvent),
    /// Emit a message.
    EmitMessage(String),
    /// Clear the buffer.
    Clear,
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
    mut look_up_events: EventWriter<LookupEvent>,
    mut request_act_events: EventWriter<RunActEvent>,
    mut minibuffer: Minibuffer,
) {
    use crate::event::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookupEvent(l) => {
                look_up_events.send(l.clone());
            }
            RunActEvent(s) => {
                request_act_events.send(s.clone());
            }
            EmitMessage(s) => {
                minibuffer.message(s);
            }
            Clear => {
                minibuffer.clear();
            }
        }
    }
}

/// Run act for any [RunActEvent].
pub fn run_acts(mut events: EventReader<RunActEvent>,
                mut next_prompt_state: ResMut<NextState<PromptState>>,
                mut commands: Commands,
                mut last_act: ResMut<LastRunAct>) {
    for e in events.read() {
        if e.act.flags.contains(ActFlags::Show) {
            next_prompt_state.set(PromptState::Visible);
        }
        last_act.0 = Some(e.clone());
        commands.run_system(e.act.system_id);
    }
}
