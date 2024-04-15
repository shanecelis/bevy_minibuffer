use asky::{Message, Number};
use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_minibuffer::act::*;
use bevy_minibuffer::prelude::*;
use std::time::Duration;
#[path = "common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
async fn ask_name(mut asky: Minibuffer) -> Result<(), Error> {
    let first_name = asky
        .prompt(asky::Text::new("What's your first name?"))
        .await?;
    let last_name = asky
        .prompt(asky::Text::new("What's your last name?"))
        .await?;
    asky.prompt(Message::new(format!("Hello, {first_name} {last_name}!")))
        .await?;
    Ok(())
}

// Ask the user for their age.
async fn ask_age(mut asky: Minibuffer) -> Result<(), Error> {
    let age = asky.prompt(Number::<u8>::new("What's your age?")).await?;
    asky.delay(Duration::from_secs(2)).await;
    asky.prompt(Message::new(format!("You are {age} years old.")))
        .await?;
    Ok(())
}

/// Example of adding acts with an exclusive world system.
fn add_acts_with_mutable_world(world: &mut World) {
    let system_id = world.register_system(ask_name.pipe(future_result_sink));
    world.spawn(
        Act::preregistered(system_id)
            .named("ask_name")
            .hotkey(keyseq!(ctrl-A N)),
    );
}

/// Add acts using [Commands] with [AddAct].
fn add_acts(mut commands: Commands) {
    commands.add_act(
        Act::new().named("ask_age").hotkey(keyseq! { ctrl-A A }),
        ask_age.pipe(future_result_sink),
    );
}

fn main() {
    App::new()
        .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        // .add_plugins(DefaultPlugins)
        // .add_plugins(MinibufferPlugin::default())
        .add_plugins(common::VideoCaptureSettings {
            title: "Bevy Minibuffer Basic Example".into(),
        })
        // Add acts directly to an app via [AddAct].
        .add_systems(Startup, setup)
        .add_systems(Startup, add_builtins)
        .add_systems(Startup, add_acts)
        .add_systems(Startup, add_acts_with_mutable_world)
        .run();
}

/// Add builtin commands.
fn add_builtins(world: &mut World) {
    let mut builtin = Builtin::new(world);
    for act in [
        builtin.exec_act(),
        builtin.list_acts(),
        builtin.list_key_bindings(),
        builtin.describe_key(),
    ] {
        world.spawn(act);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
