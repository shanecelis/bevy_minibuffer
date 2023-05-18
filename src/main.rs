//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::prelude::*;
use std::future::Future;
use std::sync::Arc;
use promise_out::PromiseOut;
use once_cell::sync::OnceCell;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

const ALIGN_ITEMS_COLOR: Color = Color::rgb(1., 0.066, 0.349);
const JUSTIFY_CONTENT_COLOR: Color = Color::rgb(0.102, 0.522, 1.);
const MARGIN: Val = Val::Px(5.);

// Seems like there should be a mutex in here.
static mut PROMISES: OnceCell<Vec<PromiseOut<String>>> = OnceCell::new();
static mut PROMISES_VERSION: u32 = 0;

// event
struct ShowPrompt(bool);

#[derive(Resource)]
// resource
struct GlobalPromptState {
    active: bool,
    version: u32,
}

struct PromptState {
    prompt: String,
    input: String,
    promise: PromiseOut<String>
}

#[derive(Component)]
struct Prompt {
    // active: bool,
    // promises: Vec<PromiseOut<String>>
}

#[derive(Component)]
struct CommandTask(Task<()>);

fn main() {
    App::new()
        .add_event::<ShowPrompt>()
        .insert_resource(GlobalPromptState { version: 0, active: false })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [870., 1066.].into(),
                title: "Bevy Text Layout Example".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_startup_system(spawn_layout)
        .add_system(prompt_visibility)
        .add_system(prompt_input)
        .add_system(handle_tasks)
        .run();
}
fn handle_tasks(
    mut commands: Commands,
    mut command_tasks: Query<(Entity, &mut CommandTask)>,
) {
    for (entity, mut task) in &mut command_tasks {
        if let Some(_) = future::block_on(future::poll_once(&mut task.0)) {
            // Add our new PbrBundle of components to our tagged entity
            println!("Task handled.");
            commands.entity(entity).despawn();
            // commands.entity(entity).remove::<CommandTask>();
        } else {
            // println!("Task not handled.");
        }
    }
}
// [[https://bevy-cheatbook.github.io/programming/local.html][Local Resources - Unofficial Bevy Cheat Book]]i

/// prints every char coming in; press enter to echo the full string
fn prompt_input(
    mut commands: Commands,
    mut prompt_state: ResMut<GlobalPromptState>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: EventWriter<ShowPrompt>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&mut Prompt, &mut Text)>) {

    if keys.just_pressed(KeyCode::Tab) {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move {
            ask_name().await;
        });
        commands.spawn(CommandTask(task));
        return;
    }
    let version = unsafe { PROMISES_VERSION };
    if prompt_state.version != version {
        let new_state = unsafe { PROMISES.get().expect("No promises").len() } > 0;
        if prompt_state.active != new_state {
            prompt_state.active = new_state;
            println!("show prompt {}", new_state);
            show_prompt.send(ShowPrompt(new_state));
        }
        prompt_state.version = version;
    }

    for (mut prompt, mut text) in query.iter_mut() {
        if prompt_state.active {
            if keys.just_pressed(KeyCode::Back) {
                let _ = text.sections[1].value.pop();
                continue;
            }
            for ev in char_evr.iter() {
                text.sections[1].value.push(ev.char);
            }
            if keys.just_pressed(KeyCode::Return) {
                // Let's return this somewhere.
                let result = text.sections[1].value.clone();
                text.sections[1].value.clear();
                println!("Got result {}", result);
                let promise = unsafe { PROMISES.get_mut() }.expect("no promises").pop().expect("no promise");
                unsafe { PROMISES_VERSION += 1 };
                promise.resolve(result);
            }
        }
    }
}

fn user_read(prompt: &str) -> PromiseOut<String> {
    let promise: PromiseOut<String> = PromiseOut::default();
    println!("promise added");
    unsafe { PROMISES.get_mut() }.expect("no promises").push(promise.clone());
    unsafe { PROMISES_VERSION += 1 };
    return promise;
}

fn prompt_visibility(mut show_prompt: EventReader<ShowPrompt>,
                     query: Query<(&Parent, &Prompt)>,
                     mut q_parent: Query<&mut Visibility>) {
    if show_prompt.is_empty() {
        return;
    }
    let show = show_prompt.iter().fold(false, |acc, x| acc || x.0);
    for (parent, prompt) in query.iter() {
        if let Ok(mut v) = q_parent.get_mut(parent.get()) {
            // println!("AAAA");
            *v = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

async fn ask_name() {
    println!("ask name called");
    if let Ok(name) = &*user_read("What's your name? ").await {
        println!("Hello, {}", name);
    } else {
        println!("Got err in ask now");
    }
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    unsafe { PROMISES.set(vec![]) }.unwrap();
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                // Fill the entire window.
                // Does it have to fill the whole window?
                size: Size::all(Val::Percent(100.)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    visibility: Visibility::Hidden,
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.,
                        padding: UiRect {
                            top: Val::Px(1.),
                            left: Val::Px(1.),
                            right: Val::Px(1.),
                            bottom: Val::Px(1.),
                        },
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                .with_children(|builder| {

                            builder.spawn(TextBundle::from_sections([
                                TextSection::new(
                                    "Prompt: ",
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        color: Color::WHITE,
                                    },
                                ),
                                TextSection::new(
                                    "",
                                    TextStyle {
                                        font,
                                        font_size: 24.0,
                                        color: Color::WHITE,
                                    },
                                )
                            ]))
                            .insert(Prompt { // active: false,
                                             // promises: vec![]
                            });
                });

        });
}

