use crate::act::ActBuilder;
use bevy::prelude::*;
use std::{borrow::Cow, collections::HashMap};

/// An ActsPlugin is a plugin with a collection of [Acts].
///
/// Although it is a [Plugin], if you use [App::add_plugins] with it, then its
/// acts will not be added. [ActBuilder] contains a non-cloneable field that
/// must be taken. [Plugin::build] does not permit this with its read-only
/// `&self` access. Instead we use [AddActs::add_acts] to add both the acts and
/// the Plugin comprising an ActsPlugin.
pub trait ActsPlugin: Plugin {
    // fn acts(&self) -> &Acts;
    // fn acts_mut(&mut self) -> &mut Acts;

    /// Take the acts. This removes them from the plugin so that they may be
    /// altered and the plugin may be added with [App::add_plugins] or
    /// [AddActs::add_acts].
    fn take_acts(&mut self) -> Acts;
}

/// A collection of acts
///
/// Acts may be inspected and modified before adding to app.
#[derive(Debug, Deref, DerefMut, Default)]
pub struct Acts(pub HashMap<Cow<'static, str>, ActBuilder>);

impl Acts {
    /// Create a new plugin with a set of acts.
    pub fn new<I: IntoIterator<Item = impl Into<ActBuilder>>>(v: I) -> Self {
        Acts(
            v.into_iter()
                .map(|act| {
                    let act = act.into();
                    (act.name(), act)
                })
                .collect(),
        )
    }

    /// Take the acts replacing self with its default value.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Add an [ActBuilder].
    pub fn push(&mut self, builder: impl Into<ActBuilder>) -> Option<ActBuilder> {
        let builder = builder.into();
        self.insert(builder.name(), builder).inspect(|builder| {
            warn!("Replacing act '{}'.", builder.name());
        })
    }
}

impl From<ActBuilder> for Acts {
    fn from(builder: ActBuilder) -> Acts {
        Acts::new([builder])
    }
}

/// A marker for [ActBuilder]s.
pub trait ActBuilders<Marker>: sealed::ActBuilders<Marker> {}
impl<Marker, T> ActBuilders<Marker> for T where T: sealed::ActBuilders<Marker> {}

mod sealed {
    use crate::act::{ActBuilder, Acts, ActsPlugin};
    use bevy::{
        app::App,
        ecs::world::{Command, World},
    };
    pub struct ActsPluginMarker;
    pub struct ActBuilderMarker;
    pub struct MutActBuilderMarker;
    pub struct ActsMarker;
    // pub struct IntoActsMarker;
    pub struct ActBuildersTupleMarker;

    pub trait ActBuilders<Marker> {
        fn add_to_app(self, app: &mut App)
        where
            Self: Sized,
        {
            self.add_to_world(app.world_mut());
        }

        fn add_to_world(self, _world: &mut World)
        where
            Self: Sized;
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

        fn add_to_world(self, _world: &mut World) {
            panic!("This should not be called.");
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
            impl<$($param, $plugins),*> ActBuilders<(ActBuildersTupleMarker, $($param,)*)> for ($($plugins,)*)
            where
                $($plugins: ActBuilders<$param>),*
            {
                #[allow(non_snake_case, unused_variables)]
                #[track_caller]
                fn add_to_app(self, app: &mut App) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_app(app);)*
                }
                fn add_to_world(self, _world: &mut World) {
                }
            }
        }
    }

    bevy::utils::all_tuples!(impl_plugins_tuples, 0, 15, P, S);
}

/// An extension to App to add acts.
pub trait AddActs {
    /// Adds the given acts to itself.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

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
