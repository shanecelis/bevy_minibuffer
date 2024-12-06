//! Demonstrate two commands using [Minibuffer].
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
fn ask_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your first name? ")
        .observe(
            |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                let first_name = trigger.event_mut().take_result().unwrap();
                minibuffer
                    .prompt::<TextField>("What's your last name? ")
                    .observe(
                        move |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                            let last_name = trigger.event_mut().take_result().unwrap();
                            minibuffer.message(format!("Hello, {first_name} {last_name}!"));
                        },
                    );
            },
        );
}

// Ask the user for their age.
fn ask_age(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<Number<u8>>("What's your age? ")
        .observe(
            |mut trigger: Trigger<Submit<u8>>, mut minibuffer: Minibuffer| {
                let age = trigger.event_mut().take_result().unwrap();
                minibuffer.message(format!("You are {age} years old."));
            },
        );
}

fn plugin(app: &mut App) {
    app
        .add_plugins(MinibufferPlugins)
        .add_acts((
            Act::new(ask_name)
                .named("ask_name")
                .bind(keyseq!(N)),
            Act::new(ask_age)
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
            common::VideoCapturePlugin::new("two-commands").background(Srgba::hex("219ebc").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
