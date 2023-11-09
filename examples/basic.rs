use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_nano_console::commands::*;
use bevy_nano_console::prompt::*;
use bevy_nano_console::tasks::*;
// use bevy_nano_console::ui::*;
use bevy_nano_console::proc::*;
use bevy_nano_console::*;
use nano_macro::*;
use std::future::Future;

fn ask_name<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
    async move {
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                prompt.message(format!("Hello, {first_name} {last_name}!"));
            }
        } else {
            eprintln!("Got err in ask name");
        }
    }
}

fn ask_age(mut prompt: Prompt) -> impl Future<Output = ()> {
    async move {
        if let Ok(age) = prompt.read::<i32>("What's your age? ").await {
            prompt.message(format!("You are {age} years old."));
        } else {
            eprintln!("Got err in ask age");
        }
    }
}

fn main() {
    App::new()
        .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_plugins(NanoPromptPlugin)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy NanoPrompt Basic Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        // .add_command("ask_name", ask_name.pipe(task_sink))
        .add_command(
            // Command::new("ask_name", Some(vec![KeyCode::Key1])),
            Command::new("ask_name", keyseq!(1)),
            ask_name.pipe(task_sink))
        .add_command(
            Command::new("ask_age", vec![KeyCode::A, KeyCode::A]),
            ask_age.pipe(task_sink))
        .add_command(
            Command::new("exec_command", vec![KeyCode::Semicolon]),
            exec_command.pipe(task_sink))
        .run();
}
