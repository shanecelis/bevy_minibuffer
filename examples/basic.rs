use bevy::prelude::*;
use bevy::winit::WinitSettings;
use nanoprompt::commands::*;
use nanoprompt::prompt::*;
use nanoprompt::tasks::*;
use nanoprompt::ui::*;
use nanoprompt::*;
use std::future::Future;

fn ask_name5<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask name 5 called");
    async move {
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                prompt.message(format!("Hello, {} {}", first_name, last_name));
            }
        } else {
            println!("Got err in ask now");
        }
    }
}

fn ask_age2(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask age2 called");
    async move {
        if let Ok(age) = prompt.read::<i32>("What's your age? ").await {
            prompt.message(format!("You are {age} years old."));
        } else {
            println!("Got err in ask age");
        }
    }
}

fn main() {
    App::new()
        .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_plugin(NanoPromptPlugin)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy NanoPrompt Basic Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        // .add_command("ask_name", ask_name5.pipe(task_sink))
        .add_command(
            Command::new("ask_name", Some(KeyCode::Key1)),
            ask_name5.pipe(task_sink),
        )
        .add_command("ask_age2", ask_age2.pipe(task_sink))
        .add_command(
            Command::new("exec_command", Some(KeyCode::Semicolon)),
            exec_command.pipe(task_sink),
        )
        .run();
}
