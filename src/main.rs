// #![feature(return_position_impl_trait_in_trait)]
#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]
use bevy::ecs::component::Tick;
use bevy::ecs::prelude::Commands;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{SystemMeta, SystemParam, SystemState};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::Duration;
use futures_lite::future;
use promise_out::{
    pair::Producer,
    Promise,
};
use std::borrow::Cow;
use std::future::Future;
use std::sync::{Arc, Mutex};

// const MARGIN: Val = Val::Px(5.);
const PADDING: Val = Val::Px(3.);

struct RunCommandEvent(Box<dyn ScheduleLabel>);

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(Cow<'static, str>);

#[derive(Component)]
struct PromptContainer;
#[derive(Component)]
struct PromptNode;

#[allow(dead_code)]
#[derive(Debug)]
enum NanoError {
    Cancelled,
    Message(Cow<'static, str>),
}

struct ReadPrompt {
    prompt: PromptBuf,
    active: bool,
    promise: Producer<PromptBuf, NanoError>,
}

#[derive(Resource, Clone)]
pub struct PromptProvider {
    prompt_stack: Arc<Mutex<Vec<ReadPrompt>>>,
    hide_delay: f32,
}

impl Default for PromptProvider {
    fn default() -> Self {
        Self {
            prompt_stack: Arc::new(Mutex::new(vec![])),
            hide_delay: 1.0,
        }
    }
}

impl PromptProvider {
    fn new_prompt(&mut self) -> Prompt {
        let prompt = Prompt::new(self.prompt_stack.clone());
        prompt
    }
}

#[derive(Clone, Default, Debug)]
pub struct PromptBuf {
    pub prompt: String,
    pub input: String,
    pub message: String,
}

impl<T> From<T> for PromptBuf where T: Into<String> {
    fn from(value: T) -> Self {
        PromptBuf {
            prompt: value.into(),
            input: "".into(),
            message: "".into()
        }
    }
}

pub struct Prompt {
    pub buf: PromptBuf,
    prompts: Arc<Mutex<Vec<ReadPrompt>>>,
}

unsafe impl SystemParam for Prompt {
    type State = PromptProvider;
    type Item<'w, 's> = Prompt;

    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        world.get_resource_mut::<PromptProvider>().unwrap().clone()
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        state.new_prompt()
    }
}

impl Prompt {
    fn new(prompts: Arc<Mutex<Vec<ReadPrompt>>>) -> Self {
        Self {
            buf: PromptBuf {
                prompt: String::from(""),
                message: String::from(""),
                input: String::from(""),
            },
            prompts,
        }
    }
}

trait NanoPrompt {
    // type Output<T> = Result<T, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf);
    fn buf_write(&mut self, buf: &PromptBuf); // -> Result<(),
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError>;

    async fn read_string<T: Into<PromptBuf>>(&mut self, prompt: T) -> Result<String, NanoError> {
        let buf = prompt.into();
        // self.buf_read(&mut buf);
        // buf.input.clear();
        // buf.prompt = prompt.to_owned();
        self.buf_write(&buf);
        self.read_raw().await.map(|prompt_buf| prompt_buf.input)
    }

    async fn read<T: LookUp>(&mut self, prompt: impl Into<PromptBuf>) -> Result<T, NanoError> {
        let buf = prompt.into();
        self.buf_write(&buf);
        loop {
            match self.read_raw().await {
                Ok(mut new_buf) => match T::look_up(&new_buf.input) {
                    Ok(v) => return Ok(v),
                    Err(LookUpError::Message(m)) => {
                        new_buf.message = m;
                        self.buf_write(&new_buf);
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        }
    }
}

async fn read_int(prompt: &mut impl NanoPrompt, label: &str) -> Result<i32, NanoError> {
    let mut buf = PromptBuf::default();
    buf.prompt = label.to_owned();
    prompt.buf_write(&buf);
    loop {
        match prompt.read_raw().await {
            Ok(mut new_buf) => match new_buf.input.parse::<i32>() {
                Ok(int) => return Ok(int),
                Err(e) => {
                    new_buf.message = format!(" expected int: {}", e);
                    prompt.buf_write(&new_buf);
                }
            },
            Err(e) => return Err(e),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum PromptState {
    #[default]
    // Uninit,
    Invisible,
    Visible,
}

impl NanoPrompt for Prompt {
    // type Output<T> = Consumer<T, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf) {
        buf.clone_from(&self.buf);

    }
    fn buf_write(&mut self, buf: &PromptBuf) {
        self.buf.clone_from(&buf);
    }
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError> {
        let (promise, waiter) = Producer::<PromptBuf, NanoError>::new();
        self.prompts.lock().unwrap().push(ReadPrompt {
            prompt: self.buf.clone(),
            promise: promise,
            active: false,
        });
        return waiter.await;
    }
}

#[allow(dead_code)]
enum LookUpError {
    Message(String),
    NanoError(NanoError),
}

trait LookUp: Sized {
    fn look_up(input: &str) -> Result<Self, LookUpError>;
}

impl LookUp for () {
    fn look_up(_: &str) -> Result<Self, LookUpError> {
        Ok(())
    }
}

impl LookUp for String {
    fn look_up(input: &str) -> Result<Self, LookUpError> {
        Ok(input.to_owned())
    }
}

impl LookUp for i32 {
    fn look_up(input: &str) -> Result<Self, LookUpError> {
        match input.parse::<i32>() {
            Ok(int) => Ok(int),
            Err(e) => Err(LookUpError::Message(format!(" expected int: {}", e))),
        }
    }
}

#[derive(Component)]
struct TaskSink(Task<()>);

impl TaskSink {
    fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(future);
        Self(task)
    }
}

trait AddCommand {
    // fn add_command<Params>(&mut self, system: impl IntoSystemConfigs<Params>) -> &mut Self;
    fn add_command<Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        system: impl IntoSystem<(), (), Marker> + 'static,
    ) -> &mut Self;
}

impl AddCommand for App {
    fn add_command<Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        system: impl IntoSystem<(), (), Marker> + 'static,
    ) -> &mut Self {
        let system: Box<dyn System<In = (), Out = ()> + 'static> =
            Box::new(IntoSystem::into_system(system));
        // let name = system.name();
        self.add_systems(CommandOneShot(name.into()), system);
        self
    }
}

fn main() {
    App::new()
        .add_event::<RunCommandEvent>()
        .add_state::<PromptState>()
        .init_resource::<PromptProvider>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy Nano Prompt Example".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, spawn_layout)
        .add_systems(OnEnter(PromptState::Visible), show_prompt)
        .add_systems(OnExit(PromptState::Visible), hide_prompt_delayed)
        .add_systems(Update, hide_prompt_maybe)
        .add_systems(Update, prompt_input)
        .add_systems(Update, poll_tasks)
        .add_systems(PreUpdate, run_commands)
        // .add_command("ask_name", ask_name3)
        // .add_command("ask_name", ask_name4.pipe(task_sink))
        .add_command("ask_name", ask_name5.pipe(task_sink))
        // .add_command("ask_name", ask_name6.pipe(task_sink))
        .add_command("ask_age", ask_age.pipe(task_sink))
        .add_command("ask_age2", ask_age2.pipe(task_sink))
        .run();
}

fn run_commands(world: &mut World) {
    let mut event_system_state = SystemState::<EventReader<RunCommandEvent>>::new(world);
    let schedules: Vec<Box<dyn ScheduleLabel>> = {
        let mut events = event_system_state.get_mut(world);
        events.iter().map(|e| e.0.clone()).collect()
    };

    for schedule in schedules {
        match world.try_run_schedule(schedule) {
            Err(e) => eprintln!("Problem running command: {:?}", e),
            _ => {}
        }
    }
}

fn poll_tasks(mut commands: Commands, mut command_tasks: Query<(Entity, &mut TaskSink)>) {
    for (entity, mut task) in &mut command_tasks {
        if let Some(_) = future::block_on(future::poll_once(&mut task.0)) {
            // Once
            //
            commands.entity(entity).despawn();
        }
    }
}
// [[https://bevy-cheatbook.github.io/programming/local.html][Local Resources - Unofficial Bevy Cheat Book]]i

/// prints every char coming in; press enter to echo the full string
fn prompt_input(
    prompt_provider: ResMut<PromptProvider>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Text, With<PromptNode>>,
    // mut text_prompt: TextPrompt,
) {
    if keys.just_pressed(KeyCode::Tab) {
        println!("tab pressed");
        run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_name".into()))));
        return;
    }

    if keys.just_pressed(KeyCode::Key1) {
        run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_age2".into()))));
        return;
    }

    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();
    for mut text in query.iter_mut() {
        let len = prompts.len();
        if prompts.len() > 0 {
            let mut text_prompt = TextPrompt { text: &mut text };

            if keys.just_pressed(KeyCode::Escape) {
                let message = text_prompt.message_get_mut();
                *message = " Quit".into();
                let promise = {
                    let read_prompt = prompts.pop().unwrap();
                    read_prompt.promise
                };
                promise.reject(NanoError::Cancelled);
                if prompts.len() == 0 {
                    show_prompt.set(PromptState::Invisible);
                }
                continue;
            }
            if keys.just_pressed(KeyCode::Return) {
                let mut buf = PromptBuf::default();
                text_prompt.buf_read(&mut buf);
                let promise = {
                    let read_prompt = prompts.pop().unwrap();
                    read_prompt.promise
                };
                promise.resolve(buf);
                text_prompt.prompt_get_mut().clear();
                text_prompt.input_get_mut().clear();
                text_prompt.message_get_mut().clear();
                if prompts.len() == 0 {
                    show_prompt.set(PromptState::Invisible);
                }
                continue;
            }
            let active = prompts.last().unwrap().active;
            if ! active {
                // Must set it up.
                if len > 1 {
                    if let Some(last) = prompts.get_mut(len - 2) {
                        // Record last prompt.
                        text_prompt.buf_read(&mut last.prompt);
                        eprintln!("last prompt {:?}", last.prompt);
                    }
                }
                for i in 0..len - 1 {
                    prompts[i].active = false;
                }
                let read_prompt = prompts.last_mut().unwrap();
                text_prompt.buf_write(&read_prompt.prompt);
                read_prompt.active = true;
                show_prompt.set(PromptState::Visible);
            }
            if keys.just_pressed(KeyCode::Back) {
                let _ = text_prompt.input_get_mut().pop();
                text_prompt.message_get_mut().clear();
                continue;
            }
            if char_evr.len() > 0 {
                text_prompt
                    .input_get_mut()
                    .extend(char_evr.iter().map(|ev| ev.char));
                text_prompt.message_get_mut().clear();
            }
        }
    }
}

struct TextPrompt<'a> {
    text: &'a mut Text,
}

#[allow(dead_code)]
impl<'a> TextPrompt<'a> {
    fn prompt_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[0].value
    }
    fn prompt_get(&self) -> &str {
        &self.text.sections[0].value
    }
    fn input_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[1].value
    }
    fn input_get(&self) -> &str {
        &self.text.sections[1].value
    }
    fn message_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[2].value
    }
    fn message_get(&self) -> &str {
        &self.text.sections[2].value
    }
}

