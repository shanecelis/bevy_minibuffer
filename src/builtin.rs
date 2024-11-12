use crate::{
    act::{self, PluginOnce}, future_sink, future_result_sink,
    prelude::{keyseq, ActBuilder, ActsPlugin},
};
use bevy::ecs::system::IntoSystem;

/// Builtin acts: exec_act, list_acts, list_key_bindings, describe_key.
pub struct Builtin {
    /// Set of builtin acts
    pub acts: ActsPlugin
}

impl Default for Builtin {
    fn default() -> Self {
        Self {
            acts:
            ActsPlugin::new([
                ActBuilder::new(act::exec_act.pipe(future_result_sink))
                    .named("exec_act")
                    .hotkey(keyseq! { shift-; })
                    .hotkey(keyseq! { alt-X })
                    .in_exec_act(false),
                ActBuilder::new(act::list_acts.pipe(future_sink))
                    .named("list_acts")
                    .hotkey(keyseq! { ctrl-H A }),
                ActBuilder::new(act::list_key_bindings.pipe(future_sink))
                    .named("list_key_bindings")
                    .hotkey(keyseq! { ctrl-H B }),
                ActBuilder::new(act::describe_key.pipe(future_result_sink))
                    .named("describe_key")
                    .hotkey(keyseq! { ctrl-H K })
            ])
        }
    }
}

impl PluginOnce for Builtin {
    fn build(self, app: &mut bevy::app::App) {
        self.acts.build(app);
    }
}
