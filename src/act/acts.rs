use crate::act::{ActBuilder, PluginOnce};
use bevy::prelude::*;

/// Houses a collection of acts.
///
/// Acts may be inspected and modified before adding to app.
#[derive(Debug, Deref, DerefMut)]
pub struct ActsPlugin {
    // Why use RwLock? Because `Plugin` must be `Send` and `ActBuilder` is not
    // `Clone`. In `build(&self)` we want to consume the acts but we only have
    // `&self` so this is our workaround.
    acts: Vec<ActBuilder>,
}

// impl trait IndexMut<AsRef<str>> for ActsPlugin {
//     type Output = Act;

//     fn index(

// }

impl ActsPlugin {
    /// Create a new plugin with a set of acts.
    pub fn new<I: IntoIterator<Item = ActBuilder>>(v: I) -> Self {
        ActsPlugin {
            acts: v.into_iter().collect(),
        }
    }

    /// Take an act from this collection.
    pub fn take(&mut self, name: impl AsRef<str>) -> Option<ActBuilder> {
        let name = name.as_ref();
        self.acts
            .iter()
            .position(|act| act.name() == name)
            .map(|index| self.acts.remove(index))
    }

    // /// Get the current acts readonly.
    // pub fn get(&self) -> RwLockReadGuard<Vec<ActBuilder>> {
    //     self.acts.read().unwrap()
    // }

    // /// Get the current acts mutable.
    // pub fn get_mut(&self) -> RwLockWriteGuard<Vec<ActBuilder>> {
    //     self.acts.write().unwrap()
    // }

    // /// Clear all the acts.
    // pub fn clear(&self) {
    //     let _ = self.get_mut().drain(..);
    // }
}

impl Default for ActsPlugin {
    fn default() -> Self {
        ActsPlugin::new(vec![])
    }
}

impl PluginOnce for ActsPlugin {
    fn build(mut self, app: &mut bevy::app::App) {
        for act in self.acts.drain(..) {
            PluginOnce::build(act, app);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::prelude::*;
//     use super::*;

//     fn act1() {}
//     #[test]
//     fn check_acts() {
//         let plugin = ActsPlugin::default();
//         assert_eq!(plugin.get().len(), 0);
//     }

//     #[test]
//     fn check_drain_read() {
//         let plugin = ActsPlugin::default();
//         plugin.get_mut().push(Act::new(act1));
//         assert_eq!(plugin.get().len(), 1);
//     }
// }
