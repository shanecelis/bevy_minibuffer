use crate::act::{ActBuilder, PluginOnce};
use bevy::prelude::*;
use std::{borrow::Cow, collections::HashMap};

/// Houses a collection of acts.
///
/// Acts may be inspected and modified before adding to app.
#[derive(Debug, Deref, DerefMut)]
pub struct Acts(pub HashMap<Cow<'static, str>, ActBuilder>);

impl Acts {
    /// Create a new plugin with a set of acts.
    pub fn new<I: IntoIterator<Item = impl Into<ActBuilder>>>(v: I) -> Self {
        Acts(v.into_iter().map(|act| { let act = act.into(); (act.name(), act) }).collect())
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, Acts::default())
    }
}

impl From<ActBuilder> for Acts {
    fn from(builder: ActBuilder) -> Acts {
        Acts::new([builder])
    }
}

// impl From<&mut ActBuilder> for Acts {
//     fn from(builder: &mut ActBuilder) -> Acts {
//         Acts::new([ActBuilder::from(builder)])
//     }
// }

impl Default for Acts {
    fn default() -> Self {
        Acts(HashMap::new())
    }
}

impl PluginOnce for Acts {
    fn build(mut self, app: &mut bevy::app::App) {
        for (_, act) in self.drain() {
            PluginOnce::build(act, app);
        }
    }
}

pub trait ActBuilders<Marker>: sealed::ActBuilders<Marker> {}
impl <Marker, T> ActBuilders<Marker> for T where T: sealed::ActBuilders<Marker> {}

mod sealed {
    use bevy::{app::App, ecs::world::{World, Command}};
    use crate::{universal::PluginWithActs, act::{Acts, PluginOnce, ActBuilder}};
    pub struct PluginOnceMarker;
    pub struct PluginWithActsMarker;
    pub struct ActBuilderMarker;
    pub struct MutActBuilderMarker;
    pub struct ActsMarker;
    pub struct IntoActsMarker;
    pub struct ActBuilderTupleMarker;

    pub trait ActBuilders<Marker> {
        fn add_to_app(mut self, app: &mut App) where Self: Sized {
            self.add_to_world(app.world_mut());
        }

        fn add_to_world(self, world: &mut World);//  where Self: Sized {
        //     todo!("No add_to_world implementation.");
        // }
    }

    impl ActBuilders<ActBuilderMarker> for ActBuilder {
        fn add_to_world(self, world: &mut World) {
            self.apply(world);
        }
    }

    // impl<T> ActBuilders<IntoActsMarker> for T where Acts: From<T> {
    //     fn add_to_world(self, world: &mut World) {
    //         let acts = Acts::from(self);
    //         for (_, v) in acts.0.into_iter() {
    //             v.apply(world);
    //         }
    //     }
    // }

    impl<P: PluginWithActs> ActBuilders<PluginWithActsMarker> for P {
        fn add_to_app(mut self, app: &mut App) {
            let acts: Acts = self.take_acts();
            <Acts as ActBuilders<ActsMarker>>::add_to_world(acts, app.world_mut());
            app.add_plugins(self);
        }

        fn add_to_world(self, world: &mut World) {
            panic!("This shouldn't be called.");
        }
    }

    // impl<P: PluginOnce> ActBuilders<PluginOnceMarker> for P {
    //     fn add_to_world(self, world: &mut World) {
    //         todo!();
    //         // self.apply(world);
    //     }
    // }

    impl ActBuilders<MutActBuilderMarker> for &mut ActBuilder {
        fn add_to_world(self, world: &mut World) {
            ActBuilder::from(self).apply(world);
        }
    }

    impl ActBuilders<ActsMarker> for Acts {
        fn add_to_world(self, world: &mut World) {
            for (_, v) in self.0.into_iter() {
                v.apply(world);
            }
        }
    }

    macro_rules! impl_plugins_tuples {
        ($(($param: ident, $plugins: ident)),*) => {
            impl<$($param, $plugins),*> ActBuilders<(ActBuilderTupleMarker, $($param,)*)> for ($($plugins,)*)
            where
                $($plugins: ActBuilders<$param>),*
            {
                #[allow(non_snake_case, unused_variables)]
                #[track_caller]
                fn add_to_app(self, app: &mut App) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_app(app);)*
                }
                fn add_to_world(self, world: &mut World) {
                }
            }
        }
    }

    bevy::utils::all_tuples!(impl_plugins_tuples, 0, 15, P, S);
}


pub trait AddActs {
    fn add_acts<M>(&mut self, acts: impl ActBuilders<M>) -> &mut Self;
}

impl AddActs for App {
    fn add_acts<M>(&mut self, acts: impl ActBuilders<M>) -> &mut Self {
        acts.add_to_app(self);
        self
    }
}

// impl<'w, 's> AddActs for Commands<'w, 's> {
//     fn add_acts<M>(&mut self, acts: impl ActBuilders<M>) -> &mut Self {
//         self.add(|world: &mut World| {
//             acts.add_to_world(world);
//         });
//         self
//     }
// }
// pub struct PluginActsMarker;

// impl bevy::app::Plugins<PluginActsMarker> for Acts {
//     fn add_to_world(self, app: &mut App) {
//         todo!();
//         // app.add_plugins(self.into_plugin());
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::prelude::*;
//     use super::*;

//     fn act1() {}
//     #[test]
//     fn check_acts() {
//         let plugin = Acts::default();
//         assert_eq!(plugin.get().len(), 0);
//     }

//     #[test]
//     fn check_drain_read() {
//         let plugin = Acts::default();
//         plugin.get_mut().push(Act::new(act1));
//         assert_eq!(plugin.get().len(), 1);
//     }
// }
