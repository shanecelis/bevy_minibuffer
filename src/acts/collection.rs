use super::ActBuilder;
use bevy::prelude::*;
use std::{borrow::Cow, collections::HashMap};

/// A collection of acts
///
/// Acts may be inspected and modified before adding to app.
#[derive(Debug, Deref, DerefMut, Default)]
pub struct Acts(pub HashMap<Cow<'static, str>, ActBuilder>);

impl Acts {
    /// Create a new plugin with a set of acts.
    pub fn new<I: IntoIterator<Item = impl Into<ActBuilder>>>(v: I) -> Self {
        Acts(
            v.into_iter()
                .map(|act| {
                    let act = act.into();
                    (act.name(), act)
                })
                .collect(),
        )
    }

    /// Take the acts replacing self with its default value.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Add an [ActBuilder].
    pub fn push(&mut self, builder: impl Into<ActBuilder>) -> Option<ActBuilder> {
        let builder = builder.into();
        self.insert(builder.name(), builder).inspect(|builder| {
            warn!("Replacing act '{}'.", builder.name());
        })
    }
}

impl From<ActBuilder> for Acts {
    fn from(builder: ActBuilder) -> Acts {
        Acts::new([builder])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    fn act1() {}
    #[test]
    fn check_acts() {
        let acts = Acts::default();
        assert_eq!(acts.len(), 0);
    }

    #[test]
    fn check_drain_read() {
        let mut acts = Acts::default();
        acts.push(Act::new(act1));
        assert_eq!(acts.len(), 1);
    }

    #[test]
    fn check_duplicate_names() {
        let mut acts = Acts::default();
        acts.push(Act::new(act1));
        assert!(acts.push(Act::new(act1)).is_some());
        assert_eq!(acts.len(), 1);
    }
}
