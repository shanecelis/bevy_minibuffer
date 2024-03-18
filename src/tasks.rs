use bevy::ecs::prelude::*;
use futures_lite::future;

use asky::bevy::TaskSink;

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

