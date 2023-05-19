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
static mut PROMISES: OnceCell<Vec<PromptState>> = OnceCell::new();
static mut PROMISES_VERSION: u32 = 0;

// event
struct ShowPrompt(bool);

#[derive(Resource)]
// resource
struct GlobalPromptState {
    active: bool,
    version: u32,
}

// struct PromptParam<'a> {
//     prompt: &'a str,
//     input: Option<&'a str>
// }

#[derive(Debug)]
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

impl CommandTask {
    fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(future);
        Self(task)
    }
}

fn main() {
    App::new()
        .add_event::<ShowPrompt>()
        .insert_resource(GlobalPromptState { version: 0, active: false })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [870., 1066.].into(),
                title: "Bevy Prompt Example".to_string(),
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
            // println!("Task handled.");
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
        // let thread_pool = AsyncComputeTaskPool::get();
        // let task = thread_pool.spawn(async move {
        //     ask_name().await;
        // });
        // commands.spawn(CommandTask(task));
        //
        // commands.spawn(CommandTask::new(async move {
        //     ask_name().await;
        // }));

        commands.spawn(CommandTask::new(ask_name()));
        return;
    }
    let version = unsafe { PROMISES_VERSION };
    if prompt_state.version != version {
        let new_state = unsafe { PROMISES.get().expect("No promises").len() } > 0;
        if prompt_state.active != new_state {
            prompt_state.active = new_state;
            println!("show prompt {}", new_state);
            show_prompt.send(ShowPrompt(new_state));

            if new_state {
                if let Some(prompt_state) = unsafe { PROMISES.get() }.expect("No promises").last() {
                    for (mut prompt, mut text) in query.iter_mut() {
                        text.sections[0].value.clone_from(&prompt_state.prompt);
                    }
                }
            }
        }
        prompt_state.version = version;
    }

    for (mut prompt, mut text) in query.iter_mut() {
        if prompt_state.active {
          let mut text_prompt = TextPrompt { text: &mut text };
            if keys.just_pressed(KeyCode::Back) {
                // let _ = text.sections[1].value.pop();
                let _ = text_prompt.input_get_mut().pop();
                continue;
            }
            for ev in char_evr.iter() {
                // text.sections[1].value.push(ev.char);
                text_prompt.input_get_mut().push(ev.char);
            }
            if keys.just_pressed(KeyCode::Return) {
                // Let's return this somewhere.
                // let result = text.sections[1].value.clone();
                let result = text_prompt.input_get_mut().clone();
                // text.sections[1].value.clear();
                println!("Got result {}", result);
                let prompt_state = unsafe { PROMISES.get_mut() }.expect("no promises").pop().expect("no promise");
                unsafe { PROMISES_VERSION += 1 };
                prompt_state.promise.resolve(result);
            }
        }
    }
}

struct Nanobuffer {
  prompt: String,
  message: String,
  input: String,
  inline_message: String,
  is_reading: bool,
}

trait PromptString {
  fn set(s: &str);
  fn clear();
}

enum NanoError {
  Cancelled,
  Message(&'static str)
}


trait NanoPrompt {
  // type Output : Future<Output = Result<String, NanoError>>;
  type Output;// : Future<Output = Result<String, String>>;
  fn prompt_get_mut(&mut self) -> &mut String;
  fn input_get_mut(&mut self) -> &mut String;
  fn message_get_mut(&mut self) -> &mut String;
  fn prompt_get(&self) -> &String;
  fn input_get(&self) -> &String;
  fn message_get(&self) -> &String;
  fn read(&mut self) -> Self::Output;

  fn read_string(&mut self, prompt: &str) -> Self::Output {
    self.input_get_mut().clear();
    let p = self.prompt_get_mut();
    p.clear();
    p.extend(prompt.chars());
    self.read()
  }
}

struct ProxyPrompt {
  prompt: String,
  message: String,
  input: String,
}

impl NanoPrompt for ProxyPrompt {
  type Output = PromiseOut<String>;
  fn prompt_get_mut(&mut self) -> &mut String {
    &mut self.prompt
  }
  fn input_get_mut(&mut self) -> &mut String {
    &mut self.input
  }
  fn message_get_mut(&mut self) -> &mut String {
    &mut self.message
  }
  fn prompt_get(&self) -> &String {
    &self.prompt
  }
  fn input_get(&self) -> &String {
    &self.input
  }
  fn message_get(&self) -> &String {
    &self.message
  }
  fn read(&mut self) -> Self::Output {
    let promise = PromiseOut::default();
    // self.promise = Some(promise.clone());
    // unsafe { PROMISES.get_mut() }.expect("no promises").push(PromptState { prompt: prompt.to_owned(),
    //                                                                        input: String::from(""),
    //                                                                        promise: promise.clone() });
    unsafe { PROMISES_VERSION += 1 };
    return promise;
  }
}

struct TextPrompt<'a> { text: &'a mut Text,
                    // promise: Option<PromiseOut<String>>
}

impl<'a> NanoPrompt for TextPrompt<'a> {
  type Output = PromiseOut<String>;
  fn prompt_get_mut(&mut self) -> &mut String {
    &mut self.text.sections[0].value
  }
  fn input_get_mut(&mut self) -> &mut String {
    &mut self.text.sections[1].value
  }
  fn message_get_mut(&mut self) -> &mut String {
    &mut self.text.sections[2].value
  }
  fn prompt_get(&self) -> &String {
    &self.text.sections[0].value
  }
  fn input_get(&self) -> &String {
    &self.text.sections[1].value
  }
  fn message_get(&self) -> &String {
    &self.text.sections[2].value
  }
  fn read(&mut self) -> Self::Output {
    let promise = PromiseOut::default();
    // self.promise = Some(promise.clone());
    // unsafe { PROMISES.get_mut() }.expect("no promises").push(PromptState { prompt: prompt.to_owned(),
    //                                                                        input: String::from(""),
    //                                                                        promise: promise.clone() });
    unsafe { PROMISES_VERSION += 1 };
    return promise;
  }
}

// impl Nanobuffer {
//   is_reading() -> bool;
//   read_string() -> impl Future<Result<String, String>>; //nanobuffer::Error
//   read<T>() -> impl Future<Result<T, String>>;
// }

// public interface INanobuffer {
//   /** Shows a message to the user when not reading. */
//   public string Message { get; set; }
//   /** Prompt shown when reading. */
//   public string Prompt { get; set; }
//   /** Input line from user. Set before Read() for default input. */
//   public string Input { get; set; }
//   /** Message shown when prompting. */
//   public string ErrorMessage { get; set; }
//   /** Returns true if currently reading from user. (This is a read only property.) */
//   public bool IsReading { get; }

//   /** Read input from the user with the given parser and token. Cancel based
//       on token. */
//   public
//     Task<T>
//     Read<T>(TryParseSource<T> tryParse,
//                          CancellationToken token);
// }

// use Into<PromptState>
fn user_read(prompt: &str) -> PromiseOut<String> {
    let promise: PromiseOut<String> = PromiseOut::default();
    println!("promise added");
    unsafe { PROMISES.get_mut() }.expect("no promises").push(PromptState { prompt: prompt.to_owned(),
                                                                           input: String::from(""),
                                                                           promise: promise.clone() });
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

