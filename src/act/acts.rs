use crate::act::ActBuilder;
use bevy::prelude::*;
use std::{borrow::Cow, collections::HashMap};


pub trait ActsPlugin: Plugin {
    fn take_acts(&mut self) -> Acts;
}

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

    pub fn push(&mut self, builder: impl Into<ActBuilder>) -> Option<ActBuilder> {
        let builder = builder.into();
        self.insert(builder.name(), builder).map(|builder| {
            warn!("Replacing act '{}'.", builder.name());
            builder
        })
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

pub trait ActBuilders<Marker>: sealed::ActBuilders<Marker> {}
impl <Marker, T> ActBuilders<Marker> for T where T: sealed::ActBuilders<Marker> {}

mod sealed {
    use bevy::{app::App, ecs::world::{World, Command}};
    use crate::{act::{Acts, ActBuilder, ActsPlugin}};
    pub struct PluginOnceMarker;
    pub struct ActsPluginMarker;
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

    impl<P: ActsPlugin> ActBuilders<ActsPluginMarker> for P {
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

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use super::*;

    fn act1() {}
    #[test]
    fn check_acts() {
        let acts = Acts::default();
        assert_eq!(acts.len(), 0);
    }

    #[test]
    fn check_drain_read() {
        let mut acts = Acts::default();
        acts.push(Act::new(act1));
        assert_eq!(acts.len(), 1);
    }

    #[test]
    fn check_duplicate_names() {
        let mut acts = Acts::default();
        acts.push(Act::new(act1));
        assert!(acts.push(Act::new(act1)).is_some());
        assert_eq!(acts.len(), 1);
    }
}
