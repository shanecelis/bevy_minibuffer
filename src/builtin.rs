use std::fmt::Display;
use asky::bevy::future_sink;
use crate::{
    prelude::{keyseq, Act, RunActEvent},
    act
};
use bevy::ecs::{event::Event, world::World, system::IntoSystem};

/// Construct builtin acts
pub struct Builtin<'w> {
    world: &'w mut World,
}

impl<'w> Builtin<'w> {
    pub fn new<'a: 'w>(world: &'a mut World) -> Builtin<'w> {
        Self { world }
    }

    pub fn exec_act(&mut self) -> Act {
            Act::new()
                .named("exec_act")
                .hotkey(keyseq! { shift-; })
                .hotkey(keyseq! { alt-X })
                .in_exec_act(false)
                .register(act::exec_act.pipe(future_sink), self.world)
    }

    pub fn list_acts(&mut self) -> Act {
        Act::new()
            .named("list_acts")
            .hotkey(keyseq! { ctrl-H A })
            .register(act::list_acts.pipe(future_sink), self.world)
    }

    pub fn list_key_bindings(&mut self) -> Act {
        self.list_bindings::<RunActEvent>()
            .named("list_key_bindings")
    }

    /// Create a new command that lists bindings for event `E`.
    pub fn list_bindings<E: Event + Clone + Display>(&mut self) -> Act {
        Act::new()
            .hotkey(keyseq! { ctrl-H B })
            .register(act::list_key_bindings::<E>.pipe(future_sink), self.world)
    }
}
