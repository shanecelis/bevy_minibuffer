//! Ask the user a question with tab completion.
//!
//! Uses a trie for performance.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use trie_rs::Trie;
#[path = "common/lib.rs"]
mod common;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt_lookup(
            "What's your name? ",
            Trie::from_iter(["John", "Sean", "Shane"]),
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

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts(Act::new(hello_name).bind(keyseq! { Space }))
        .add_systems(Startup, hello_name);
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion-trie")
                .background(Srgba::hex("aea4bf").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
