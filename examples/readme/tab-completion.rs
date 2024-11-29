//! Ask user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_name(mut minibuffer: Minibuffer) {
    let names = vec!["John", "Sean", "Shane"];
    minibuffer.read("What's your name? ",
                    names)
        .observe(|trigger: Trigger<AskyEvent<String>>| {
            info!("Hello, {}", trigger.event().as_ref().clone().unwrap());
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
