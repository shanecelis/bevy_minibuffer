use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use bevy_minibuffer::universal::UniversalPlugin;
use std::time::Duration;
#[path = "../common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
async fn demo(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let yes = minibuffer.prompt::<Confirm>("Want to see something cool?").await?;
    let beat = Duration::from_secs_f32(2.0);

    minibuffer.message(if yes { "Oh, good!" } else { "Oh, nevermind." });
    minibuffer.delay(beat);
    if ! yes {
        return Ok(());
    }
    let index = minibuffer.prompt::<RadioGroup>(
                    // "Which do you prefer?".to_string(),
                    ["brainfuck", "rust", "x86 machine code"]).await?;
    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Add acts using [Commands].
fn add_acts(mut commands: Commands) {
    commands.add(
        Act::new(demo.pipe(future_result_sink))
            .hotkey(keyseq!(D)),
    );
}
fn main() {
    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Demo Example".into(),
    };
    App::new()
        // .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_plugins((
            DefaultPlugins.set(video_settings.window_plugin()),
            MinibufferPlugins.set(video_settings.minibuffer_plugin()),
        ))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        // .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_plugins(UniversalPlugin::default().into_plugin())
        // Add builtin commands.
        .add_plugins(Builtin::default().into_plugin())
        .add_systems(Startup, (setup, add_acts))
        // .add_systems(PostStartup, demo.pipe(future_result_sink))
        .run();
}
