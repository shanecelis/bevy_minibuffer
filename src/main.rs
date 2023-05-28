use bevy::ecs::prelude::Commands;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{CommandQueue, SystemState, SystemParam, SystemMeta};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::Duration;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::ecs::component::Tick;
use futures_lite::future;
use once_cell::sync::OnceCell;
use promise_out::{Promise, pair::{Producer, Consumer}};
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};

const MARGIN: Val = Val::Px(5.);
const PADDING: Val = Val::Px(3.);

struct RunCommandEvent(Box<dyn ScheduleLabel>);

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandOneShot(Cow<'static, str>);

#[derive(Component)]
struct PromptContainer;
#[derive(Component)]
struct PromptNode;

#[derive(Debug)]
enum NanoError {
    Cancelled,
    Message(Cow<'static, str>),
}

struct ReadPrompt {
    prompt: PromptBuf,
    active: bool,
    promise: Producer<String, NanoError>,
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

#[derive(Clone, Default)]
pub struct PromptBuf {
    pub prompt: String,
    pub message: String,
    pub input: String,
}

// #[derive(Default)]
pub struct Prompt {
  pub buf: PromptBuf,
  prompts: Arc<Mutex<Vec<ReadPrompt>>>,
}

#[derive(Default)]
struct MyOption<T>(Option<T>);

// #[derive(SystemParam)]
// pub struct Prompt<'w> {
//   pub prompt_provider: ResMut<'w, PromptProvider>,
//   #[system_param(ignore)]
//   cell: Option<Prompt>,
//   // pub what: bool
// }

unsafe impl SystemParam for Prompt {
    type State = PromptProvider;
    type Item<'w, 's> = Prompt;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        world.get_resource_mut::<PromptProvider>().unwrap().clone()
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        state.new_prompt()
    }
}

impl Prompt {
    fn new(prompts: Arc<Mutex<Vec<ReadPrompt>>>) -> Self {
        Self {
            buf: PromptBuf {prompt: String::from(""),
                            message: String::from(""),
                            input: String::from("") },
            prompts,
        }
    }
}

trait NanoPrompt {
    type Output: Future<Output = Result<String, NanoError>>;

    fn buf_read(&self, buf: &mut PromptBuf);
    fn buf_write(&mut self, buf: &PromptBuf);// -> Result<(),
    fn read(&mut self) -> Self::Output;

    fn read_string(&mut self, prompt: &str) -> Self::Output {
        let mut buf = PromptBuf::default();
        self.buf_read(&mut buf);
        buf.input.clear();
        buf.prompt = prompt.to_owned();
        self.buf_write(&mut buf);
        self.read()
    }
}

async fn read_int(prompt: &mut impl NanoPrompt, label: &str) -> Result<i32, NanoError> {
  loop {
    let mut buf = PromptBuf::default();
    buf.prompt = label.to_owned();
    prompt.buf_write(&buf);
    match prompt.read().await {
      Ok(str) => {
        match str.parse::<i32>() {
          Ok(int) => return Ok(int),
          Err(e) => {
              prompt.buf_read(&mut buf);
              buf.message = format!(" expected int: {}", e);
              prompt.buf_write(&buf);
          }
        }
      }
      Err(e) => return Err(e)
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
    type Output = Consumer<String, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf) {
      buf.clone_from(&self.buf);

    }
    fn buf_write(&mut self, buf: &PromptBuf) {
      self.buf.clone_from(&buf);
    }
    fn read(&mut self) -> Self::Output {
        let (promise, waiter) = Producer::new();
        self.prompts.lock().unwrap().push(ReadPrompt {
            prompt: self.buf.clone(),
            promise: promise,
            active: false,
        });
        return waiter;
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
        .add_startup_system(spawn_layout)
        .add_systems(OnEnter(PromptState::Visible), show_prompt)
        .add_systems(OnExit(PromptState::Visible), hide_prompt_delayed)
        .add_system(hide_prompt_maybe)
        .add_system(prompt_input)
        .add_system(poll_tasks)
        .add_systems(PreUpdate, run_commands)
        // .add_command("ask_name", ask_name3)
        // .add_command("ask_name", ask_name4.pipe(task_sink))
        .add_command("ask_name", ask_name5.pipe(task_sink))
        // .add_command("ask_name", ask_name6.pipe(task_sink))
        // .add_command("ask_age", ask_age.pipe(task_sink))
        .run();
}

fn run_commands(world: &mut World) {
    let mut event_system_state = SystemState::<(EventReader<RunCommandEvent>)>::new(world);
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
            commands.entity(entity).despawn();
        }
    }
}
// [[https://bevy-cheatbook.github.io/programming/local.html][Local Resources - Unofficial Bevy Cheat Book]]i

/// prints every char coming in; press enter to echo the full string
fn prompt_input(
    mut commands: Commands,
    mut prompt_provider: ResMut<PromptProvider>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut run_command: EventWriter<RunCommandEvent>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Text, With<PromptNode>>,
    // mut text_prompt: TextPrompt,
) {
    if keys.just_pressed(KeyCode::Tab) {
        println!("tab pressed");
        // commands.spawn(TaskSink::new(ask_name()));
        // commands.spawn(TaskSink::new(ask_name3(prompt_provider.new_prompt())));
        run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_name".into()))));
        // run_command.send(RunCommandEvent(Box::new(CommandOneShot("ask_age".into()))));
        return;
    }

    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();
    for mut text in query.iter_mut() {
        if prompts.len() > 0 {
            let mut text_prompt = TextPrompt { text: &mut text };

            if keys.just_pressed(KeyCode::Escape) {
                // let mut buf = PromptBuf::default();
                // let result = buf.input.clone();
                // println!("Cancel prompt");
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
                // return;
            }
            if keys.just_pressed(KeyCode::Return) {
                // let mut buf = PromptBuf::default();
                let result = text_prompt.input_get().to_owned();
                // let result = buf.input.clone();
                // println!("Got result {}", result);
                let promise = {
                    let read_prompt = prompts.pop().unwrap();
                    read_prompt.promise
                };
                promise.resolve(result);
                if prompts.len() == 0 {
                    show_prompt.set(PromptState::Invisible);
                }
                continue;
                // return;
            }
            let read_prompt = prompts.last_mut().unwrap();
            if !read_prompt.active {
                // Must set it up.
                text_prompt.buf_write(&read_prompt.prompt);
                read_prompt.active = true;
                for i in 0..prompts.len() - 1 {
                    prompts[i].active = false;
                }
                show_prompt.set(PromptState::Visible);
            }
            if keys.just_pressed(KeyCode::Back) {
                let mut buf = PromptBuf::default();
                text_prompt.buf_read(&mut buf);
                let _ = buf.input.pop();
                text_prompt.buf_write(&buf);
                continue;
              // return;
            }
            text_prompt
                .input_get_mut()
                .extend(char_evr.iter().map(|ev| ev.char));
        }
    }
}

struct TextPrompt<'a> {
    text: &'a mut Text,
    // #[system_param(ignore)]
    // marker: PhantomData<Marker>,
}

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

#[derive(SystemParam)]
pub struct TextPromptParam<'w, 's> {
    query: Query<'w, 's, &'static mut Text, With<PromptNode>>,
}

impl<'w, 's> NanoPrompt for TextPromptParam<'w, 's> {
    type Output = Consumer<String, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf) {
      let text = self.query.single();
      buf.prompt.clone_from(&text.sections[0].value);
      buf.input.clone_from(&text.sections[1].value);
      buf.message.clone_from(&text.sections[2].value);
    }
    fn buf_write(&mut self, buf: &PromptBuf) {
      let mut text = self.query.single_mut();
      text.sections[0].value.clone_from(&buf.prompt);
      text.sections[1].value.clone_from(&buf.input);
      text.sections[2].value.clone_from(&buf.message);
    }
    fn read(&mut self) -> Self::Output {
        panic!("Not sure this should ever be called.");
    }
}
// struct TextPromptParam<'w, 's> {
//     query: Query<'w, 's, &'static mut Text, With<PromptNode>>,
// }
// const _: () = {
//     type __StructFieldsAlias<'w, 's> = (
//         Query<'w, 's, &'static mut Text, With<PromptNode>>,
//     );
//     #[doc(hidden)]
//     struct FetchState {
//         state: <__StructFieldsAlias<
//             'static,
//             'static,
//         > as bevy::ecs::system::SystemParam>::State,
//     }
//     unsafe impl bevy::ecs::system::SystemParam for TextPromptParam<'_, '_> {
//         type State = FetchState;
//         type Item<'w, 's> = TextPromptParam<'w, 's>;
//         fn init_state(
//             world: &mut bevy::ecs::world::World,
//             system_meta: &mut bevy::ecs::system::SystemMeta,
//         ) -> Self::State {
//             FetchState {
//                 state: <__StructFieldsAlias<
//                     '_,
//                     '_,
//                 > as bevy::ecs::system::SystemParam>::init_state(world, system_meta),
//             }
//         }
//         fn new_archetype(
//             state: &mut Self::State,
//             archetype: &bevy::ecs::archetype::Archetype,
//             system_meta: &mut bevy::ecs::system::SystemMeta,
//         ) {
//             <__StructFieldsAlias<
//                 '_,
//                 '_,
//             > as bevy::ecs::system::SystemParam>::new_archetype(
//                 &mut state.state,
//                 archetype,
//                 system_meta,
//             )
//         }
//         fn apply(
//             state: &mut Self::State,
//             system_meta: &bevy::ecs::system::SystemMeta,
//             world: &mut bevy::ecs::world::World,
//         ) {
//             <__StructFieldsAlias<
//                 '_,
//                 '_,
//             > as bevy::ecs::system::SystemParam>::apply(
//                 &mut state.state,
//                 system_meta,
//                 world,
//             );
//         }
//         unsafe fn get_param<'w, 's>(
//             state: &'s mut Self::State,
//             system_meta: &bevy::ecs::system::SystemMeta,
//             world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
//             change_tick: bevy::ecs::component::Tick,
//         ) -> Self::Item<'w, 's> {
//             let (f0,) = <(
//                 Query<'w, 's, &'static mut Text, With<PromptNode>>,
//             ) as bevy::ecs::system::SystemParam>::get_param(
//                 &mut state.state,
//                 system_meta,
//                 world,
//                 change_tick,
//             );
//             TextPromptParam { query: f0 }
//         }
//     }
//     unsafe impl<'w, 's> bevy::ecs::system::ReadOnlySystemParam
//     for TextPromptParam<'w, 's>
//     where
//         Query<
//             'w,
//             's,
//             &'static mut Text,
//             With<PromptNode>,
//         >: bevy::ecs::system::ReadOnlySystemParam,
//     {}
// };