impl<'a> NanoPrompt for TextPrompt<'a> {
    fn buf_read(&self, buf: &mut PromptBuf) {
        buf.prompt.clone_from(&self.text.sections[0].value);
        buf.input.clone_from(&self.text.sections[1].value);
        buf.message.clone_from(&self.text.sections[2].value);
    }
    fn buf_write(&mut self, buf: &PromptBuf) {
        self.text.sections[0].value.clone_from(&buf.prompt);
        self.text.sections[1].value.clone_from(&buf.input);
        self.text.sections[2].value.clone_from(&buf.message);
    }
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError> {
        panic!("Not sure this should ever be called.");
    }
}

fn show_prompt(mut query: Query<&mut Visibility, With<PromptContainer>>) {
    let mut visibility = query.single_mut();
    *visibility = Visibility::Visible;
}

#[derive(Component)]
struct HideTime {
    timer: Timer,
}

fn hide_prompt_delayed(mut commands: Commands, query: Query<Entity, With<PromptContainer>>) {
    let id = query.single();
    commands.entity(id).insert(HideTime {
        timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
    });
}

fn hide_prompt_maybe(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<PromptState>>,
    mut query: Query<(Entity, &mut Visibility, &mut HideTime)>,
) {
    for (id, mut visibility, mut hide) in query.iter_mut() {
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == PromptState::Invisible {
                *visibility = Visibility::Hidden;
            }
            commands.entity(id).remove::<HideTime>();
        }
    }
}

