use asky::Message;
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;
use common::VideoCaptureSettings;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: Minibuffer) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt(asky::Text::new("What's your first name?"))
        .await?;
    let last_name = minibuffer
        .prompt(asky::Text::new("What's your last name?"))
        .await?;
    minibuffer
        .prompt(Message::new(format!("Hello, {first_name} {last_name}!")))
        .await?;
    Ok(())
}

fn main() {
    App::new()
        // .add_plugins(DefaultPlugins)
        // .add_plugins(MinibufferPlugin::default())
        .add_plugins(VideoCaptureSettings { title: "Bevy Minibuffer Simplest Example".into() })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add_act(
        Act::new().named("ask_name").hotkey(keyseq!(ctrl-A N)),
        ask_name.pipe(future_sink),
    );
}
