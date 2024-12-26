//! Acts and their flags, builders, and collections
use crate::{event::RunActEvent, input::Hotkey};
use bevy::{
    ecs::{system::{EntityCommand, RegisteredSystemError, SystemId}, world::CommandQueue},
    prelude::*,
};
use bevy_input_sequence::{action, input_sequence::KeySequence, KeyChord};
use bitflags::bitflags;
use std::{
    marker::PhantomData,
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Display,
        // Write
    },
    any::{Any, TypeId},
    sync::Arc,
};

#[derive(Debug)]
pub enum RunActError {
    CannotAcceptInput,
    RegisteredSystemError,
    CannotConvertInput,
}

pub trait RunAct {
    fn run(&self, system_entity: Entity, world: &mut Commands) -> Result<(), RunActError>;
    fn run_with_input(&self, system_entity: Entity, input: &dyn Any, world: &mut Commands) -> Result<(), RunActError>;
    fn debug_string(&self, input: &dyn Any) -> Option<String>;
    // fn system_name(&self) -> Cow<'static, str>;
}

#[derive(Clone, Debug)]
pub struct ActSystem;
/// An alternative implementation that works directly on the world. It's not currently used.
// mod world {
//     use super::*;
//     pub trait RunAct {
//         fn run(&self, world: &mut World) -> Result<(), RunActError>;
//         fn run_with_input(&self, input: &dyn Any, world: &mut World) -> Result<(), RunActError>;
//     }
//     impl RunAct for ActSystem {
//         fn run(&self, world: &mut World) -> Result<(), RunActError> {
//             world.run_system(self.0).map_err(|_| RunActError::RegisteredSystemError)
//         }

//         fn run_with_input(&self, input: &dyn Any, world: &mut World) -> Result<(), RunActError> {
//             Err(RunActError::CannotAcceptInput)
//         }
//     }

//     impl<I> RunAct for ActWithInputSystem<I> where I: Default + Clone {
//         fn run(&self, world: &mut World) -> Result<(), RunActError> {
//             world.run_system_with_input(self.0, I::default()).map_err(|_| RunActError::RegisteredSystemError)
//         }

//         fn run_with_input(&self, input: &dyn Any, world: &mut World) -> Result<(), RunActError> {
//             match input.downcast_ref::<I>() {
//                 Some(input) => {
//                     let input = input.clone();
//                     world.run_system_with_input(self.0, input).map_err(|_| RunActError::RegisteredSystemError)
//                 }
//                 None => Err(RunActError::CannotConvertInput),
//             }
//         }
//     }
// }

impl RunAct for ActSystem {
    fn run(&self, system_entity: Entity, commands: &mut Commands) -> Result<(), RunActError> {
        let system_id = SystemId::from_entity(system_entity);
        commands.run_system(system_id);
        Ok(())
    }

    fn run_with_input(&self, system_entity: Entity, input: &dyn Any, commands: &mut Commands) -> Result<(), RunActError> {
        Err(RunActError::CannotAcceptInput)
    }

    fn debug_string(&self, input: &dyn Any) -> Option<String> {
        None
    }

    // fn system_name(&self) -> Cow<'static, str> {
    //     self.1.clone()
    // }
}

#[derive(Clone, Debug)]
pub struct ActWithInputSystem<I: 'static>(PhantomData<Fn(I)>);

impl<I: 'static> ActWithInputSystem<I> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<I> RunAct for ActWithInputSystem<I> where I: Clone + Default + Debug + Send + Sync {
    fn run(&self, system_entity: Entity, commands: &mut Commands) -> Result<(), RunActError> {

        let system_id = SystemId::<In<I>>::from_entity(system_entity);
        commands.run_system_with_input(system_id, I::default());
        Ok(())
    }

    fn run_with_input(&self, system_entity: Entity, input: &dyn Any, commands: &mut Commands) -> Result<(), RunActError> {
        // The debugging with Any was _rough_.
        // info!("input typeid {:?}", input.type_id());
        // info!("Arc typeid {:?}", TypeId::of::<Arc<dyn Any>>());
        // info!("Arc 2typeid {:?}", TypeId::of::<Arc<dyn Any + 'static + Send + Sync>>());
        // info!("Option<f32> typeid {:?}", TypeId::of::<Option<f32>>());
        // info!("&Option<f32> typeid {:?}", TypeId::of::<&Option<f32>>());
        // info!("f32 typeid {:?}", TypeId::of::<f32>());
        // info!("&f32 typeid {:?}", TypeId::of::<&f32>());
        match input.downcast_ref::<I>() {
            Some(input) => {
                let system_id = SystemId::<In<I>>::from_entity(system_entity);
                let input = input.clone();
                commands.run_system_with_input(system_id, input);
                Ok(())
            }
            None => Err(RunActError::CannotConvertInput),
        }
    }

    // fn system_name(&self) -> Cow<'static, str> {
    //     self.1.clone()
    // }

    fn debug_string(&self, input: &dyn Any) -> Option<String> {
        input.downcast_ref::<I>().map(|input: &I| format!("{:?}", input))
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RunActMap(Box<dyn RunAct + Send + Sync>);

impl RunActMap {
    pub fn new(runner: impl RunAct + Send + Sync + 'static) -> Self {
        Self(Box::new(runner))
    }
}
