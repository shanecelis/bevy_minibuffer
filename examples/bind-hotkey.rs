//! Add an act with a hotkey.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn hello_world(mut minibuffer: Minibuffer) {
    minibuffer.message("Hello, world");
    minibuffer.set_visible(true);
}

fn plugin(app: &mut App) {
    app.add_acts(Act::new(hello_world).bind(keyseq! { Ctrl-H }));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins, plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
