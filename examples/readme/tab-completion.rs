//! Ask user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer.read("What's your name? ",
                    vec!["John", "Sean", "Shane"])
        .observe(|mut trigger: Trigger<Submit<String>>| {
            info!("Hello, {}", trigger.event_mut().take_result().unwrap());
        });
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .add_systems(PostStartup, hello_name)
        .run();
}
