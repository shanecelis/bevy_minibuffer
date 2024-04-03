use std::fmt::{Debug, Display};
use bevy::ecs::prelude::*;
use bevy::tasks::block_on;
use futures_lite::future;
use asky::{Message, bevy::TaskSink};
use crate::Minibuffer;

/// Check for tasks which may emit a event we want to send.
pub fn poll_event_tasks<T: Send + Event>(
    mut commands: Commands,
    mut run_command: EventWriter<T>,
    mut tasks: Query<(Entity, &mut TaskSink<Option<T>>)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(maybe) = future::block_on(future::poll_once(&mut task.0)) {
            if let Some(event) = maybe {
                run_command.send(event);
            }
            commands.entity(entity).despawn();
        }
    }
}

pub fn poll_tasks_err<T: Send + Sync + 'static, E: Debug + Display + Send + Sync + 'static>(
    mut commands: Commands,
    asky: Minibuffer,
    mut tasks: Query<(Entity, &mut TaskSink<Result<T, E>>)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            // Once
            if let Err(error) = result {
                eprintln!("XXX Got error here {:?}.", error);
                // FIXME: I need the right entity to make this work.

                let a = asky.clone();
                let future = async move {
                    let _ = a.clone().prompt(Message::new(format!("error: {}", error))).await;
                };
                commands.spawn(TaskSink::new(future));

            }
            commands.entity(entity).despawn();
        }
    }
}
