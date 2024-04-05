use asky::{bevy::future_sink, Message, Number};
use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_minibuffer::commands::*;
use bevy_minibuffer::prompt::*;
use bevy_minibuffer::*;
use keyseq::bevy::pkeyseq as keyseq;
use std::time::Duration;

/// Ask the user for their name. Say hello.
async fn ask_name(mut asky: Minibuffer) -> Result<(), Error> {
    let first_name = asky.prompt(asky::Text::new("What's your first name?")).await?;
    let last_name = asky.prompt(asky::Text::new("What's your last name?")).await?;
    asky.prompt(Message::new(format!("Hello, {first_name} {last_name}!"))).await?;
    Ok(())
}

// Ask the user for their age.
async fn ask_age(mut asky: Minibuffer) -> Result<(), Error> {
    let age = asky.prompt(Number::<u8>::new("What's your age?")).await?;
    asky.delay(Duration::from_secs(2)).await?;
    asky.prompt(Message::new(format!("You are {age} years old."))).await?;
    Ok(())
}

/// Example of adding acts with an exclusive world system.
fn add_acts_with_mutable_world(world: &mut World) {
    let system_id = world.register_system(ask_name.pipe(future_sink));
    world.spawn(
        Act::preregistered(system_id)
            .named("ask_name")
            .hotkey(keyseq!(1)),
    );
}

/// Add acts using [Commands] with [AddAct].
fn add_acts(mut commands: Commands) {

    commands.add_act(Act::new()
                     .named("ask_age")
                     .hotkey(keyseq!(D D)),

                     ask_age.pipe(future_sink));

    commands.add_act(Act::new()
                     .named("ask_name")
                     .hotkey(keyseq!(E E)),

                     ask_name.pipe(future_sink));
}

fn main() {
    App::new()
        .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy NanoPrompt Basic Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(MinibufferPlugin {
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
        // Add acts directly to an app via [AddAct].
        .add_act(
            Act::new()
                .named("exec_act")
                .hotkey(keyseq!(shift-;))
                .in_exec_act(false),
            exec_act.pipe(future_sink),
        )
        .add_act(
            Act::new()
                .named("list_acts")
                .hotkey(keyseq!(ctrl-H A)),
            list_acts.pipe(future_sink),
        )
        .add_act(
            Act::new()
                .named("list_key_bindings")
                .hotkey(keyseq!(ctrl-H B)),
            list_key_bindings::<StartActEvent>.pipe(future_sink),
        )
        .add_systems(Startup, setup)
        .add_systems(Startup, add_acts)
        .add_systems(Startup, add_acts_with_mutable_world)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
