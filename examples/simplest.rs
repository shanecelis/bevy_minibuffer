use bevy::prelude::*;
use bevy_minibuffer::{prelude::*, sync::Minibuffer};

#[path = "common/lib.rs"]
mod common;

/// Ask the user for their name. Say hello.
fn ask_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your first name? ")
        .observe(
            |trigger: Trigger<AskyEvent<String>>, mut minibuffer: Minibuffer| {
                let first_name = trigger.event().0.clone().unwrap();
                minibuffer
                    .prompt::<TextField>("What's your last name? ")
                    .observe(
                        move |trigger: Trigger<AskyEvent<String>>, mut minibuffer: Minibuffer| {
                            let last_name = trigger.event().0.clone().unwrap();
                            minibuffer.message(format!("Hello, {first_name} {last_name}!"));
                        },
                    );
            },
        );
}

fn main() {
    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Simplest Example".into(),
    };
    App::new()
        // .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_plugins((
            DefaultPlugins.set(video_settings.window_plugin()),
            MinibufferPlugins.set(video_settings.minibuffer_plugin()),
        ))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add(Act::new(ask_name).hotkey(keyseq! { Ctrl-A N }));
}
