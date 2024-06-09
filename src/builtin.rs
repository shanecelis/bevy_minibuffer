use crate::{
    act, future_result_sink,
    prelude::{keyseq, ActBuilder},
};
use asky::bevy::future_sink;
use bevy::ecs::system::IntoSystem;

/// Construct builtin acts
// pub struct Builtin<'w> {
//     world: &'w  World,
// }
pub struct Builtin;

impl Builtin {
    // pub fn new<'a: 'w>(world: &'a  World) -> Builtin<'w> {
    //     Self { world }
    // }

    pub fn exec_act(&self) -> ActBuilder {
        ActBuilder::new(act::exec_act.pipe(future_result_sink))
            .named("exec_act")
            .hotkey(keyseq! { shift-; })
            .hotkey(keyseq! { alt-X })
            .in_exec_act(false)
    }

    pub fn list_acts(&self) -> ActBuilder {
        ActBuilder::new(act::list_acts.pipe(future_sink))
            .named("list_acts")
            .hotkey(keyseq! { ctrl-H A })
    }

    pub fn list_key_bindings(&self) -> ActBuilder {
        self.list_bindings().named("list_key_bindings")
    }

    /// Create a new command that lists bindings for event `E`.
    pub fn list_bindings(&self) -> ActBuilder {
        ActBuilder::new(act::list_key_bindings.pipe(future_sink)).hotkey(keyseq! { ctrl-H B })
    }

    pub fn describe_key(&self) -> ActBuilder {
        ActBuilder::new(act::describe_key.pipe(future_result_sink))
            .named("describe_key")
            .hotkey(keyseq! { ctrl-H K })
    }
}

impl bevy::app::Plugin for Builtin {
    fn build(&self, app: &mut bevy::app::App) {
        for act_builder in [
            self.exec_act(),
            self.list_acts(),
            self.list_key_bindings(),
            self.describe_key(),
        ] {
            let act = act_builder.build(&mut app.world);
            app.world.spawn(act);
        }
    }
}
