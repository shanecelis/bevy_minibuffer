//! Events
use crate::{
    acts::{Act, ActFlags, ActRef, ActSystem, RunActMap},
    input::{Hotkey, KeyChord},
    prompt::PromptState,
    ui::MinibufferNode,
    Error, Minibuffer,
};
use bevy::{
    core::FrameCount,
    ecs::{
        event::{Event, EventReader},
        system::{Commands, QueryLens},
    },
    prelude::*,
};

#[cfg(feature = "async")]
use bevy_channel_trigger::ChannelTriggerApp;
// #[cfg(feature = "async")]
// use bevy_crossbeam_event::CrossbeamEventApp;
use std::{borrow::Cow, fmt::Debug};

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
        .add_systems(Startup, setup_observers)
        .init_resource::<LastRunAct>();
}

fn setup_observers(query: Query<Entity, With<MinibufferNode>>, mut commands: Commands) {
    match query.get_single() {
        Ok(root) => {
            commands.entity(root).with_children(|parent| {
                parent.spawn(Observer::new(dispatch_trigger));
                parent.spawn(Observer::new(run_acts_obs));
                parent.spawn(Observer::new(run_acts_by_name_obs));
                parent.spawn(Observer::new(set_visible_on_flag));
                parent.spawn(Observer::new(crate::acts::tape::process_event));
            });
        }
        Err(e) => {
            error!("Can not setup minibuffer observers: {e}");
        }
    }
}

/// Requests an act to be run
#[derive(Clone, Event, Debug, Copy)]
pub struct RunActEvent {
    /// The act to run
    pub(crate) act: ActRef,
    /// Which if any of its hotkeys started it
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
        Self {
            name: name.into(),
            // input: None,
        }
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

fn set_visible_on_flag(
    trigger: Trigger<RunActEvent>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
) {
    if trigger.event().act.flags.contains(ActFlags::ShowMinibuffer) {
        next_prompt_state.set(PromptState::Visible);
    }
}

impl RunActEvent {
    /// Make a new run act event.
    pub fn new(act: ActRef) -> Self {
        Self {
            act,
            hotkey: None,
            // input: None,
        }
    }

    pub fn from_act(act: &Act, id: Entity) -> Self {
        Self {
            act: ActRef {
                id,
                flags: act.flags,
            },
            hotkey: None,
            // input: None,
        }
    }

    /// Set the hotkey index.
    pub fn with_hotkey(mut self, index: usize) -> Self {
        self.hotkey = Some(index);
        self
    }

    /// Return the hotkey associated with this run.
    pub fn hotkey(&self, acts: &mut QueryLens<&Act>) -> Option<Hotkey> {
        acts.query()
            .get(self.act.id)
            .ok()
            .and_then(|act| self.hotkey.map(|index| act.hotkeys[index].clone()))
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

fn dispatch_trigger(
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

/// Run act for any [RunActEvent].
pub(crate) fn run_acts(mut events: EventReader<RunActEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.trigger(*e);
    }
}

/// Run act for any [RunActEvent].
fn run_acts_obs(
    trigger: Trigger<RunActEvent>,
    mut commands: Commands,
    run_act_map: Res<RunActMap>,
    acts: Query<&Act>,
    mut last: ResMut<LastRunAct>,
    frame_count: Res<FrameCount>,
) {
    let e = trigger.event();
    let act = match acts.get(e.act.id) {
        Ok(act) => act,
        Err(e) => {
            warn!("Could not find act: {e}");
            return;
        }
    };
    trace!("act {:?} frame {}", &act, frame_count.0);
    let run_act = act
        .input
        .as_ref()
        .and_then(|x| run_act_map.get(x).map(|y| &**y));

    let run_act = run_act.unwrap_or(&ActSystem);
    last.0 = Some(*trigger.event());
    if let Err(error) = run_act.run(act.system_id, &mut commands) {
        warn!("Error running act '{}': {:?}", act.name, error);
    }
}

/// Lookup and run act for any [RunActByNameEvent].
pub(crate) fn run_acts_by_name(mut events: EventReader<RunActByNameEvent>, mut commands: Commands) {
    for e in events.read() {
        commands.trigger(e.clone());
    }
}

fn run_acts_by_name_obs(
    trigger: Trigger<RunActByNameEvent>,
    mut commands: Commands,
    acts: Query<(Entity, &Act)>,
) {
    let e = trigger.event();
    if let Some((id, act)) = acts.iter().find(|(_, a)| a.name == e.name) {
        let new_event = RunActEvent {
            act: ActRef::from_act(act, id),
            hotkey: None,
        };
        commands.trigger(new_event);
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

        let actual_id = (*boxed).type_id();
        let boxed_id = boxed.type_id();

        assert_eq!(actual_id, TypeId::of::<f32>());
        assert_eq!(boxed_id, TypeId::of::<Arc<dyn Any>>());
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
