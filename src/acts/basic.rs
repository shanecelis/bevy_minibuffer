//! Bare minimum of acts for a useable and discoverable console
use crate::{
    acts::{ActCache, ActFlags, ActsPlugin},
    autocomplete::LookupMap,
    event::LastRunAct,
    input::{Hotkey, KeyChord},
    prelude::*,
    prelude::{keyseq, ActBuilder, Acts},
    prompt::{CompletionState, PromptState},
    Minibuffer,
};

use std::{borrow::Cow, fmt::Debug};

use crate::{autocomplete::RequireMatch, prompt::KeyChordEvent};
use bevy::{prelude::*, window::RequestRedraw};
use tabular::{Row, Table};
use trie_rs::inc_search::IncSearch;
use trie_rs::map::{Trie, TrieBuilder};

/// Execute an act by name. Similar to Emacs' `M-x` or vim's `:` key binding.
pub fn run_act(mut minibuffer: Minibuffer, acts: Query<&Act>, last_act: Res<LastRunAct>) {
    let mut builder = TrieBuilder::new();
    for act in acts.iter() {
        if act.flags.contains(ActFlags::RunAct | ActFlags::Active) {
            builder.push(act.name(), act.clone());
        }
    }
    let acts: Trie<u8, Act> = builder.build();
    let prompt: Cow<'static, str> = last_act
        .hotkey()
        .map(|hotkey| {
            // We're hardcoding this little vim-ism. We feel slightly vandalous
            // _and_ good about it.
            if hotkey.alias.as_ref().map(|x| x == ":").unwrap_or(false) {
                ":".into()
            } else {
                format!("{} ", hotkey).into()
            }
        })
        .unwrap_or("run_act".into());
    minibuffer
        .prompt_lookup(prompt, acts.clone())
        .insert(RequireMatch)
        .observe(
            move |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| match trigger
                .event_mut()
                .take_result()
            {
                Ok(act_name) => match acts.resolve_res(&act_name) {
                    Ok(act) => {
                        minibuffer.run_act(act);
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
            },
        );
}

/// List acts currently operant.
pub fn list_acts(acts: Query<&Act>) -> String {
    let mut table = Table::new("{:<}  {:<}");
    table.add_row(Row::new().with_cell("ACT ").with_cell("KEY BINDING"));
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
            let bindings = act.hotkeys.iter().map(|hotkey| hotkey.to_string());

            for binding in bindings {
                table.add_row(
                    Row::new()
                        .with_cell(name.take().unwrap_or(""))
                        .with_cell(binding),
                );
            }
        }
    }
    format!("{}", table)
}

/// Can pipe any string to the message buffer.
///
/// The minibuffer might not be visible when this is called. Consider adding
/// [ActFlags::ShowMinibuffer] to the act's flags to ensure it will be shown.
///
/// Used internally by `list_acts` for instance
///
/// ```ignore
/// ActBuilder::new(list_acts.pipe(to_message))
///     .named("list_acts")
///     .add_flags(ActFlags::ShowMinibuffer)
///     .hotkey(keyseq! { Ctrl-H A }),
/// ```
pub fn to_message(In(msg): In<String>, mut minibuffer: Minibuffer) {
    minibuffer.message(msg);
}

