//! Ask the user a question.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

fn hello_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your name? ")
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
    app.add_plugins(MinibufferPlugins)
        // .add_acts(BasicActs::default())
        .add_systems(Startup, hello_name);
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("solicit-user")
                .background(Srgba::hex("8338ec").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
