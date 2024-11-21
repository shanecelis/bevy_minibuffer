use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use bevy_minibuffer::universal::UniversalPlugin;
use std::time::Duration;
#[path = "../common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
async fn demo(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let beat = Duration::from_secs_f32(2.0);
    let yes = minibuffer.prompt::<Confirm>("Want to see something cool?").await?;

    minibuffer.message(if yes { "Oh, good!" } else { "Oh, nevermind." });
    let _ = minibuffer.delay_or_chord(beat).await;
    if ! yes {
        return Ok(());
    }

    let lang = minibuffer.prompt_group::<Radio>(
                    "Which do you prefer?",
                    ["brainfuck", "rust", "x86 machine code"]).await?;
    minibuffer.message(if lang == 1 { "Me too!" } else { "More power to you." });
    let _ = minibuffer.delay_or_chord(beat).await;

    let selection = minibuffer.prompt_group::<Checkbox>(
                    "What engines do you use?",
                    ["Unity", "Unreal", "Godot", "bevy", "other"]).await?;

    minibuffer.message(if selection[3] { "Well, have I got news for you!" }
                       else if !selection.iter().any(|x| *x) { "This may not interest you then." }
                       else { "Those are also great." });
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Minibuffer works for bevy now!");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("So...");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.prompt::<Confirm>("Let's sign you up on our email list.").await;
    minibuffer.prompt::<TextField>("What's your email? ").await;
    if let Ok(p) = minibuffer.prompt::<Password>("I'm gonna need your password too. ").await {
        minibuffer.message("Heh heh.");
    } else {
        minibuffer.message("Please, I need it for real.");
    }
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Just kidding.");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("I don't NEED your password.");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("I just wanted it for REASONS.");
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
        .add_systems(PostStartup, demo.pipe(future_result_sink))
        .run();
}
