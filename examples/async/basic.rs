use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use std::time::Duration;
#[path = "../common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt::<TextField>("What's your first name?")
        .await?;
    let last_name = minibuffer
        .prompt::<TextField>("What's your last name?")
        .await?;
    minibuffer.message(format!("Hello, {first_name} {last_name}!"));
    Ok(())
}

// Ask the user for their age.
async fn ask_age(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let age = minibuffer.prompt::<Number<u8>>("What's your age? ").await?;
    // minibuffer.delay(Duration::from_secs(2)).await;
    minibuffer.message(format!("You are {age} years old."));
    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Basic Example".into(),
    };
    App::new()
        // .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_plugins((
            DefaultPlugins.set(video_settings.window_plugin()),
            MinibufferPlugins.set(video_settings.minibuffer_plugin()),
        ))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        // .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_systems(Startup, setup)
        .add_acts((
            // Add builtin commands.
            Builtin::default(),
            // Add universal argument commands.
            UniversalArgPlugin::default(),
            Act::new(ask_name.pipe(future_result_sink))
                .named("ask_name")
                .hotkey(keyseq!(Ctrl-A N)),
            Act::new(ask_age.pipe(future_result_sink))
                .named("ask_age")
                .hotkey(keyseq! { Ctrl-A A }),
            ))
        .run();
}
