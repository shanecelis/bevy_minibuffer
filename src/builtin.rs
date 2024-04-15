use crate::{
    act,
    prelude::{keyseq, Act, RunActEvent},
    future_result_sink,
};
use asky::bevy::future_sink;
use bevy::ecs::{event::Event, system::IntoSystem, world::World};
use std::fmt::Display;

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
            .register(act::exec_act.pipe(future_result_sink), self.world)
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

    pub fn describe_key(&mut self) -> Act {
        Act::new()
            .named("describe_key")
            .hotkey(keyseq! { ctrl-H K })
            .register(act::describe_key::<RunActEvent>.pipe(future_result_sink), self.world)
    }

    /// Add all the builtin acts.
    pub fn add_all(&mut self) {
        for act in [
            self.exec_act(),
            self.list_acts(),
            self.list_key_bindings(),
            self.describe_key(),
        ] {
            self.world.spawn(act);
        }
    }
}
