use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_nano_console::commands::*;
use bevy_nano_console::*;
use bevy_nano_console::ui::*;
use bevy_nano_console::style::*;
use keyseq::bevy::pkeyseq as keyseq;
use std::{time::Duration, future::Future};
use asky::prelude::*;
use asky::bevy::{Asky, future_sink};

async fn ask_name<'a>(mut asky: Minibuffer) {
    if let Ok(first_name) = asky.prompt(asky::Text::new("What's your \nfirst name? ")).await {
        if let Ok(last_name) = asky.prompt(asky::Text::new("What's your last name? ")).await {
            let _ = asky.prompt(Message::new(format!("Hello, {first_name} {last_name}!"))).await;
            return;
        }
    }
    let _ = asky.prompt(Message::new("Got err in ask name")).await;
}

// fn ask_age(mut prompt: Prompt) -> impl Future<Output = ()> {
//     async move {
//         if let Ok(age) = prompt.read::<i32>("What's your age? ").await {
//             prompt.message(format!("You are {age} years old."));
//         } else {
//             eprintln!("Got err in ask age");
//         }
//     }
// }

fn asky_age(mut asky: Asky, query: Query<Entity, With<PromptContainer>>) -> impl Future<Output = ()> {
    let id: Entity = query.single();
    async move {
        let _ = asky.clear(id).await;
        if let Ok(age) = asky.prompt_styled(Number::<u8>::new("What's your age? "), id, MinibufferStyle::default()).await {
            let _ = asky.delay(Duration::from_secs(2)).await;
            let _ = asky.clear(id).await;
            let _ = asky.prompt(Message::new(format!("You are {age} years old.")), id).await;
        } else {
            let _ = asky.clear(id).await;
            let _ = asky.prompt(Message::new("error: I can only handle u8s for age.."), id).await;
        }
    }
}

// fn mb_age(mut asky: Minibuffer) -> impl Future<Output = ()> {
//     async move {
//         if let Ok(age) = asky.prompt(Number::<u8>::new("What's your age? ")).await {
//             let _ = asky.delay(Duration::from_secs(2)).await;
//             let _ = asky.prompt(Message::new(format!("You are {age} years old."))).await;
//         } else {
//             let _ = asky.prompt(Message::new("error: I can only handle u8s for age..")).await;
//         }
//     }
// }
async fn mb_age(mut asky: Minibuffer) {
    if let Ok(age) = asky.prompt(Number::<u8>::new("What's your age? ")).await {
        let _ = asky.delay(Duration::from_secs(2)).await;
        let _ = asky.prompt(Message::new(format!("You are {age} years old."))).await;
    } else {
        let _ = asky.prompt(Message::new("error: I can only handle u8s for age..")).await;
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
        .add_act(Act::unregistered().named("ask_nam"), ask_name.pipe(future_sink))
        // .add_command(
        //     // Command::new("ask_name", Some(vec![KeyCode::Digit1])),
        //     Act::new("ask_name", keyseq!(1)),
        //     ask_name.pipe(future_sink),
        // )
        // .add_command(
        //     // Command::new("ask_age", vec![KeyCode::KeyA, KeyCode::KeyA]),
        //     Act::new("ask_age", keyseq!(A A)),
        //     ask_age.pipe(future_sink),
        // )
        .add_act(
            Act::unregistered()
                .named("exec_act")
                .hotkey(keyseq!(shift-;))
                .in_exec_act(false),
            exec_act.pipe(future_sink),
        )

        .add_act(
            Act::unregistered()
                .named("toggle_vis")
                .hotkey(keyseq!('`'))
                .in_exec_act(false),
            toggle_visibility,
        )
        .add_act(
            Act::unregistered()
                .named("list_acts")
                .hotkey(keyseq!(ctrl-H A)),
            list_acts.pipe(future_sink),
        )

        .add_act(
            Act::unregistered()
                .named("list_key_bindings")
                .hotkey(keyseq!(ctrl-H B)),
            list_key_bindings::<StartActEvent>.pipe(future_sink),
        )
        .add_systems(Startup, setup)
        .add_systems(Startup, add_acts)
        .add_systems(Startup, add_acts2)
        .run();
}

fn add_acts(world: &mut World) {
    let system_id = world.register_system(ask_name.pipe(future_sink));
    world.spawn(Act::new(system_id)
        .named("ask_name2")
        .hotkey(keyseq!(1)));
}

fn add_acts2(mut commands: Commands) {

    // commands.spawn(Act::unregistered()
    //                .named("ask_age")
    //                .hotkey(keyseq!(A A)))
    //     .add(Register::new(ask_age.pipe(future_sink)));

    // commands.add_act(Act::unregistered()
    //                  .named("ask_age2")
    //                  .hotkey(keyseq!(B B)),

    //                  ask_age.pipe(future_sink));

    commands.add_act(Act::unregistered()
                     .named("asky_age")
                     .hotkey(keyseq!(C C)),

                     asky_age.pipe(future_sink));

    commands.add_act(Act::unregistered()
                     .named("mb_age")
                     .hotkey(keyseq!(D D)),

                     mb_age.pipe(future_sink));
}
