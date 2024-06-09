use crate::Minibuffer;
#[allow(unused_imports)]
pub use asky::bevy::{future_sink, option_future_sink};
use asky::Message;
use bevy::ecs::system::In;
use bevy_defer::{AsyncExecutor, NonSend};
use std::{fmt::Display, future::Future};

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
    exec.spawn(async move {
        if let Err(e) = future.await {
            let _ = minibuffer.prompt(Message::new(format!("error {e}"))).await;
        }
    });
}