#[allow(dead_code)]
fn hide_prompt(mut query: Query<&mut Visibility, With<PromptContainer>>) {
    let mut visibility = query.single_mut();
    *visibility = Visibility::Hidden;
}

#[allow(dead_code)]
async fn ask_name2(mut prompt: impl NanoPrompt) {
    println!("ask name 2 called");
    if let Ok(name) = prompt.read_string("What's your first name? ").await {
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
        if let Ok(first_name) = prompt.read_string("What's your first name? ").await {
            if let Ok(last_name) = prompt.read_string("What's your last name? ").await {
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
        if let Ok(first_name) = prompt.read_string("What's your first name? ").await {
            if let Ok(last_name) = prompt.read_string("What's your last name? ").await {
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

// fn ask_name6<'a>(mut prompt: TextPromptParam) -> impl Future<Output = ()> {
//     println!("ask name 5 called");
//     async move {
//         if let Ok(first_name) = prompt.read_string("What's your first name? ").await {
//             if let Ok(last_name) = prompt.read_string("What's your last name? ").await {
//                 println!("Hello, {} {}", first_name, last_name);
//             }
//         } else {
//             println!("Got err in ask now");
//         }
//     }
// }

#[allow(dead_code)]
fn ask_age(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask age called");
    async move {
        if let Ok(age) = read_int(&mut prompt, "What's your age? ").await {
            println!("You are {} years old.", age);
        } else {
            println!("Got err in ask age");
        }
    }
}

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

fn task_sink<T: Future<Output = ()> + Send + 'static>(In(future): In<T>, mut commands: Commands) {
    commands.spawn(TaskSink::new(async move { future.await }));
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            visibility: Visibility::Hidden,
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                right: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                flex_grow: 1.,
                padding: UiRect {
                    top: PADDING,
                    left: PADDING,
                    right: PADDING,
                    bottom: PADDING,
                },
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .insert(PromptContainer {})
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
                            color: Color::GRAY,
                        },
                    ),
                    TextSection::new(
                        " message",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::YELLOW,
                        },
                    ),
                    // This is a dummy section to keep the line height stable.
                    TextSection::new(
                        " ",
                        TextStyle {
                            font,
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ),
                ]))
                .insert(PromptNode {});
        });
}

#[cfg(test)]
mod tests {

    #[allow(unused_must_use)]
    #[test]
    fn test_option_default() {
        let a: Option<PromptCel> = default();
    }
}
