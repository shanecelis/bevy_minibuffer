//! Add a command with a hotkey.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_world() {
    info!("Hello, world");
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_acts(Act::new(hello_world).hotkey(keyseq! { Ctrl-H }))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
