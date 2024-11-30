//! Add an act.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;
fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
}

fn plugin(app: &mut App) {
    app
        .add_plugins(MinibufferPlugins)
        .add_acts((Act::new(hello_world), BasicActs::default()));
}

fn main() {
    App::new()
        .add_plugins((common::VideoCapturePlugin::new("add-act")
                      .background(Srgba::hex("fb5607").unwrap()),
                      plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
