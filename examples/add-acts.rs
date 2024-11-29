//! Add a command.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_world() {
    info!("Hello, world");
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_acts(Act::new(hello_world))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
