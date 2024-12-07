//! Bare minimum of acts for a useable and discoverable console
use crate::{
    acts::{ActCache, ActFlags, ActsPlugin, basic::BasicActs},
    autocomplete::Resolve,
    event::LastRunAct,
    prelude::*,
    prelude::{keyseq, ActBuilder, Acts},
};

use std::{
    borrow::Cow,
    fmt::{Debug, Write},
};
use crate::sink::future_result_sink;
use bevy::prelude::*;
use bevy_defer::AsyncWorld;
use trie_rs::map::{Trie, TrieBuilder};
use futures::Future;

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
pub fn run_act(
    mut minibuffer: MinibufferAsync,
    acts: Query<&Act>,
    last_act: Res<LastRunAct>,
) -> impl Future<Output = Result<(), crate::Error>> {
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        if act.flags.contains(ActFlags::RunAct | ActFlags::Active) {
            builder.push(act.name(), act.clone());
        }
    }
    let acts: Trie<u8, Act> = builder.build();
    let prompt: Cow<'static, str> = last_act
        .hotkey()
        .map(|hotkey| format!("{} ", hotkey).into())
        .unwrap_or("run_act".into());
    async move {
        match minibuffer.read(prompt, acts.clone()).await {
            // TODO: Get rid of clone.
            Ok(act_name) => match acts.resolve_res(&act_name) {
                Ok(act) => {
                    AsyncWorld::new().send_event(RunActEvent::new(act))?;
                }
                Err(e) => {
                    minibuffer.message(format!(
                        "Error: Could not resolve act named {:?}: {}",
                        act_name, e
                    ));
                }
            },
            Err(e) => {
                minibuffer.message(format!("Error: {e}"));
            }
        }
        Ok(())
    }
}

/// Input a key sequence. This will tell you what it does.
pub fn describe_key(
    acts: Query<&Act>,
    mut cache: ResMut<ActCache>,
    mut minibuffer: MinibufferAsync,
) -> impl Future<Output = Result<(), crate::Error>> {
    use trie_rs::inc_search::Answer;
    let trie: Trie<_, _> = cache.trie(acts.iter()).clone();
    async move {
        let mut search = trie.inc_search();
        let prompt = "Press key: ";
        let mut accum = String::new();

        loop {
            minibuffer.message(format!("{}{}", prompt, accum));
            let chord = minibuffer.get_chord().await?;
            match search.query(&chord) {
                Some(x) => {
                    let _ = write!(accum, "{} ", chord);
                    let v = search.value();
                    let msg = match x {
                        Answer::Match => format!("{}is bound to {}", accum, v.unwrap().name),
                        Answer::PrefixAndMatch => {
                            format!("{}is bound to {} and more", accum, v.unwrap().name)
                        }
                        Answer::Prefix => accum.clone(),
                    };
                    minibuffer.message(msg);
                    if matches!(x, Answer::Match) {
                        break;
                    }
                }
                None => {
                    let _ = write!(accum, "{} ", chord);
                    let msg = format!("{}is unbound", accum);
                    minibuffer.message(msg);
                    break;
                }
            }
        }
        Ok(())
    }
}
/// Bare minimum of acts for a useable and discoverable console
///
/// Key bindings may be altered or removed prior to adding this as a
/// plugin. Likewise acts may be altered or removed.
///
/// Although it is a [Plugin], if you use [App::add_plugins], the acts will not
/// be added. [ActBuilder] contains a non-cloneable that must be taken which
/// [Plugin::build] does not permit with its read-only `&self` access. Instead
/// use [AddActs::add_acts].
#[derive(Debug, Deref, DerefMut)]
pub struct BasicAsyncActs {
    /// Set of basic acts
    pub acts: Acts,
}

impl BasicAsyncActs {
    /// Substitute acts within [BasicActs].
    pub fn subtitute_async(mut self, basic_acts: &mut BasicActs) {
        let mut last_name = String::new();
        for (name, act) in self.take_acts().0.into_iter() {
            last_name.replace_range(.., &name);
            if basic_acts.insert(name, act).is_none() {
                warn!("Substitution of act '{}' not present.", last_name);
            }
        }
    }
}

impl Default for BasicAsyncActs {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                ActBuilder::new(run_act.pipe(future_result_sink))
                    .named("run_act")
                    .bind_aliased(keyseq! { Shift-; }, ":")
                    .bind(keyseq! { Alt-X })
                    .add_flags(ActFlags::Adverb)
                    .sub_flags(ActFlags::RunAct),
                ActBuilder::new(describe_key.pipe(future_result_sink))
                    .named("describe_key")
                    .bind(keyseq! { Ctrl-H K }),
            ]),
        }
    }
}

impl Plugin for BasicAsyncActs {
    fn build(&self, _app: &mut App) {
        self.warn_on_unused_acts();
    }
}

impl ActsPlugin for BasicAsyncActs {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}
