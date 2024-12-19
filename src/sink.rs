//! Pipe systems with futures into a sink.
use crate::MinibufferAsync as Minibuffer;
#[allow(unused_imports)]
use bevy::{ecs::system::In, prelude::warn};
use bevy_defer::{AsyncExecutor, NonSend};
use std::{fmt::Display, future::Future};

/// Show error if any in minibuffer.
pub fn result<T, E>(In(result): In<Result<T,E>>,
                    mut minibuffer: Minibuffer)
where T: 'static,
      E: 'static + Display
{
    if let Err(e) = result {
        minibuffer.message(format!("{e}"));
    }
}

// TODO: Rename these to no sink suffix. Use the sink module name.
/// Show error if any in minibuffer.
pub fn future_result_sink<
    T: 'static,
    E: 'static + Display,
    F: Future<Output = Result<T, E>> + 'static,
>(
    In(future): In<F>,
    exec: NonSend<AsyncExecutor>,
    mut minibuffer: Minibuffer,
) {
    exec.spawn_any(async move {
        if let Err(e) = future.await {
            minibuffer.message(format!("{e}"));
        }
    });
}

/// Execute a future.
pub fn future_sink<F: Future<Output = ()> + 'static>(
    In(future): In<F>,
    exec: NonSend<AsyncExecutor>,
) {
    exec.spawn_any(future);
}
