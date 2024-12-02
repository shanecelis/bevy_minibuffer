use super::Acts;
use bevy::prelude::*;

/// A [Plugin] with a collection of [Acts]
///
/// Although it is a [Plugin], if you use [App::add_plugins] with it, then its
/// acts will not be added. [ActBuilder] contains a non-cloneable field that
/// must be taken. [Plugin::build] does not permit this with its read-only
/// `&self` access. Instead we use [AddActs::add_acts] to add both the acts and
/// the Plugin comprising an ActsPlugin.
pub trait ActsPlugin: Plugin {
    /// Immutable reference to acts.
    fn acts(&self) -> &Acts;
    /// Mutable reference to acts.
    fn acts_mut(&mut self) -> &mut Acts;

    /// Take the acts. This removes them from the plugin so that they may be
    /// altered and the plugin may then be added with [App::add_plugins] or
    /// [AddActs::add_acts].
    fn take_acts(&mut self) -> Acts {
        self.acts_mut().take()
    }

    /// Warn if there are unused acts in this `ActsPlugin`.
    ///
    /// Typically this should be called in the implementers `Plugin::build()`
    /// method. This will provide some safeguard to ensure `Act`s don't get lost
    /// in the shuffle.
    fn warn_on_unused_acts(&self) {
        let acts = self.acts();
        if !acts.is_empty() {
            let count = acts.len();
            warn!(
                "{} has {} act{} not added; consider using add_acts() instead of add_plugins() for it.",
                self.name(),
                count,
                if count == 1 { " that was" } else { "s that were" }
            );
        }
    }
}
