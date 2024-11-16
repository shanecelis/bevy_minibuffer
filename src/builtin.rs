use crate::{
    autocomplete::AutoComplete,
    prelude::*,
    lookup::Resolve,
    Minibuffer,
    Message,
    prompt::{CompletionState, PromptState},
    act::{self, PluginOnce, ActFlags},
    prelude::{keyseq, ActBuilder, ActsPlugin},
};

use std::{
    // cell::RefCell,
    sync::Mutex,
    borrow::Cow,
    fmt::{self, Debug, Display, Write},
    future::Future,
};

use bevy::{
    ecs::system::{BoxedSystem, SystemId},
    prelude::*,
    window::RequestRedraw,
};
use tabular::{Row, Table};
use trie_rs::map::{Trie, TrieBuilder};
#[cfg(feature = "async")]
use crate::{future_sink, future_result_sink};
use bevy::{prelude::*, ecs::system::IntoSystem};

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
#[cfg(feature = "async")]
pub fn exec_act(
    mut minibuffer: Minibuffer,
    acts: Query<&Act>,
) -> impl Future<Output = Result<(), crate::Error>> {
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        if act.flags.contains(ActFlags::ExecAct | ActFlags::Active) {
            builder.push(act.name(), act.clone());
        }
    }
    let acts: Trie<u8, Act> = builder.build();
    async move {
        todo!();
        // match minibuffer.read(":".to_string(), acts.clone()).await {
        //     // TODO: Get rid of clone.
        //     Ok(act_name) => match acts.resolve(&act_name) {
        //         Ok(act) => {
        //             AsyncWorld::new().send_event(RunActEvent(act))?;
        //         }
        //         Err(e) => {
        //             minibuffer.prompt(Message::new(format!(
        //                 "Error: Could not resolve act named {:?}: {}",
        //                 act_name, e
        //             )))
        //             .await?;
        //         }
        //     },
        //     Err(e) => {
        //         minibuffer.prompt(Message::new(format!("Error: {e}"))).await?;
        //     }
        // }
        Ok(())
    }
}

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
#[cfg(not(feature = "async"))]
pub fn exec_act(
    mut minibuffer: Minibuffer,
    acts: Query<&Act>,
) {
    eprintln!("here");
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        if act.flags.contains(ActFlags::ExecAct | ActFlags::Active) {
            builder.push(act.name(), act.clone());
        }
    }
    let acts: Trie<u8, Act> = builder.build();
    minibuffer.read(":", acts.clone())
        .observe(move |trigger: Trigger<AskyEvent<String>>,
                 // query: Query<&AutoComplete>,
                 mut writer: EventWriter<RunActEvent>,
                 mut minibuffer: Minibuffer| {
            // let autocomplete = query.get(trigger.entity()).unwrap();
                // let act_name = trigger.event().0.unwrap().cloned();
            match &trigger.event().0 {
                Ok(act_name) => match acts.resolve(&act_name) {
                    Ok(act) => {
                        writer.send(RunActEvent(act));
                    }
                    Err(e) => {
                        minibuffer.message(format!(
                            "Error: Could not resolve act named {:?}: {}",
                            act_name, e));
                    }
                },
                Err(e) => {
                    minibuffer.message(format!("Error: {e}"));
                }
            }
        });
}

/// List acts currently operant.
pub fn list_acts(mut asky: Minibuffer, acts: Query<&Act>) {
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new().with_cell("ACT").with_cell("KEY BINDING"));
    let mut acts: Vec<_> = acts.iter().collect();
    acts.sort_by(|a, b| a.name().cmp(b.name()));
    for act in &acts {
        let mut name = Some(act.name());
        if act.hotkeys.is_empty() {
            table.add_row(
                Row::new()
                    .with_cell(name.take().unwrap_or(""))
                    .with_cell(""),
            );
        } else {
            let bindings = act.hotkeys.iter().map(|chords| {
                chords.iter().fold(String::new(), |mut output, chord| {
                    let _ = write!(output, "{} ", chord);
                    output
                })
            });

            for binding in bindings {
                table.add_row(
                    Row::new()
                        .with_cell(name.take().unwrap_or(""))
                        .with_cell(binding),
                );
            }
        }
    }
    let msg = format!("{}", table);
    asky.message(msg);
}

