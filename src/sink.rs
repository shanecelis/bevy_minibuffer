//! Pipe systems with futures into a sink.
use crate::Minibuffer;
use bevy::ecs::system::In;
use std::fmt::Display;
#[cfg(doc)]
use crate::acts::ActFlags;

/// Show error if any in minibuffer.
pub fn result<T, E>(In(result): In<Result<T, E>>, mut minibuffer: Minibuffer)
where
    T: 'static,
    E: 'static + Display,
{
    if let Err(e) = result {
        minibuffer.message(format!("{e}"));
    }
}

/// Pipe a string to the message buffer.
///
/// The minibuffer might not be visible when this is called. Consider adding
/// [ActFlags::ShowMinibuffer] to the act's flags to ensure it will be shown.
///
/// Used internally by `list_acts` for instance
///
/// ```ignore
/// ActBuilder::new(list_acts.pipe(message))
///     .named("list_acts")
///     .add_flags(ActFlags::ShowMinibuffer)
///     .hotkey(keyseq! { Ctrl-H A }),
/// ```
pub fn string(In(msg): In<String>, mut minibuffer: Minibuffer) {
    minibuffer.message(msg);
}

/// Can optinoally pipe any string to the message buffer.
pub fn option_string(In(msg): In<Option<String>>, mut minibuffer: Minibuffer) {
    if let Some(msg) = msg {
        minibuffer.message(msg);
    }
}

#[cfg(feature = "async")]
mod future {
    use super::*;
    use crate::MinibufferAsync as Minibuffer;
    use bevy_defer::{AsyncExecutor, NonSend};
    use std::future::Future;
    /// Execute a future.
    pub fn future<F: Future<Output = ()> + 'static>(
        In(future): In<F>,
        exec: NonSend<AsyncExecutor>,
    ) {
        exec.spawn_any(future);
    }

    /// Show error if any in minibuffer.
    pub fn future_result<
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
}
#[cfg(feature = "async")]
pub use future::*;
