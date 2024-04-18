use asky::Message;
use bevy::ecs::system::In;
use bevy_defer::{NonSend, AsyncExecutor};
use crate::Minibuffer;
use std::{
    future::Future,
    fmt::Display,
};
#[allow(unused_imports)]
pub use asky::bevy::{future_sink,
                     option_future_sink};

/// Show error if any in minibuffer.
pub fn future_result_sink<T: 'static, E: 'static + Display, F: Future<Output = Result<T, E>> + 'static>(
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
