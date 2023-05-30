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
use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    winit::WinitSettings,
};
use futures_lite::future;
use promise_out::{pair::Producer, Promise};
use std::borrow::Cow;
use std::future::Future;
use std::sync::{Arc, Mutex};
use bitflags::bitflags;

// const MARGIN: Val = Val::Px(5.);
const PADDING: Val = Val::Px(3.);
const LEFT_PADDING: Val = Val::Px(6.);

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
    prior: Option<PromptBuf>,
    promise: Producer<PromptBuf, NanoError>,
}

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
    selection: Option<usize>,
    last_selection: Option<usize>,
}

#[derive(Resource, Debug, Default)]
struct CommandConfig {
    commands: Vec<Command>
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
    pub completion: Option<Vec<String>>,
}

impl<T> From<T> for PromptBuf
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        PromptBuf {
            prompt: value.into(),
            input: "".into(),
            message: "".into(),
            completion: None,
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
                completion: None,
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

    async fn read<T: LookUp>(&mut self, prompt: impl Into<PromptBuf>) -> Result<T, NanoError> {
        let buf = prompt.into();
        self.buf_write(&buf);
        loop {
            match self.read_raw().await {
                Ok(mut new_buf) => match T::look_up(&new_buf.input) {
                    Ok(v) => return Ok(v),
                    Err(LookUpError::Message(m)) => {
                        new_buf.message = m.to_string();
                        self.buf_write(&new_buf);
                    }
                    Err(LookUpError::Incomplete(v)) => {
                        new_buf.completion = Some(v);
                        self.buf_write(&new_buf);
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        }
    }

    async fn read_crit<T>(&mut self, prompt: impl Into<PromptBuf>, look_up: &impl LookUpObject<Item=T>) -> Result<T, NanoError> {
        let buf = prompt.into();
        self.buf_write(&buf);
        loop {
            match self.read_raw().await {
                Ok(mut new_buf) => match look_up.look_up(&new_buf.input) {
                    Ok(v) => return Ok(v),
                    Err(LookUpError::Message(m)) => {
                        new_buf.message = m.to_string();
                        self.buf_write(&new_buf);
                    }
                    Err(LookUpError::Incomplete(v)) => {
                        new_buf.completion = Some(v);
                        self.buf_write(&new_buf);
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
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
            prior: None,
        });
        return waiter.await;
    }
}

#[allow(dead_code)]
enum LookUpError {
    Message(Cow<'static, str>),
    NanoError(NanoError),
    Incomplete(Vec<String>),
}

// impl LookUpObject for &[String] {
//     type Item = String;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         let matches: Vec<&String> = self.iter()
//                                        .filter(|word| word.starts_with(input))
//                                        .collect();
//         if matches.len() == 1 {
//             Ok(matches[0].clone())
//         } else if matches.len() > 1 {
//             Err(LookUpError::Incomplete(matches.into_iter().map(|s| s.clone()).collect()))
//         } else {
//             Err(LookUpError::Message(" no matches".into()))
//         }
//     }
// }

// impl LookUpObject for &[&str] {
//     type Item = String;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         let matches: Vec<&&str> = self.iter()
//                                        .filter(|word| word.starts_with(input))
//                                        .collect();
//         if matches.len() == 1 {
//             Ok(matches[0].to_string())
//         } else if matches.len() > 1 {
//             Err(LookUpError::Incomplete(matches.into_iter().map(|s| s.to_string()).collect()))
//         } else {
//             Err(LookUpError::Message(" no matches".into()))
//         }
//     }
// }

impl<T: AsRef<str>> LookUpObject for &[T] {
    type Item = String;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        let matches: Vec<&str> = self.iter()
                                       .map(|word| word.as_ref())
                                       .filter(|word| word.starts_with(input))
                                       .collect();
        if matches.len() == 1 {
            Ok(matches[0].to_string())
        } else if matches.len() > 1 {
            Err(LookUpError::Incomplete(matches.into_iter().map(|s| s.to_string()).collect()))
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }
}


// impl<'a> LookUpObject for &[Cow<'a, str>] {
//     type Item = String;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         let matches: Vec<&&str> = self.iter()
//                                        .filter(|word| word.starts_with(input))
//                                        .collect();
//         if matches.len() == 1 {
//             Ok(matches[0].to_string())
//         } else if matches.len() > 1 {
//             Err(LookUpError::Incomplete(matches.into_iter().map(|s| s.to_string()).collect()))
//         } else {
//             Err(LookUpError::Message(" no matches".into()))
//         }
//     }
// }


trait LookUpObject: Sized {
    type Item;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<T> LookUpObject for T where T : LookUp {
    type Item = T;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        T::look_up(input)
    }
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
            Err(e) => Err(LookUpError::Message(format!(" expected int: {}", e).into())),
        }
    }
}

struct TomDickHarry(String);

impl LookUp for TomDickHarry {
    fn look_up(input: &str) -> Result<Self, LookUpError> {
        match input {
            "Tom" => Ok(TomDickHarry(input.into())),
            "Dick" => Ok(TomDickHarry(input.into())),
            "Harry" => Ok(TomDickHarry(input.into())),
            _ => Err(LookUpError::Incomplete(vec!["Tom".into(), "Dick".into(), "Harry".into()]))
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
bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const Alt     = 0b00000001;
        const Control = 0b00000010;
        const Shift   = 0b00000100;
        const System  = 0b00001000; // Windows or Command
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub mods: Modifiers,
    pub key: KeyCode,
}
type KeySeq = Vec<Key>;

impl From<KeyCode> for Key {
    fn from(v: KeyCode) -> Self {
        Key {
            key: v,
            mods: Modifiers::empty()
        }
    }
}

impl Modifiers {
    fn from_input(input: &Res<Input<KeyCode>>) -> Modifiers {
        let mut mods = Modifiers::empty();
        if input.any_pressed([KeyCode::LShift, KeyCode::RShift]) {
            mods |= Modifiers::Shift;
        }
        if input.any_pressed([KeyCode::LControl, KeyCode::RControl]) {
            mods |= Modifiers::Control;
        }
        if input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]) {
            mods |= Modifiers::Alt;
        }
        if input.any_pressed([KeyCode::LWin, KeyCode::RWin]) {
            mods |= Modifiers::System;
        }
        mods
    }
}

#[derive(Debug, Clone)]
struct Command {
    name: Cow<'static, str>,
    hotkey: Option<Key>
}

impl Command {
    fn new(name: impl Into<Cow<'static, str>>, hotkey: Option<impl Into<Key>>) -> Self {
        Command { name: name.into(),
                  hotkey: hotkey.map(|v| v.into()) }
    }
}

impl<T> From<T> for Command where T: Into<Cow<'static, str>> {
    fn from(v: T) -> Self {
        Command {
            name: v.into(),
            hotkey: None
        }
    }
}

trait AddCommand {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        system: impl IntoSystemConfigs<Params>,
    ) -> &mut Self;
}

impl AddCommand for App {
    fn add_command<Params>(
        &mut self,
        cmd: impl Into<Command>,
        system: impl IntoSystemConfigs<Params>,
    ) -> &mut Self {
        let cmd = cmd.into();
        let name = cmd.name.clone();
        self.add_systems(CommandOneShot(name.clone()), system);
        let sys = move |mut config: ResMut<CommandConfig>| {
            if config.commands.iter().any(|i| i.name == name) {
                warn!("nano command '{name}' already registered.");
            } else {
                config.commands.push(cmd.clone());
            }
        };
        // XXX: Do these Startup systems stick around?
        self.add_systems(Startup, sys);
        self
    }
}

fn main() {
    App::new()
        .add_event::<RunCommandEvent>()
        .add_state::<PromptState>()
        .init_resource::<PromptProvider>()
        .init_resource::<CommandConfig>()
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
        .add_systems(Update, mouse_scroll)
        .add_systems(Update, hotkey_input)
        // .add_command("ask_name", ask_name3)
        // .add_command("ask_name", ask_name4.pipe(task_sink))
        // .add_command("ask_name", ask_name5.pipe(task_sink))
        .add_command("ask_name", ask_name6.pipe(task_sink))
        // .add_command("ask_name", ask_name6.pipe(task_sink))
        // .add_command("ask_age", ask_age.pipe(task_sink))
        .add_command("ask_age2", ask_age2.pipe(task_sink))
        .add_command(Command::new("exec_command", Some(KeyCode::Semicolon)), exec_command.pipe(task_sink))
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

fn hotkey_input(
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    config: Res<CommandConfig>)
{
    let mods = Modifiers::from_input(&keys);
    for command in &config.commands {
        if let Some(ref key) = command.hotkey {
            if key.mods == mods && keys.just_pressed(key.key) {
                eprintln!("We were called for {}", command.name);
            }
        }
    }
}

/// prints every char coming in; press enter to echo the full string
fn prompt_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    prompt_provider: ResMut<PromptProvider>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Text, With<PromptNode>>,
    completion: Query<(Entity, &Children), With<ScrollingList>>,
    // mut text_prompt: TextPrompt,
) {
    if keys.just_pressed(KeyCode::Tab) {
        println!("tab pressed");
        run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_name".into()))));
        return;
    }

    // if keys.just_pressed(KeyCode::Semicolon) {
    //     run_command.send(RunCommandEvent(Box::new(CommandOneShot("exec_command".into()))));
    //     return;
    // }
    if keys.just_pressed(KeyCode::Key1) {
        run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_age2".into()))));
        return;
    }

    // eprintln!("chars {:?}", char_evr.iter().map(|ev| ev.char).collect::<Vec<_>>());
    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();
    let (completion_node, children) = completion.single();
    let children: Vec<Entity> = children.to_vec();
    for mut text in query.iter_mut() {
        let len = prompts.len();
        if prompts.len() > 0 {

            let font = asset_server.load("fonts/FiraSans-Bold.ttf");
            let mut text_prompt = TextPrompt { text: &mut text, completion: completion_node,
                                               children: &children, font: font.clone(),
                                               commands: &mut commands };

            if keys.just_pressed(KeyCode::Escape) {
                let message = text_prompt.message_get_mut();
                *message = " Quit".into();
                let promise = {
                    let read_prompt = prompts.pop().unwrap();
                    read_prompt.promise
                };
                promise.reject(NanoError::Cancelled);
                if prompts.len() == 0 {
                    // text_prompt.prompt_get_mut().clear();
                    // text_prompt.input_get_mut().clear();
                    // text_prompt.message_get_mut().clear();
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
                if prompts.len() == 0 {
                    // This causes a one frame flicker.
                    // text_prompt.prompt_get_mut().clear();
                    // text_prompt.input_get_mut().clear();
                    // text_prompt.message_get_mut().clear();
                    show_prompt.set(PromptState::Invisible);
                }
                continue;
            }
            let active = prompts.last().unwrap().active;
            if !active {
                // Must set it up.
                if len > 1 {
                    if let Some(last) = prompts.get_mut(len - 2) {
                        // Record last prompt.
                        if last.prior.is_none() {
                            let mut buf: PromptBuf = default();
                            text_prompt.buf_read(&mut buf);
                            eprintln!("record last prompt {:?}", buf);
                            last.prior = Some(buf);
                        }
                    }
                }
                for i in 0..len - 1 {
                    prompts[i].active = false;
                }
                let read_prompt = prompts.last_mut().unwrap();
                let buf = read_prompt.prior.take().unwrap_or_else(|| read_prompt.prompt.clone());

                eprintln!("setup new prompt {:?}", buf);
                text_prompt.buf_write(&buf);
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

struct TextPrompt<'a, 'w, 's> {
    text: &'a mut Text,
    completion: Entity,
    children: &'a [Entity],
    commands: &'a mut Commands<'w, 's>,
    font: Handle<Font>

}

#[allow(dead_code)]
impl<'a, 'w, 's> TextPrompt<'a, 'w, 's> {
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

impl<'a, 'w, 's> NanoPrompt for TextPrompt<'a, 'w, 's> {
    fn buf_read(&self, buf: &mut PromptBuf) {
        buf.prompt.clone_from(&self.text.sections[0].value);
        buf.input.clone_from(&self.text.sections[1].value);
        buf.message.clone_from(&self.text.sections[2].value);
    }
    fn buf_write(&mut self, buf: &PromptBuf) {
        self.text.sections[0].value.clone_from(&buf.prompt);
        self.text.sections[1].value.clone_from(&buf.input);
        self.text.sections[2].value.clone_from(&buf.message);
        if let Some(values) = &buf.completion {
            let new_children: Vec<Entity> = values.clone().into_iter().map(|label|
                                                              self.commands.spawn(completion_item(label,
                                                                                       Color::WHITE,
                                                                                       self.font.clone()))
            .id()).collect();

            self.commands.entity(self.completion).replace_children(&new_children);
            for child in self.children.iter() {
                self.commands.entity(*child).despawn();
            }
            // self.node.remove_children(self.children);
        }
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

fn ask_name6<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
    println!("ask name 6 called");
    async move {
        if let Ok(TomDickHarry(first_name)) = prompt.read("What's your first name? ").await {
            println!("Hello, {}", first_name);
        } else {
            println!("Got err in ask now");
        }
    }
}

fn exec_command(mut prompt: Prompt,
                    config: Res<CommandConfig>
) -> impl Future<Output = ()> {
    let commands: Vec<_> = config.commands.clone().into_iter().map(|c| c.name).collect();
    async move {
        if let Ok(command) = prompt.read_crit(": ", &&commands[..]).await {
            println!("COMMAND: {command}");
        } else {
            println!("Got err in ask now");
        }
    }
}

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

fn task_sink<T: Future<Output = ()> + Send + 'static>(In(future): In<T>, mut commands: Commands) {
    commands.spawn(TaskSink::new(async move { future.await }));
}

fn completion_item(label: String, color: Color, font: Handle<Font>) -> (TextBundle, Label, AccessibilityNode) {
    (TextBundle::from_section(
        label,
        TextStyle {
            font: font,
            font_size: 20.,
            color,
        },
    ),
    Label,
    AccessibilityNode(NodeBuilder::new(Role::ListItem)))
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            // visibility: Visibility::Hidden,
            style: Style {
                position_type: PositionType::Absolute,
                // top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                right: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,

                // align_items: AlignItems::FlexEnd,
                // justify_content:
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PromptContainer {})
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|builder| {
                    // List with hidden overflow
                    builder
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::FlexEnd,
                                // height: Val::Percent(50.),
                                min_width: Val::Percent(25.),
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                            background_color: Color::rgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        }).with_children(|builder| {
                    builder
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexStart,
                                    flex_grow: 0.,
                                    padding: UiRect {
                                        top: PADDING,
                                        left: LEFT_PADDING,
                                        right: PADDING * 2.,
                                        bottom: PADDING,
                                    },
                                    margin: UiRect {
                                        bottom: PADDING,
                                        ..default()
                                    },
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::BLACK),
                                ..default()
                            },
                            ScrollingList::default(),
                            AccessibilityNode(NodeBuilder::new(Role::List)),
                        ))
                        .with_children(|parent| {
                            // List items
                            for i in 0..30 {
                                parent.spawn(completion_item(format!("Item {i}"),
                                                             Color::WHITE,
                                                             font.clone()));
                            }
                        });

                    builder.spawn(NodeBundle { ..default() });
                });
                });
            builder
                .spawn(NodeBundle {
                    // visibility: Visibility::Hidden,
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.,
                        padding: UiRect {
                            top: PADDING,
                            left: LEFT_PADDING,
                            right: PADDING,
                            bottom: PADDING,
                        },
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                // .insert(PromptContainer {})
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
        });
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;

            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.top = Val::Px(scrolling_list.position);
        }
    }
}

#[cfg(test)]
mod tests {

    #[allow(unused_must_use)]
    #[test]
    fn test_option_default() {
        let a: Option<PromptCel> = default();
    }
}
