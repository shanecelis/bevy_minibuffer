//! Opt-in to basic acts.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts(BasicActs::default());
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("opt-in").background(Srgba::hex("ffbe0b").unwrap()),
            plugin,
        ))
        .add_systems(
            Startup,
            |mut commands: Commands, mut minibuffer: Minibuffer| {
                commands.spawn(Camera2d);
                minibuffer.message("Type 'Ctrl-H A' to see basic commands.");
                minibuffer.set_visible(true);
            },
        )
        .run();
}
