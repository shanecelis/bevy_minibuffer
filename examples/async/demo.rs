use bevy::prelude::*;
use bevy_minibuffer::prelude::{*, Error};
use bevy_asky::prelude::*;
use std::time::Duration;
#[path = "../common/lib.rs"]
mod common;

/// Demo some of Minibuffer's prompts.
async fn demo(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let beat = Duration::from_secs_f32(2.0);
    let yes: bool = minibuffer.prompt::<Confirm>("Hey, psst. Want to see something cool?").await?;
    minibuffer.message(if yes { "Oh, good!" } else { "Oh, ok. Hit 'D' if you change your mind." });
    if ! yes {
        return Ok(());
    }
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("First a few questions.");
    let _ = minibuffer.delay_or_chord(beat).await;
    let lang: usize = minibuffer.prompt_with::<RadioGroup>("Which do you prefer?", |commands| {
        commands.prompt_children::<Radio>(["brainf*ck", "rust", "x86 machine code"]);
    }).await?;
    minibuffer.message(if lang == 1 { "Me too!" } else { "More power to you." });
    let _ = minibuffer.delay_or_chord(beat).await;
    let selection: Vec<bool> = minibuffer
        .prompt_with::<CheckboxGroup>("What game engines do you use?", |commands| {
            commands.prompt_children::<Checkbox>(["Unity", "Unreal", "Godot", "Bevy", "other"]);
        }).await?;
    minibuffer.message(if selection[3] { "Well, have I got news for you!" }
                       else if !selection.iter().any(|x| *x) { "This may not interest you then." }
                       else { "Those are also great." });
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Minibuffer works for bevy now!");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("So...");
    let _ = minibuffer.delay_or_chord(beat).await;
    let signup = minibuffer.prompt::<Confirm>("Let's sign you up on our email list.").await?;
    let email: Result<String, Error> = minibuffer.prompt::<TextField>("What's your email? ").await;
    if email.is_err() {
        minibuffer.message("canceled?");
        let _ = minibuffer.delay_or_chord(beat).await;
        minibuffer.message("Fine. Be like that.");
        let _ = minibuffer.delay_or_chord(beat).await;
    }
    if minibuffer.prompt::<Password>("What's your password? ").await.is_ok() {
        minibuffer.message("Heh heh. Just kidding.");
    } else {
        minibuffer.message("canceled? Well, had to try.");
    }
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Bye.");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.clear();
    minibuffer.set_visible(false);
    Ok(())
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        // .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_systems(Startup, setup)
        .add_systems(PostStartup, demo.pipe(future_result_sink))
        .add_acts((
            // Add builtin commands.
            Builtin::default(),
            UniversalArgPlugin::default(),
            Act::new(demo.pipe(future_result_sink))
            .hotkey(keyseq!(D))))
        .run();
}
