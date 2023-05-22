//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::ecs::prelude::Commands;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{CommandQueue, SystemState};
use bevy::prelude::*;
// use bevy::ecs::storage::Resources;
// use bevy::app::SystemAppConfig;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use once_cell::sync::OnceCell;
use promise_out::PromiseOut;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};

const ALIGN_ITEMS_COLOR: Color = Color::rgb(1., 0.066, 0.349);
const JUSTIFY_CONTENT_COLOR: Color = Color::rgb(0.102, 0.522, 1.);
const MARGIN: Val = Val::Px(5.);

// Seems like there should be a mutex in here.
static mut PROMISES: OnceCell<Vec<PromptState>> = OnceCell::new();
static mut PROMISES_VERSION: u32 = 0;

// event
struct ShowPrompt(bool);
struct RunCommandEvent(Box<dyn ScheduleLabel>);
impl RunCommandEvent {
    fn into_parts(self) -> Box<dyn ScheduleLabel> {
        self.0
    }
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(String);

#[derive(Debug)]
struct PromptState {
    prompt: String,
    input: String,
    promise: PromiseOut<String>,
}

#[derive(Component)]
struct PromptNode {}

struct ReadPrompt {
    prompt: ProxyPrompt,
    active: bool,
    promise: PromiseOut<String>,
}

impl ReadPrompt {
    fn into_parts(self) -> (ProxyPrompt, bool, PromiseOut<String>) {
        (self.prompt, self.active, self.promise)
    }
}

#[derive(Resource)]
struct PromptProvider {
    prompt_stack: Arc<Mutex<Vec<ReadPrompt>>>,
}

impl Default for PromptProvider {
    fn default() -> Self {
        Self {
            prompt_stack: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl PromptProvider {
    fn new_prompt(&mut self) -> ProxyPrompt {
        let prompt = ProxyPrompt::new(self.prompt_stack.clone());
        prompt
    }
}

#[derive(Clone)]
struct ProxyPrompt {
    prompt: String,
    message: String,
    input: String,
    prompts: Arc<Mutex<Vec<ReadPrompt>>>,
}

impl ProxyPrompt {
    fn new(prompts: Arc<Mutex<Vec<ReadPrompt>>>) -> Self {
        Self {
            prompt: String::from(""),
            message: String::from(""),
            input: String::from(""),
            prompts,
        }
    }
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
        self.prompts.lock().unwrap().push(ReadPrompt {
            prompt: self.clone(),
            promise: promise.clone(),
            active: false,
        });
        return promise;
    }
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

struct Dummy;

impl Future for Dummy {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

async fn silly() -> Dummy {
    Dummy {}
}

fn silly2(mut prompt_provider: ResMut<PromptProvider>) -> impl Future<Output = ()> {
    async {
        Dummy.await;
    }
}

fn main() {
    let sys = IntoSystem::into_system(ask_name4);
    // let sys = IntoSystem::into_system(task_sink::<Dummy>);
    // let sys2 = IntoSystem::into_system(silly2);
    let name = sys.name();
    println!("sys name {}", name);
    App::new()
        .add_event::<ShowPrompt>()
        .add_event::<RunCommandEvent>()
        .init_resource::<PromptProvider>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [870., 1066.].into(),
                title: "Bevy Nano Prompt Example".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_startup_system(spawn_layout)
        .add_system(prompt_visibility)
        .add_system(prompt_input)
        .add_system(handle_tasks)
        // .add_systems(Last, run_commands_hack)
        .add_systems(PreUpdate, run_commands)
        // .add_systems(CommandOneShot(name.into_owned()), ask_name3)
        .add_systems(CommandOneShot(name.into_owned()), ask_name4.pipe(task_sink))
        // .add_systems(PostUpdate, run_commands)
        .run();
}

fn run_commands(world: &mut World) {
    let mut event_system_state = SystemState::<(EventReader<RunCommandEvent>)>::new(world);
    let schedules: Vec<Box<dyn ScheduleLabel>> = {
        let mut events = event_system_state.get_mut(world);
        let mut look = false;
        if events.len() > 0 {
            println!("event count {}", events.len());
            look = true;
        }
        let results = events.iter().map(|e| e.0.clone()).collect();

        if look {
            println!("events after iter count {}", events.len());
        }
        events.clear();
        if look {
            println!("events after clear count {}", events.len());
        }
        results
    };

    for schedule in schedules {
        match world.try_run_schedule(schedule) {
            Err(e) => println!("Problem running command: {:?}", e),
            _ => {}
        }
    }
}

fn run_commands_hack(mut events: EventReader<RunCommandEvent>) {
    if events.len() > 0 {
        println!("hack: event count {}", events.len());
        // look = true;
    }
    if !events.is_empty() {
        events.clear();
    }
}

//                 world: &mut World) {//, resources: &mut Resources<true>) {
//   // https://bevy-cheatbook.github.io/programming/res.html
//   let mut command = Commands::new(&mut CommandQueue::default(), &world);
//   let mut event_system_state = SystemState::<(
//         EventReader<RunCommandEvent>
//     )>::new(world);
//     let (mut events) = event_system_state.get_mut(world);

//     for RunCommandEvent(system) in events.iter() {
//   // for RunCommandEvent(system) in run_commands.iter() {
//     system.run((), &mut world);

//   }

// }

fn handle_tasks(mut commands: Commands, mut command_tasks: Query<(Entity, &mut CommandTask)>) {
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
    mut prompt_provider: ResMut<PromptProvider>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: EventWriter<ShowPrompt>,
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Text, With<PromptNode>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        println!("tab pressed");
        // commands.spawn(CommandTask::new(ask_name()));
        // commands.spawn(CommandTask::new(ask_name3(prompt_provider.new_prompt())));
        run_command.send(RunCommandEvent(Box::new(CommandOneShot(
            "nanoprompt::ask_name4".to_owned(),
        ))));
        return;
    }

    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();
    for mut text in query.iter_mut() {
        if prompts.len() > 0 {
            let mut text_prompt = TextPrompt { text: &mut text };
            if keys.just_pressed(KeyCode::Return) {
                let read_prompt = prompts.pop().unwrap();
                let result = text_prompt.input_get().clone();
                println!("Got result {}", result);
                let (_, _, promise) = read_prompt.into_parts();
                promise.resolve(result);
                if prompts.len() == 0 {
                    show_prompt.send(ShowPrompt(false));
                }
                continue;
            }
            let read_prompt = prompts.last_mut().unwrap();
            if !read_prompt.active {
                // Must set it up.
                text_prompt.clone_from(&read_prompt.prompt);
                read_prompt.active = true;
                for i in 0..prompts.len() - 1 {
                    prompts[i].active = false;
                }

                show_prompt.send(ShowPrompt(true));
            }
            if keys.just_pressed(KeyCode::Back) {
                let _ = text_prompt.input_get_mut().pop();
                continue;
            }
            text_prompt
                .input_get_mut()
                .extend(char_evr.iter().map(|ev| ev.char));
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
    Message(&'static str),
}

// XXX: Rename to NanoConsole?
trait NanoPrompt {
    // type Output : Future<Output = Result<String, NanoError>>;
    type Output: Future<Output = Arc<Result<String, String>>>;
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

    fn clone_from<T: NanoPrompt>(&mut self, other: &T) {
        self.prompt_get_mut().clone_from(other.prompt_get());
        self.message_get_mut().clone_from(other.message_get());
        self.input_get_mut().clone_from(other.input_get());
    }
}

struct TextPrompt<'a> {
    text: &'a mut Text,
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
        panic!("Not sure this should ever be called.");
    }
}

fn user_read(prompt: &str) -> PromiseOut<String> {
    let promise: PromiseOut<String> = PromiseOut::default();
    println!("promise added");
    unsafe { PROMISES.get_mut() }
        .expect("no promises")
        .push(PromptState {
            prompt: prompt.to_owned(),
            input: String::from(""),
            promise: promise.clone(),
        });
    unsafe { PROMISES_VERSION += 1 };
    return promise;
}

fn prompt_visibility(
    mut show_prompt: EventReader<ShowPrompt>,
    query: Query<(&Parent, &PromptNode)>,
    mut q_parent: Query<&mut Visibility>,
) {
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

async fn ask_name2(mut prompt: impl NanoPrompt) {
    println!("ask name 2 called");
    if let Ok(name) = &*prompt.read_string("What's your first name? ").await {
        println!("Hello, {}", name);
    } else {
        println!("Got err in ask now");
    }
}

// Take a look at pipe system. https://docs.rs/bevy/latest/bevy/ecs/system/trait.SystemParamFunction.html
fn ask_name3<'a>(mut commands: Commands, mut prompt_provider: ResMut<'a, PromptProvider>) {
    let mut prompt = prompt_provider.new_prompt();
    commands.spawn(CommandTask::new(async move {
        println!("ask name 3 called");
        if let Ok(first_name) = &*prompt.read_string("What's your first name? ").await {
            if let Ok(last_name) = &*prompt.read_string("What's your last name? ").await {
                println!("Hello, {} {}", first_name, last_name);
            }
        } else {
            println!("Got err in ask now");
        }
    }));
}

// This one doesn't work.
// async fn ask_name4<'a>(mut prompt_provider: ResMut<'a, PromptProvider>) {
//     let mut prompt = prompt_provider.new_prompt();
//     println!("ask name 3 called");
//     if let Ok(first_name) = &*prompt.read_string("What's your first name? ").await {
//       if let Ok(last_name) = &*prompt.read_string("What's your last name? ").await {
//         println!("Hello, {} {}", first_name, last_name);
//       }
//     } else {
//         println!("Got err in ask now");
//     }
// }

fn ask_name4<'a>(mut prompt_provider: ResMut<'a, PromptProvider>) -> impl Future<Output = ()> {
    let mut prompt = prompt_provider.new_prompt();
    println!("ask name 3 called");
    async move {
        if let Ok(first_name) = &*prompt.read_string("What's your first name? ").await {
            if let Ok(last_name) = &*prompt.read_string("What's your last name? ").await {
                println!("Hello, {} {}", first_name, last_name);
            }
        } else {
            println!("Got err in ask now");
        }
    }
}

fn task_sink<T: Future<Output = ()> + Send + 'static>(In(future): In<T>, mut commands: Commands) {
    commands.spawn(CommandTask::new(async move { future.await }));
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    // This just has to be initialized somewhere.
    unsafe { PROMISES.set(vec![]) }.unwrap();

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                // Fill the entire window.
                // Does it have to fill the whole window?
                width: Val::Percent(100.),
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
                    builder
                        .spawn(TextBundle::from_sections([
                            TextSection::new(
                                "PromptNode: ",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "input",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                " message",
                                TextStyle {
                                    font,
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                        ]))
                        .insert(PromptNode { // active: false,
                                             // promises: vec![]
                            });
                });
        });
}
