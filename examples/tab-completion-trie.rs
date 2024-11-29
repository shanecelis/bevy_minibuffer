//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use trie_rs::Trie;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.read("What's your name? ",
                    Trie::from_iter(["John", "Sean", "Shane"]))
        .observe(|mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
            minibuffer.message(format!("Hello, {}.", trigger.event_mut().take_result().unwrap()));
        });
}

fn plugin(app: &mut App) {
    app
        .add_systems(PostStartup, hello_name);
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins, plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