/// List key bindings available.
pub fn list_key_bindings(mut asky: Minibuffer, acts: Query<&Act>) {
    let mut table = Table::new("{:<}\t{:<}");
    table.add_row(Row::new().with_cell("KEY BINDING ").with_cell("ACT"));

    let mut key_bindings: Vec<(String, Cow<'static, str>)> = acts
        .iter()
        .flat_map(|act| {
            act.hotkeys.iter().map(|hotkey| {
                let binding = hotkey.iter().fold(String::new(), |mut output, chord| {
                    let _ = write!(output, "{} ", chord);
                    output
                });
                (binding, act.name.clone())
            })
        })
        .collect();
    // Sort by key binding name? No.
    // key_bindings.sort_by(|a, b| a.0.cmp(&b.0));
    // Sort by act name? Yes.
    key_bindings.sort_by(|a, b| a.1.cmp(&b.1));
    for (binding, act) in key_bindings.into_iter()
        // Don't show some act name in a row. Replace the same named items with
        // an empty string. It's an implicit ibid.
        .scan(Cow::from(""), |last, (bind, act)| {
            if *last == act {
                *last = act.clone();
                Some((bind, Cow::from("")))
            } else {
                *last = act.clone();
                Some((bind, act))
            }
        }) {
        table.add_row(Row::new().with_cell(binding).with_cell(act.into_owned()));
    }
    let msg = format!("{}", table);
    asky.message(msg);
}

/// Toggle visibility.
pub fn toggle_visibility(
    mut redraw: EventWriter<RequestRedraw>,
    prompt_state: Res<State<PromptState>>,
    completion_state: Res<State<CompletionState>>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
) {
    match (**prompt_state, **completion_state) {
        (PromptState::Invisible, CompletionState::Invisible) => {
            next_prompt_state.set(PromptState::Visible);
            next_completion_state.set(CompletionState::Visible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Visible, CompletionState::Visible) => {
            next_prompt_state.set(PromptState::Invisible);
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Invisible, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Visible, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
        (PromptState::Finished, _) => {
            next_completion_state.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
        }
    }
}

/// Input a key sequence. This will tell you what it does.
#[cfg(feature = "async")]
pub fn describe_key(
    acts: Query<&Act>,
    mut cache: ResMut<ActCache>,
    mut minibuffer: MinibufferAsync,
) -> impl Future<Output = Result<(), crate::Error>> {
    use trie_rs::inc_search::Answer;
    let trie: Trie<_, _> = cache.trie(acts.iter()).clone();
    async move {
        let mut search = trie.inc_search();
        let mut accum = String::from("Press key: ");

        loop {
            minibuffer.message(accum.clone());
            let chord = minibuffer.get_chord().await?;
            match search.query(&chord) {
                Some(x) => {
                    let _ = write!(accum, "{} ", chord);
                    let v = search.value();
                    let msg = match x {
                        Answer::Match => format!("{}is bound to {:?}", accum, v.unwrap().name),
                        Answer::PrefixAndMatch => {
                            format!("{}is bound to {:?} and more", accum, v.unwrap().name)
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


/// Builtin acts: exec_act, list_acts, list_key_bindings, describe_key.
pub struct Builtin {
    /// Set of builtin acts
    pub acts: ActsPlugin
}

impl Default for Builtin {
    fn default() -> Self {
        Self {
            acts:
            ActsPlugin::new([
#[cfg(feature = "async")]
                ActBuilder::new(exec_act.pipe(future_result_sink))
                    .named("exec_act")
                    .hotkey(keyseq! { shift-; })
                    .hotkey(keyseq! { alt-X })
                    .in_exec_act(false),

#[cfg(not(feature = "async"))]
                ActBuilder::new(exec_act)
                    .named("exec_act")
                    .hotkey(keyseq! { shift-; })
                    .hotkey(keyseq! { alt-X })
                    .in_exec_act(false),
                ActBuilder::new(list_acts)
                    .named("list_acts")
                    .hotkey(keyseq! { ctrl-H A }),
                ActBuilder::new(list_key_bindings)
                    .named("list_key_bindings")
                    .hotkey(keyseq! { ctrl-H B }),
#[cfg(feature = "async")]
                ActBuilder::new(describe_key.pipe(future_result_sink))
                    .named("describe_key")
                    .hotkey(keyseq! { ctrl-H K })
            ])
        }
    }
}

impl PluginOnce for Builtin {
    fn build(self, app: &mut App) {
        self.acts.build(app);
    }
}
