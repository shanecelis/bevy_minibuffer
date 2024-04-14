use asky::bevy::future_sink;
use crate::{
    prelude::{keyseq, Act},
    act::exec_act,
};
use bevy::ecs::{world::World, system::IntoSystem};

pub struct Builtin<'w> {
    world: &'w mut World,
    // pub exec_act: Act,
}

impl<'w> Builtin<'w> {
    pub fn new(world: &mut World) -> Builtin<'_> {
        Self { world }
    }

    pub fn exec_act(&self) -> Act {
            Act::new()
                .named("exec_act")
                .hotkey(keyseq! { shift-; })
                .hotkey(keyseq! { alt-X })
                .in_exec_act(false)
                .register(exec_act.pipe(future_sink), self.world)
    }
}