// unsafe impl SystemParam for TextPrompt<'_> {
//     // type State = TextPromptParam<'w, 's>;
//     type State = ();
//     type Item<'world, 'state> = TextPrompt<'world>;

//     fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
//       // TextPromptParam { query: world.query::<'w, 's, &'static mut Text, With<PromptNode>>() }
//     }

//     #[inline]
//     unsafe fn get_param<'world, 'state>(
//         state: &'state mut Self::State,
//         system_meta: &SystemMeta,
//         world: UnsafeWorldCell<'world>,
//         change_tick: Tick,
//     ) -> Self::Item<'world, 'state> {
//         let world = world.world_mut();
//         let mut query = world.query::<'world, 'state, (&'static mut Text, With<PromptNode>)>();
//         let (mut text, _) = query.single_mut(world);
//         // let text = state.get_mut(&mut world);
//       TextPrompt { text }
//     }
// }

impl<'a> NanoPrompt for TextPrompt<'a> {
    type Output = Consumer<String, NanoError>;

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
    fn read(&mut self) -> Self::Output {
        panic!("Not sure this should ever be called.");
    }
}

fn show_prompt(mut query: Query<(&mut Visibility, &PromptContainer)>) {
    let (mut visibility, prompt) = query.single_mut();
    *visibility = Visibility::Visible;
}

