use std::fmt;
use bevy::ecs::event::Event;
use bevy::ecs::system::SystemId;


#[derive(Clone, Event)]
pub struct StartActEvent(pub SystemId);

impl fmt::Debug for StartActEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rnd_state = bevy::utils::RandomState::with_seed(0);
        let hash = rnd_state.hash_one(self.0);
        write!(f, "StartActEvent({:04})", hash % 10000)
    }
}


#[derive(Debug, Clone, Event)]
pub enum LookUpEvent {
    Hide,
    Completions(Vec<String>),
}

#[derive(Debug, Clone, Event)]
pub enum DispatchEvent {
    LookUpEvent(LookUpEvent),
    StartActEvent(StartActEvent),
}

impl From<LookUpEvent> for DispatchEvent {
    fn from(e: LookUpEvent) -> Self {
        Self::LookUpEvent(e)
    }
}
impl From<StartActEvent> for DispatchEvent {
    fn from(e: StartActEvent) -> Self {
        Self::StartActEvent(e)
    }
}
