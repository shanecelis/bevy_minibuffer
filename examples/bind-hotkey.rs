//! Bind an act to a hotkey.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
#[path = "common/lib.rs"]
mod common;

fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, World!");
    minibuffer.set_visible(true);
}

fn plugin(app: &mut App) {
    app
        .add_plugins(MinibufferPlugins)
        .add_acts((Act::new(hello_world).bind(keyseq! { Ctrl-W }),
                  BasicActs::default()));
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((common::VideoCapturePlugin::new("bind-hotkey")
                      .background(Srgba::hex("ff006e").unwrap()),
                      plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