#[derive(Component)]
struct HideTime {
  timer: Timer,
}

fn hide_prompt_delayed(mut commands: Commands,
                       mut query: Query<(Entity, &PromptContainer)>) {
    let (id, prompt) = query.single();
    commands.entity(id).insert(HideTime { timer: Timer::new(Duration::from_secs(1), TimerMode::Once) } );
}

fn hide_prompt_maybe(mut commands: Commands,
                     time: Res<Time>,
                     state: Res<State<PromptState>>,
                     mut query: Query<(Entity, &mut Visibility, &mut HideTime)>) {
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

fn hide_prompt(mut query: Query<(&mut Visibility, &PromptContainer)>) {
    let (mut visibility, prompt) = query.single_mut();
    *visibility = Visibility::Hidden;
}

async fn ask_name2(mut prompt: impl NanoPrompt) {
    println!("ask name 2 called");
    if let Ok(name) = prompt.read_string("What's your first name? ").await {
        println!("Hello, {}", name);
    } else {
        println!("Got err in ask now");
    }
}

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
        if let Ok(first_name) = prompt.read_string("What's your first name? ").await {
            if let Ok(last_name) = prompt.read_string("What's your last name? ").await {
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

// fn ask_age(mut prompt: TextPromptParam) -> impl Future<Output = ()> {
//     println!("ask age called");
//     async move {
//         if let Ok(age) = read_int(&mut prompt, "What's your age? ").await {
//             println!("You are {} years old.", age);
//         } else {
//             println!("Got err in ask age");
//         }
//     }
// }

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
                .insert(PromptNode {});
        });
}


#[cfg(test)]
mod tests {

#[allow(unused_must_use)]
#[test]
fn test_option_default() {
  let a : Option<PromptCel> = default();
}
}
