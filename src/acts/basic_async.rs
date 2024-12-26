//! Bare minimum of acts for a useable and discoverable console
use crate::{
    acts::{basic::BasicActs, cache::{HotkeyActCache, NameActCache}, ActRef, ActFlags, ActsPlugin},
    event::LastRunAct,
    prelude::*,
    prelude::{keyseq, ActBuilder, Acts},
};

use crate::sink::future_result_sink;
use bevy::prelude::*;
use bevy_defer::AsyncWorld;
use futures::Future;
use std::{
    collections::HashMap,
    borrow::Cow,
    fmt::{Debug, Write},
};
use trie_rs::map::Trie;

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
pub fn run_act(
    mut minibuffer: MinibufferAsync,
    mut act_cache: ResMut<NameActCache>,
    mut acts: Query<(Entity, &Act)>,
    last_act: Res<LastRunAct>,
) -> impl Future<Output = Result<(), crate::Error>> {
    let acts_trie = act_cache.trie(acts.iter(),
                                   ActFlags::RunAct | ActFlags::Active).clone();
    let prompt: Cow<'static, str> = last_act
        .hotkey(&mut acts.transmute_lens::<&Act>())
        .map(|hotkey| {
            // We're hardcoding this little vim-ism. We feel slightly vandalous
            // _and_ good about it.
            if hotkey.alias.as_ref().map(|x| x == ":").unwrap_or(false) {
                // All it does is remove the space after the prompt.
                ":".into()
            } else {
                format!("{} ", hotkey).into()
            }
        })
        .unwrap_or("run_act: ".into());
    async move {
        match minibuffer.prompt_map(prompt, acts_trie).await {
            // TODO: Get rid of clone.
            Ok(act_ref) => {
                AsyncWorld::new().send_event(RunActEvent::new(act_ref))?;
            }
            Err(e) => {
                minibuffer.message(format!("Error: {e}"));
            }
        }
        Ok(())
    }
}

/// Input a key sequence. This will tell you what it does.
pub fn describe_key(
    acts: Query<(Entity, &Act)>,
    mut cache: ResMut<HotkeyActCache>,
    mut minibuffer: MinibufferAsync,
) -> impl Future<Output = Result<(), crate::Error>> {
    use trie_rs::inc_search::Answer;
    let act_names: HashMap<Entity, Cow<'static, str>> = acts.iter().map(|(id, act)| (id, act.name.clone())).collect();
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
                    let name: Option<&Cow<'static, str>> = search.value().and_then(|act_ref: &ActRef| act_names.get(&act_ref.id));
                    let msg = match x {
                        Answer::Match => format!("{}is bound to {}", accum, name.as_deref().unwrap_or(&"???".into())),
                        Answer::PrefixAndMatch => {
                            format!("{}is bound to {} and more", accum, name.as_deref().unwrap_or(&"???".into()))
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
