//! Demonstrate the tab completers: vec, hash-map, trie, and trie-map.
//!
//! Ask the user a question with tab completion.
//!
//! Unlike most bevy apps, this one accepts command line arguments. Without an
//! argument, it will ask the user what kind of completer they want: vec,
//! hash-map, trie, or trie-map.
//!
//! This can be provided on the command line as the first argument like so:
//!
//! ```sh
//! cargo run --example tab-completer -- vec
//! ```
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use std::{collections::HashMap, process::ExitCode};

#[path = "common/lib.rs"]
mod common;

fn hello_name_vec(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt_lookup("What's your name? ", vec!["John", "Sean", "Shane"])
        .observe(
            |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                minibuffer.message(format!(
                    "Hello, {}.",
                    trigger.event_mut().take_result().unwrap()
                ));
            },
        );
}

fn hello_name_trie(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt_lookup(
            "What's your name? ",
            trie_rs::Trie::from_iter(["John", "Sean", "Shane"]),
        )
        .observe(
            |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                minibuffer.message(format!(
                    "Hello, {}.",
                    trigger.event_mut().take_result().unwrap()
                ));
            },
        );
}

#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare,
}

fn hello_name_hash_map(mut minibuffer: Minibuffer) {
    let map = HashMap::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    minibuffer.prompt_map("What's your name? ", map).observe(
        |mut trigger: Trigger<Completed<Popular>>, mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result().unwrap();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's a {:?} name.", popular),
                _ => "I don't know what kind of name that is.".to_string(),
            });
        },
    );
}

fn hello_name_trie_map(mut minibuffer: Minibuffer) {
    let trie = trie_rs::map::Trie::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    minibuffer.prompt_map("What's your name? ", trie).observe(
        |mut trigger: Trigger<Completed<Popular>>, mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result().unwrap();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's a {:?} name.", popular),
                _ => "I don't know what kind of name that is.".to_string(),
            });
        },
    );
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        // Bare systems can be passed in if there's no configuration
        // for the act that's necessary.
        //
        // Act::new(hello_name_vec),
        hello_name_vec,
        hello_name_hash_map,
        hello_name_trie,
        hello_name_trie_map,
    ));
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let argument: Option<String> = args.next();
    let is_help = argument.as_ref().map(|arg| arg == "-h" || arg == "--help");
    if is_help.unwrap_or(false) || args.next().is_some() {
        eprintln!("usage: tab-completion <vec, hash-map, trie, trie-map>");
        return ExitCode::from(2);
    }

    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion")
                .background(Srgba::hex("3a86ff").unwrap()),
            plugin,
        ))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Startup, (move || argument.clone()).pipe(choose_completion))
        .run();
    ExitCode::SUCCESS
}

const OPTIONS: [(&str, &str); 4] = [
    ("vec (simple)", "hello_name_vec"),
    ("hash-map (maps to a value V)", "hello_name_hash_map"),
    ("trie (performant)", "hello_name_trie"),
    ("trie-map (performant)", "hello_name_trie_map"),
];

fn choose_completion(In(arg): In<Option<String>>, mut minibuffer: Minibuffer) {
    if let Some(arg) = arg {
        if let Some(act_name) = OPTIONS
            .iter()
            .find(|x| {
                x.0.split_whitespace()
                    .next()
                    .map(|y| y == arg)
                    .unwrap_or(false)
            })
            .map(|x| x.1)
        {
            minibuffer.run_act(act_name);
        } else {
            eprintln!("No act for that argument.");
            std::process::exit(1);
        }
    } else {
        minibuffer
            .prompt::<RadioGroup>("Choose a completion kind: ")
            .prompt_children::<Radio>(OPTIONS.iter().map(|x| x.0))
            .observe(
                move |mut trigger: Trigger<Submit<usize>>, mut minibuffer: Minibuffer, mut commands: Commands| {
                    if let Ok(index) = trigger.event_mut().take_result() {
                        minibuffer.run_act(OPTIONS[index].1);
                    } else {
                        commands.entity(trigger.target()).despawn_recursive();
                    }
                },
            );
    }
}
