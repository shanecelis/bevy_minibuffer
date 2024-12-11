use bevy::prelude::*;

/// A marker for [ActBuilder]s.
pub trait ActBuilders<Marker>: sealed::ActBuilders<Marker> {}
impl<Marker, T> ActBuilders<Marker> for T where T: sealed::ActBuilders<Marker> {}

mod sealed {
    use crate::acts::{ActBuilder, Acts, ActsPlugin};
    use bevy::{
        app::App,
        ecs::world::{Command, World},
        prelude::IntoSystem,
    };
    pub struct ActsPluginMarker;
    pub struct ActBuilderMarker;
    pub struct MutActBuilderMarker;
    pub struct ActsMarker;
    pub struct SystemMarker;
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

    impl<S: IntoSystem<(), (), P> + 'static, P> ActBuilders<(SystemMarker, P)> for S {
        fn add_to_world(self, world: &mut World) {
            ActBuilder::new(self).add_to_world(world);
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
