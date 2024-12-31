//! Demonstrate two commands using [MinibufferAsync].
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

// Ask the user for their age.
async fn ask_age(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let age = minibuffer.prompt::<Number<u8>>("What's your age? ").await?;
    minibuffer.message(format!("You are {age} years old."));
    Ok(())
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts((
            Act::new(ask_name.pipe(sink::future_result))
                .named("ask_name")
                .bind(keyseq!(N)),
            Act::new(ask_age.pipe(sink::future_result))
                .named("ask_age")
                .bind(keyseq!(A)),
            // Add a basic act but just one of them.
            BasicActs::default().remove("run_act").unwrap(),
        ))
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Hit 'N' for ask_name. Hit 'A' for ask_age.");
            minibuffer.set_visible(true);
        });
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("two-commands-async")
                .background(Srgba::hex("8ecae6").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
