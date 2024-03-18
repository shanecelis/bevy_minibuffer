use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use bevy::ecs::component::Tick;
use bevy::ecs::system::{SystemMeta, SystemParam};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;

// Try not to rely on these things.
use crate::prompt::*;

#[derive(Debug, PartialEq)]
pub enum ProcState {
    Uninit,
    Active,
    // Warm,
}

#[derive(Debug)]
pub(crate) enum ProcContent {
    Prompt(ReadPrompt),
    Message(CowStr),
}

#[derive(Debug)]
pub struct Proc(pub(crate) ProcContent, pub(crate) ProcState);

impl From<ReadPrompt> for Proc {
    fn from(prompt: ReadPrompt) -> Self {
        Self(ProcContent::Prompt(prompt), ProcState::Uninit)
    }
}

impl<T: Into<CowStr>> From<T> for Proc {
    fn from(msg: T) -> Self {
        Self(ProcContent::Message(msg.into()), ProcState::Uninit)
    }
}


#[derive(Debug)]
pub struct ConsoleState {
    pub(crate) asleep: Vec<Proc>,
    pub(crate) unprocessed: Vec<Proc>,
}

impl ConsoleState {
    fn new() -> Self {
        ConsoleState {
            asleep: Vec::new(),
            unprocessed: Vec::new(),
        }
    }

    pub fn push(&mut self, proc: Proc) {
        self.unprocessed.push(proc);
    }
}

pub struct Prompt {
    pub buf: PromptBuf,
    pub config: ConsoleConfig,
}

unsafe impl SystemParam for Prompt {
    type State = ConsoleConfig;
    type Item<'w, 's> = Prompt;

    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        world.get_resource_mut::<ConsoleConfig>().unwrap().clone()
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Prompt::new(state.clone())
    }
}

impl Prompt {
    fn new(config: ConsoleConfig) -> Self {
        Self {
            buf: default(),
            config,
        }
    }

    pub fn message<T: Into<Cow<'static, str>>>(&mut self, msg: T) {
        self.config.state.lock().unwrap().push(msg.into().into())
    }
}
