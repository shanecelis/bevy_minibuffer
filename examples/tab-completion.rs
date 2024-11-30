//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer
        .read("What's your name? ", vec!["John", "Sean", "Shane"])
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
    app
        .add_plugins(MinibufferPlugins)
        .add_systems(PostStartup, hello_name);
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((common::VideoCapturePlugin::new("tab-completion")
                      .background(Srgba::hex("3a86ff").unwrap()),
                      plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
