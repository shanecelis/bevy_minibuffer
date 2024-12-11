//! Demonstrate the async tab completers: vec, hash-map, trie, and trie-map.
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
use trie_rs::map::Trie;
use std::{
    collections::HashMap,
    process::ExitCode,
};

#[path = "../common/lib.rs"]
mod common;

async fn hello_name_vec(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let v = vec!["John", "Sean", "Shane"];
    let name = minibuffer.prompt_lookup("What's your name? ", v).await?;
    minibuffer.message(format!("Hello, {}.", name));
    Ok(())
}

async fn hello_name_trie(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let trie = trie_rs::Trie::from_iter(["John", "Sean", "Shane"]);
    let name = minibuffer.prompt_lookup("What's your name? ", trie).await?;
    minibuffer.message(format!("Hello, {}.", name));
    Ok(())
}

#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare,
}

async fn hello_name_hash_map(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let map = HashMap::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    let result = minibuffer.prompt_map("What's your name? ", map).await;
    minibuffer.message(match result {
        Ok(popular) => format!("That's a {:?} name.", popular),
        _ => "I don't know what kind of name that is.".to_string(),
    });
    Ok(())
}

async fn hello_name_trie_map(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let trie = Trie::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    let result = minibuffer.prompt_map("What's your name? ", trie).await;
    minibuffer.message(match result {
        Ok(popular) => format!("That's a {:?} name.", popular),
        _ => "I don't know what kind of name that is.".to_string(),
    });
    Ok(())
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts((BasicActs::default(),
                   // Bare systems can be passed in if there's no configuration
                   // for the act that's necessary.
                   //
                   // Act::new(hello_name_vec),
                   (hello_name_vec.pipe(future_result_sink)),
                   (hello_name_hash_map.pipe(future_result_sink)),
                   (hello_name_trie.pipe(future_result_sink)),
                   (hello_name_trie_map.pipe(future_result_sink)),
                  ));
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let argument: Option<String> = args.next();
    let is_help = argument.as_ref().map(|arg| arg == "-h" || arg == "--help");
    if  is_help.unwrap_or(false) || args.next().is_some() {
        eprintln!("usage: tab-completion <vec, hash-map, trie, trie-map>");
        return ExitCode::from(2);
    }

    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion-async")
                .background(Srgba::hex("3a86ff").unwrap()),
            plugin,
        ))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Startup, (move || argument.clone()).pipe(choose_completion.pipe(future_result_sink)))
        .run();
    ExitCode::SUCCESS
}

const OPTIONS: [(&'static str, &'static str); 4] = [
    ("vec (simple)", "hello_name_vec"),
    ("hash-map (maps to a value V)", "hello_name_hash_map"),
    ("trie (performant)", "hello_name_trie"),
    ("trie-map (performant)", "hello_name_trie_map"),
];

async fn choose_completion(In(arg): In<Option<String>>, mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    if let Some(arg) = arg {
        if let Some(act_name) = OPTIONS.iter().find(|x| x.0.split_whitespace().next().map(|y| y == arg).unwrap_or(false)).map(|x| x.1) {

            minibuffer.run_act(act_name);
        } else {
            eprintln!("No act for that argument.");
            std::process::exit(1);
        }
    } else {
        let index: usize = minibuffer
            .prompt_with::<RadioGroup>("Choose a completion kind: ", |parent| {
                parent.prompt_children::<Radio>(OPTIONS.iter().map(|x| x.0));
            }).await?;
        if let Some(tuple) = OPTIONS.get(index) {
            minibuffer.run_act(tuple.1);
        } else {
            unreachable!();
        }
    }
    Ok(())
}
