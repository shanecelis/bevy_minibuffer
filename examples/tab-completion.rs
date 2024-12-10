//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use std::collections::HashMap;

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

// fn choose_completion(mut minibuffer: Minibuffer) {
//     let trie = trie_rs::map::Trie::from_iter([
//         ("vec", "hello_name_vec"),
//         ("hash map", "hello_name_hash_map"),
//         ("trie", "hello_name_trie"),
//         ("trie map", "hello_name_trie_map"),
//     ]);
//     minibuffer.prompt_map("Choose your completion: ", trie).observe(
//         |mut trigger: Trigger<Completed<&'static str>>, mut minibuffer: Minibuffer| {
//             if let Ok(act_name) = trigger.event_mut().take_result().unwrap() {
//                 minibuffer.run_act(act_name);
//             }
//         },
//     );
// }
//
fn choose_completion(mut minibuffer: Minibuffer) {
    let options = vec![
        ("vec", "hello_name_vec"),
        ("hash map", "hello_name_hash_map"),
        ("trie", "hello_name_trie"),
        ("trie map", "hello_name_trie_map"),
    ];

    minibuffer
        .prompt::<RadioGroup>("Which do you prefer?")
        .prompt_children::<Radio>(options.iter().map(|x| x.0))
        .observe(
        move |mut trigger: Trigger<Submit<usize>>, mut minibuffer: Minibuffer| {
            if let Ok(index) = trigger.event_mut().take_result() {
                minibuffer.run_act(options[index].1);
            }
        },
    );
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts((BasicActs::default(),
                   Act::new(hello_name_vec),
                   Act::new(hello_name_hash_map),
                   Act::new(hello_name_trie),
                   Act::new(hello_name_trie_map),
                  ))
        .add_systems(Startup, choose_completion);
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion")
                .background(Srgba::hex("3a86ff").unwrap()),
            plugin,
        ))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
