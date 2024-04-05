use asky::{bevy::future_sink, Message};
use bevy::prelude::*;
use bevy_nano_console::commands::*;
use bevy_nano_console::prompt::*;
use bevy_nano_console::*;

/// Ask the user for their name. Say hello.
async fn ask_name(mut asky: Minibuffer) -> Result<(), Error> {
    let first_name = asky.prompt(asky::Text::new("What's your first name?")).await?;
    let last_name = asky.prompt(asky::Text::new("What's your last name?")).await?;
    asky.prompt(Message::new(format!("Hello, {first_name} {last_name}!"))).await?;
    Ok(())
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Simplest Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(NanoPromptPlugin {
            config: ConsoleConfig {
                auto_hide: true,
                // auto_hide: false,
                hide_delay: Some(3000),
                text_style: TextStyle {
                    font_size: 20.0,
                    ..default()
                },
            },
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add_act(Act::new()
                     .named("ask_name")
                     .hotkey(keyseq!(ctrl-A N)),

                     ask_name.pipe(future_sink));
}
