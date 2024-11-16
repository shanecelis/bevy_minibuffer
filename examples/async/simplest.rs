use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "../common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt::<TextField>("What's your first name? ")
        .await?;
    let last_name = minibuffer
        .prompt::<TextField>("What's your last name? ")
        .await?;
    minibuffer.message(format!("Hello, {first_name} {last_name}!"));
    Ok(())
}

fn main() {
    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Simplest Async Example".into(),
    };
    App::new()
        // .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_plugins((
            DefaultPlugins.set(video_settings.window_plugin()),
            MinibufferPlugins.set(video_settings.minibuffer_plugin()),
        ))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add(Act::new(ask_name.pipe(future_result_sink)).hotkey(keyseq! { ctrl-A N }));
}
