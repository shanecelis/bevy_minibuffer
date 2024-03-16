use bevy::ecs::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::future::Future;

use asky::bevy::TaskSink;
// #[derive(Component)]
// pub struct TaskSink<T>(pub Task<T>);

// impl<T: Send + 'static> TaskSink<T> {
//     pub fn new(future: impl Future<Output = T> + Send + 'static) -> Self {
//         let thread_pool = AsyncComputeTaskPool::get();
//         let task = thread_pool.spawn(future);
//         Self(task)
//     }
// }
// pub fn task_sink<T: Send + 'static>(
//     In(future): In<impl Future<Output = T> + Send + 'static>,
//     mut commands: Commands,
// ) {
//     eprintln!("spawn task sink for type {:?}", std::any::type_name::<T>());
//     // commands.spawn(TaskSink::new(async move { future.await }));
//     commands.spawn(TaskSink::new(future));
// }

// pub fn poll_tasks(mut commands: Commands, mut tasks: Query<(Entity, &mut TaskSink<()>)>) {
//     for (entity, mut task) in &mut tasks {
//         if future::block_on(future::poll_once(&mut task.0)).is_some() {
//             eprintln!("Got () poll task");
//             // Once
//             //
//             commands.entity(entity).despawn();
//         }
//     }
// }

/// Check for tasks which may emit a event we want to send.
pub fn poll_event_tasks<T: Send + Event>(
    mut commands: Commands,
    mut run_command: EventWriter<T>,
    mut tasks: Query<(Entity, &mut TaskSink<Option<T>>)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(maybe) = future::block_on(future::poll_once(&mut task.0)) {
            eprintln!("Got event poll task");
            if let Some(event) = maybe {
                run_command.send(event);
            }
            commands.entity(entity).despawn();
        }
    }
}

