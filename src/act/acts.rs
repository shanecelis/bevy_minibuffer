use crate::act::ActBuilder;
use bevy::prelude::*;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct ActsPlugin {
    /// Why use RwLock? Because `Plugin` must be `Send` and `ActBuilder` is not
    /// `Clone`. In `build(&self)` we want to consume the acts but we only have
    /// `&self` so this is our workaround.
    acts: RwLock<Vec<ActBuilder>>,
}

impl ActsPlugin {
    pub fn new<I: IntoIterator<Item = ActBuilder>>(v: I) -> Self {
        ActsPlugin {
            acts: RwLock::new(v.into_iter().collect()),
        }
    }

    pub fn get(&self) -> RwLockReadGuard<Vec<ActBuilder>> {
        self.acts.read().unwrap()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<Vec<ActBuilder>> {
        self.acts.write().unwrap()
    }

    pub fn clear(&self) {
        let _ = self.get_mut().drain(..);
    }
}

impl Default for ActsPlugin {
    fn default() -> Self {
        ActsPlugin::new(vec![])
    }
}

impl Plugin for ActsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        for act in self.acts.write().unwrap().drain(..) {
            let world = app.world_mut();
            let act = act.build(world);
            let keyseqs = act.build_keyseqs(world);
            world.spawn(act)
             .with_children(|builder| {
                 for keyseq in keyseqs {
                     builder.spawn(keyseq);
                 }
             });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use super::*;

    fn act1() {}
    #[test]
    fn check_acts() {
        let plugin = ActsPlugin::default();
        assert_eq!(plugin.get().len(), 0);
    }

    #[test]
    fn check_drain_read() {
        let plugin = ActsPlugin::default();
        plugin.get_mut().push(Act::new(act1));
        assert_eq!(plugin.get().len(), 1);
    }
}