/// List key bindings available.
pub fn list_key_bindings(acts: Query<&Act>) -> String {
    let mut table = Table::new("{:<}  {:<}");
    table.add_row(Row::new().with_cell("KEY BINDING ").with_cell("ACT"));

    let mut key_bindings: Vec<(String, Cow<'static, str>)> = acts
        .iter()
        .flat_map(|act| {
            act.hotkeys
                .iter()
                .map(|hotkey| (hotkey.to_string(), act.name.clone()))
        })
        .collect();
    // Sort by key binding name? No.
    // key_bindings.sort_by(|a, b| a.0.cmp(&b.0));
    // Sort by act name? Yes.
    key_bindings.sort_by(|a, b| a.1.cmp(&b.1));
    for (binding, act) in key_bindings
        .into_iter()
        // Don't show same act name in a row. Replace the same named items with
        // an empty string. It's an implicit ibid.
        .scan(Cow::from(""), |last, (bind, act)| {
            if *last == act {
                *last = act.clone();
                Some((bind, Cow::from("")))
            } else {
                *last = act.clone();
                Some((bind, act))
            }
        })
    {
        table.add_row(Row::new().with_cell(binding).with_cell(act.into_owned()));
    }
    format!("{}", table)
}

/// Toggle visibility.
#[allow(private_interfaces)]
pub fn toggle_visibility(
    mut redraw: EventWriter<RequestRedraw>,
    prompt_state: Res<State<PromptState>>,
    completion_state: Res<State<CompletionState>>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut last_completion_state: Local<CompletionState>,
) {
    match **prompt_state {
        PromptState::Invisible => {
            next_completion_state.set(*last_completion_state);
            next_prompt_state.set(PromptState::Visible);
            redraw.send(RequestRedraw);
        }
        PromptState::Visible => {
            next_completion_state.set(CompletionState::Invisible);
            next_prompt_state.set(PromptState::Invisible);
            redraw.send(RequestRedraw);
            *last_completion_state = **completion_state;
        }
    }
}

/// Reveal act for inputted key chord sequence.
///
/// Allow the user to input a key chord sequence. Reveal the bindings it has.
pub fn describe_key(acts: Query<&Act>, mut cache: ResMut<ActCache>, mut minibuffer: Minibuffer) {
    let trie: Trie<_, _> = cache.trie(acts.iter()).clone();
    let mut position = trie.inc_search().into();
    // search
    let mut accum = Hotkey::empty();

    minibuffer.message("Press key: ");
    minibuffer.get_chord().observe(
        move |mut trigger: Trigger<KeyChordEvent>,
              mut commands: Commands,
              mut minibuffer: Minibuffer| {
            use trie_rs::inc_search::Answer;
            let mut search = IncSearch::resume(&trie, position);
            let chord: KeyChord = trigger.event_mut().take().expect("key chord");
            match search.query(&chord) {
                Some(x) => {
                    // let _ = write!(accum, "{} ", chord);
                    accum.chords.push(chord);
                    let v = search.value();
                    let msg = match x {
                        Answer::Match => {
                            let act = v.expect("act");
                            // Use the hotkey's alias if available.
                            let binding = act.find_hotkey(&accum.chords).unwrap_or(&accum);
                            format!("{} is bound to {}", binding, act.name)
                        }
                        Answer::PrefixAndMatch => {
                            let act = v.expect("act");
                            // Use the hotkey's alias if available.
                            let binding = act.find_hotkey(&accum.chords).unwrap_or(&accum);
                            format!("{} is bound to {} and more", binding, act.name)
                        }
                        Answer::Prefix => format!("Press key: {}", &accum),
                    };
                    minibuffer.message(msg);
                    if matches!(x, Answer::Match) {
                        commands.entity(trigger.entity()).despawn_recursive();
                        // break;
                    }
                }
                None => {
                    accum.chords.push(chord);
                    let msg = format!("{} is unbound", &accum);
                    minibuffer.message(msg);
                    commands.entity(trigger.entity()).despawn_recursive();
                    // break;
                }
            }
            position = search.into();
        },
    );
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
pub struct BasicActs {
    /// Set of basic acts
    pub acts: Acts,
}

impl Default for BasicActs {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                ActBuilder::new(list_acts.pipe(to_message))
                    .named("list_acts")
                    .add_flags(ActFlags::ShowMinibuffer)
                    .bind(keyseq! { Ctrl-H A }),
                ActBuilder::new(list_key_bindings.pipe(to_message))
                    .named("list_key_bindings")
                    .add_flags(ActFlags::ShowMinibuffer)
                    .bind(keyseq! { Ctrl-H B }),
                ActBuilder::new(toggle_visibility)
                    .named("toggle_visibility")
                    .bind(keyseq! { Backquote })
                    .sub_flags(ActFlags::RunAct),
                ActBuilder::new(run_act)
                    .named("run_act")
                    .bind_aliased(keyseq! { Shift-; }, ":")
                    .bind(keyseq! { Alt-X })
                    .add_flags(ActFlags::Adverb)
                    .sub_flags(ActFlags::RunAct),
                ActBuilder::new(describe_key)
                    .named("describe_key")
                    .bind(keyseq! { Ctrl-H K }),
            ]),
        }
    }
}

impl BasicActs {
    /// Make run_act look like 'M-x ' at the prompt.
    pub fn emacs() -> Self {
        let mut basic = Self::default();
        let run_act = basic.get_mut("run_act").unwrap();
        run_act.hotkeys.clear();
        run_act.bind_aliased(keyseq! { Alt-X }, "M-x ");
        basic
    }
}

impl Plugin for BasicActs {
    fn build(&self, _app: &mut App) {
        self.warn_on_unused_acts();
    }
}

impl ActsPlugin for BasicActs {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}
