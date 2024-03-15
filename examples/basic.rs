use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_nano_console::commands::*;
use bevy_nano_console::proc::*;
use bevy_nano_console::prompt::*;
use bevy_nano_console::tasks::*;
use bevy_nano_console::*;
use keyseq::bevy::pkeyseq as keyseq;
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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
        // .add_command(
        //     // Command::new("ask_name", Some(vec![KeyCode::Digit1])),
        //     Act::new("ask_name", keyseq!(1)),
        //     ask_name.pipe(task_sink),
        // )
        // .add_command(
        //     // Command::new("ask_age", vec![KeyCode::KeyA, KeyCode::KeyA]),
        //     Act::new("ask_age", keyseq!(A A)),
        //     ask_age.pipe(task_sink),
        // )
        // .add_command(
        //     Act::new(
        //         "exec_command",
        //         keyseq!(shift-;),
        //     )
        //     .autocomplete(false),
        //     exec_command.pipe(task_sink),
        // )
        .add_systems(Startup, setup)
        .add_systems(Startup, add_acts)
        .run();
}

fn add_acts(world: &mut World) {
    let system_id = world.register_system(ask_name.pipe(task_sink));
    world.spawn(Act::new(system_id)
        .name("ask_name")
        .hotkey(keyseq!(1)));
}

fn add_acts2(mut commands: Commands) {
    commands.spawn(Act::unregistered()
                   .name("ask_age")
                   .hotkey(keyseq!(A A)))
        .add(Register::new(ask_age.pipe(task_sink)));
}
