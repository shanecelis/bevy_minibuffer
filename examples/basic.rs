use bevy::prelude::*;
use nanoprompt::commands::*;
use nanoprompt::prompt::*;
use nanoprompt::tasks::*;
use nanoprompt::ui::*;
use nanoprompt::*;
use std::future::Future;

#[allow(dead_code)]
async fn ask_name2(mut prompt: impl NanoPrompt) {
    println!("ask name 2 called");
    if let Ok(name) = prompt.read::<String>("What's your first name? ").await {
        println!("Hello, {}", name);
    } else {
        println!("Got err in ask now");
    }
}

#[allow(dead_code)]
// Take a look at pipe system. https://docs.rs/bevy/latest/bevy/ecs/system/trait.SystemParamFunction.html
fn ask_name3<'a>(mut commands: Commands, mut prompt_provider: ResMut<'a, PromptProvider>) {
    let mut prompt = prompt_provider.new_prompt();
    commands.spawn(TaskSink::new(async move {
        println!("ask name 3 called");
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                println!("Hello, {} {}", first_name, last_name);
            }
        } else {
            println!("Got err in ask now");
        }
    }));
}

#[allow(dead_code)]
fn ask_name4<'a>(mut prompt_provider: ResMut<'a, PromptProvider>) -> impl Future<Output = ()> {
    let mut prompt = prompt_provider.new_prompt();
    println!("ask name 4 called");
    async move {
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                println!("Hello, {} {}", first_name, last_name);
            }
        } else {
            println!("Got err in ask now");
        }
    }
}

fn ask_name5<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask name 5 called");
    async move {
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                println!("Hello, {} {}", first_name, last_name);
            }
        } else {
            println!("Got err in ask now");
        }
    }
}

// fn ask_name6<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
//     println!("ask name 6 called");
//     async move {
//         if let Ok(TomDickHarry(first_name)) = prompt.read("What's your first name? ").await {
//             println!("Hello, {}", first_name);
//         } else {
//             println!("Got err in ask now");
//         }
//     }
// }

// fn ask_name6<'a>(mut prompt: TextPromptParam) -> impl Future<Output = ()> {
//     println!("ask name 5 called");
//     async move {
//         if let Ok(first_name) = prompt.read::<string>("What's your first name? ").await {
//             if let Ok(last_name) = prompt.read::<string>("What's your last name? ").await {
//                 println!("Hello, {} {}", first_name, last_name);
//             }
//         } else {
//             println!("Got err in ask now");
//         }
//     }
// }

fn ask_age2(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask age2 called");
    async move {
        if let Ok(age) = prompt.read::<i32>("What's your age? ").await {
            println!("You are {} years old.", age);
        } else {
            println!("Got err in ask age");
        }
    }
}

fn main() {
    App::new()
        .add_plugin(NanoPromptPlugin)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy NanoPrompt Basic Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        // .add_command("ask_name", ask_name3)
        // .add_command("ask_name", ask_name4.pipe(task_sink))
        .add_command("ask_name", ask_name5.pipe(task_sink))
        // .add_command("ask_name", ask_name6.pipe(task_sink))
        // .add_command("ask_name", ask_name6.pipe(task_sink))
        // .add_command("ask_age", ask_age.pipe(task_sink))
        .add_command("ask_age2", ask_age2.pipe(task_sink))
        .add_command(
            Command::new("exec_command", Some(KeyCode::Semicolon)),
            exec_command.pipe(task_sink),
        )
        .run();
}
