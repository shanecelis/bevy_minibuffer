//! Exclude all basic commands but list_acts.
//!
//! We keep the list_acts only because that is easiest way to show that acts
//! have been excluded. However, in practice if one were to remove all but one
//! command, one would probably keep exec_act.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
#[path = "common/lib.rs"]
mod common;

fn plugin(app: &mut App) {
    let mut basic_acts = BasicActs::default();
    /// Acts is a HashMap of act names and [ActBuilder]s.
    let mut acts = basic_acts.take_acts();
    let list_acts = acts.remove("list_acts").unwrap();
    app.add_plugins(MinibufferPlugins)
        .add_acts((basic_acts,
                   list_acts));
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("exclude-commands")
                .background(Srgba::hex("023047").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands, mut minibuffer: Minibuffer| {
            commands.spawn(Camera2dBundle::default());
            minibuffer.message("Type 'Ctrl-H A' to see only one command remains.");
            minibuffer.set_visible(true);
        })
        .run();
}
