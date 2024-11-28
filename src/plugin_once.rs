use bevy::prelude::*;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display,
          // Write
    },
    sync::Mutex,
};

/// A plugin that can only be built once.
pub trait PluginOnce {
    /// Build the plugin.
    fn build(self, app: &mut App);

    /// Convert into a standard plugin.
    fn into_plugin(self) -> PluginOnceShim<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

/// A plugin for [ActBuilder], which must consumes `self` to build, so this
/// plugin holds it and uses interior mutability.
#[derive(Debug)]
pub struct PluginOnceShim<T: PluginOnce> {
    builder: Mutex<Option<T>>,
}

impl<T: PluginOnce> From<T> for PluginOnceShim<T> {
    fn from(builder: T) -> Self {
        PluginOnceShim {
            builder: Mutex::new(Some(builder)),
        }
    }
}

// impl PluginOnce for ActBuilder {
//     fn build(self, app: &mut App) {
//         let world = app.world_mut();
//         let act = self.build(world);
//         let keyseqs = act.build_keyseqs(world);
//         world.spawn(act).with_children(|builder| {
//             for keyseq in keyseqs {
//                 builder.spawn(keyseq);
//             }
//         });
//     }
// }

impl<T: PluginOnce + Sync + Send + 'static> Plugin for PluginOnceShim<T> {
    fn build(&self, app: &mut App) {
        if let Some(builder) = self.builder.lock().expect("plugin once").take() {
            PluginOnce::build(builder, app);
        } else {
            warn!("plugin once shim called a second time");
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}
