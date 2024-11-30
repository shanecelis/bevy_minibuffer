use bevy::prelude::*;
use bevy_asky::prelude::*;
use bevy_minibuffer::prelude::{Error, *};
use std::time::Duration;

#[path = "../common/lib.rs"]
mod common;

/// Demo some of Minibuffer's prompts.
async fn demo(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let beat = Duration::from_secs_f32(2.0);
    let yes: bool = minibuffer
        .prompt::<Confirm>("Hey, psst. Want to see something cool?")
        .await?;
    minibuffer.message(if yes {
        "Oh, good!"
    } else {
        "Oh, ok. Hit 'D' if you change your mind."
    });
    if !yes {
        return Ok(());
    }
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("First a few questions.");
    let _ = minibuffer.delay_or_chord(beat).await;
    let lang: usize = minibuffer
        .prompt_with::<RadioGroup>("Which do you prefer?", |commands| {
            commands.prompt_children::<Radio>(["brainf*ck", "rust", "x86 machine code"]);
        })
        .await?;
    minibuffer.message(if lang == 1 {
        "Me too!"
    } else {
        "More power to you."
    });
    let _ = minibuffer.delay_or_chord(beat).await;
    let selection: Vec<bool> = minibuffer
        .prompt_with::<CheckboxGroup>("What game engines do you use?", |commands| {
            commands.prompt_children::<Checkbox>(["Unity", "Unreal", "Godot", "Bevy", "other"]);
        })
        .await?;
    minibuffer.message(if selection[3] {
        "Well, have I got news for you!"
    } else if !selection.iter().any(|x| *x) {
        "This may not interest you then."
    } else {
        "Those are also great."
    });
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Minibuffer works for bevy now!");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("So...");
    let _ = minibuffer.delay_or_chord(beat).await;
    let signup = minibuffer
        .prompt::<Confirm>("Let's sign you up on our email list.")
        .await?;
    if !signup {
        minibuffer.message("Come on. It'll be fun!");
        let _ = minibuffer.delay_or_chord(beat).await;
    }
    let email: Result<String, Error> = minibuffer.prompt::<TextField>("What's your email? ").await;
    if email.is_err() {
        minibuffer.message("canceled?");
        let _ = minibuffer.delay_or_chord(beat).await;
        minibuffer.message("Fine. Be like that.");
        let _ = minibuffer.delay_or_chord(beat).await;
    }
    if minibuffer
        .prompt::<Password>("Tell me a secret: ")
        .await
        .is_ok()
    {
        minibuffer.message("Omg.");
        let _ = minibuffer.delay_or_chord(beat).await;
        minibuffer.message("I'm taking that to my exit.");
    } else {
        minibuffer.message("Canceled? Well, you're no fun.");
    }
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.message("Bye.");
    let _ = minibuffer.delay_or_chord(beat).await;
    minibuffer.clear();
    minibuffer.set_visible(false);
    Ok(())
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_systems(PostStartup, demo.pipe(future_result_sink));
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("demo")
                .resolution(Vec2::new(600.0, 200.0))
                .background(Srgba::hex("f94144").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .add_acts((
            // Add basic commands.
            BasicActs::default(),
            UniversalArgActs::default(),
            Act::new(demo.pipe(future_result_sink)).bind(keyseq!(D)),
        ))
        .run();
}
