use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use bevy::ecs::component::Tick;
use bevy::ecs::prelude::Commands;
use bevy::ecs::system::{SystemMeta, SystemParam};
use bevy::window::RequestRedraw;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::utils::Duration;
use changed::Cd;

use promise_out::{pair::Producer, Promise};

use crate::ui::*;
type CowStr = Cow<'static, str>;

#[allow(dead_code)]
#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(Cow<'static, str>),
}

#[derive(Debug)]
pub struct ReadPrompt {
    prompt: Cd<PromptBuf>,
    promise: Producer<PromptBuf, NanoError>,
}

#[derive(Debug, PartialEq)]
enum ProcState {
    Uninit,
    Active
}

#[derive(Debug)]
enum ProcContent {
    Prompt(ReadPrompt),
    Message(CowStr),
}

#[derive(Debug)]
pub struct Proc(ProcContent, ProcState);

impl Proc {
    fn prompt(prompt: ReadPrompt) -> Self {
        Self(ProcContent::Prompt(prompt), ProcState::Uninit)
    }

    fn message(msg: CowStr) -> Self {
        Self(ProcContent::Message(msg), ProcState::Uninit)
    }

}

#[derive(Debug, Resource, Clone)]
pub struct ConsoleConfig {
    state: Arc<Mutex<ConsoleState>>,
}

#[derive(Debug)]
pub struct ConsoleState {
    asleep: Vec<Proc>,
    unprocessed: Vec<Proc>,
}

impl ConsoleState {
    fn new() -> Self {
        ConsoleState {
            asleep: Vec::new(),
            unprocessed: Vec::new(),
        }
    }

    fn push(&mut self, proc: Proc) {
        eprintln!("pushed");
        self.unprocessed.push(proc);
    }
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConsoleState::new()))
            // hide_delay: 1.0,
        }
    }
}

// TODO: Switch to cows or options.
#[derive(Clone, Default, Debug)]
pub struct PromptBuf {
    pub prompt: String,
    pub input: String,
    pub message: String,
    pub completion: Cd<Vec<String>>,
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
            completion: Cd::new(vec![]),
        }
    }
}

pub struct Prompt {
    pub buf: PromptBuf,
    prompts: ConsoleConfig,
}

unsafe impl SystemParam for Prompt {
    type State = ConsoleConfig;
    type Item<'w, 's> = Prompt;

    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        world.get_resource_mut::<ConsoleConfig>().unwrap().clone()
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Prompt::new(state.clone())
        // state.new_prompt()
    }
}

impl Prompt {
    fn new(prompts: ConsoleConfig) -> Self {
        Self {
            buf: default(),
            prompts,
        }
    }

    pub fn message<T: Into<Cow<'static, str>>>(&mut self, msg: T) {
        self.prompts.state.lock().unwrap().push(Proc::message(msg.into()))
    }
}

impl PromptBuf {

    fn will_update(&self,
                   char_events: &EventReader<ReceivedCharacter>,
                   keys: &Res<Input<KeyCode>>,
                   backspace: bool) -> bool {

        keys.just_pressed(KeyCode::Escape) || backspace || ! char_events.is_empty()
    }

    fn update(&mut self,
              char_events: &mut EventReader<ReceivedCharacter>,
              keys: &Res<Input<KeyCode>>,
              backspace: bool) -> Result<bool, NanoError> {

        if keys.just_pressed(KeyCode::Escape) {
            self.message = " Quit".into();
            return Err(NanoError::Cancelled);
        }
        if keys.just_pressed(KeyCode::Return) {
            return Ok(true);
        }
        // if keys.just_pressed(KeyCode::Back) {
        // if keys.pressed(KeyCode::Back) {
        if backspace {
            let _ = self.input.pop();
            self.message.clear();
            return Ok(false);
        }
        if ! char_events.is_empty() {
            self.input
                .extend(char_events.iter().map(|ev| ev.char));
            self.message.clear();
        }
        Ok(false)
    }
}

pub trait NanoPrompt {
    // type Output<T> = Result<T, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf);
    fn buf_write(&mut self, buf: &mut PromptBuf); // -> Result<(),
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError>;

    async fn read<T: LookUp>(&mut self, prompt: impl Into<PromptBuf>) -> Result<T, NanoError> {
        let mut buf = prompt.into();
        self.buf_write(&mut buf);
        loop {
            match self.read_raw().await {
                Ok(mut new_buf) => match T::look_up(&new_buf.input) {
                    Ok(v) => return Ok(v),
                    Err(LookUpError::Message(m)) => {
                        new_buf.message = m.to_string();
                        self.buf_write(&mut new_buf);
                    }
                    Err(LookUpError::Incomplete(v)) => {

                        new_buf.completion.clone_from_slice(&v[..]);
                        self.buf_write(&mut new_buf);
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        }
    }

    async fn read_crit<T>(
        &mut self,
        prompt: impl Into<PromptBuf>,
        look_up: &impl LookUpObject<Item = T>,
    ) -> Result<T, NanoError> {
        let mut buf = prompt.into();
        self.buf_write(&mut buf);
        loop {
            match self.read_raw().await {
                Ok(mut new_buf) => match look_up.look_up(&new_buf.input) {
                    Ok(v) => return Ok(v),
                    Err(LookUpError::Message(m)) => {
                        new_buf.completion.clear();
                        new_buf.message = m.to_string();
                        self.buf_write(&mut new_buf);
                    }
                    Err(LookUpError::Incomplete(v)) => {
                        new_buf.completion.clear();
                        new_buf.completion.extend_from_slice(&v[..]);
                        assert!(Cd::changed(&new_buf.completion));
                        self.buf_write(&mut new_buf);
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum PromptState {
    #[default]
    // Uninit,
    Invisible,
    Visible,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum CompletionState {
    // Uninit,
    Invisible,
    #[default]
    Visible,
}

impl NanoPrompt for Prompt {
    // type Output<T> = Consumer<T, NanoError>;

    fn buf_read(&self, buf: &mut PromptBuf) {
        buf.clone_from(&self.buf);
    }
    fn buf_write(&mut self, buf: &mut PromptBuf) {
        self.buf.clone_from(&buf);
    }
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError> {
        let (promise, waiter) = Producer::<PromptBuf, NanoError>::new();
        self.prompts.state.lock().unwrap().push(Proc(ProcContent::Prompt(ReadPrompt {
            prompt: Cd::new_true(self.buf.clone()),
            promise,
        }),
        ProcState::Uninit));
        waiter.await
    }
}

#[allow(dead_code)]
pub enum LookUpError {
    Message(Cow<'static, str>),
    NanoError(NanoError),
    Incomplete(Vec<String>),
}

/// Handles &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> LookUpObject for &[T] {
    type Item = String;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        let matches: Vec<&str> = self
            .iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input))
            .collect();
        match matches[..] {
            [a] => Ok(a.to_string()),
            [_a, _b, ..] => Err(LookUpError::Incomplete(
                matches.into_iter().map(|s| s.to_string()).collect(),
            )),
            [] => Err(LookUpError::Message(" no matches".into())),
        }
    }
}

pub trait LookUpObject: Sized {
    type Item;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<T> LookUpObject for T
where
    T: LookUp,
{
    type Item = T;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        T::look_up(input)
    }
}

pub trait LookUp: Sized {
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
            _ => Err(LookUpError::Incomplete(vec![
                "Tom".into(),
                "Dick".into(),
                "Harry".into(),
            ])),
        }
    }
}

// [[https://bevy-cheatbook.github.io/programming/local.html][Local Resources - Unofficial Bevy Cheat Book]]i

pub fn prompt_input(
    mut char_events: EventReader<ReceivedCharacter>,
    keys: Res<Input<KeyCode>>,
    mut backspace_delay: Local<Option<Timer>>,
    time: Res<Time>,
    mut query: Query<&mut PromptNode>,
) {

    let backspace: bool = if keys.just_pressed(KeyCode::Back) {
        *backspace_delay = Some(Timer::from_seconds(0.5, TimerMode::Once));
        true
    } else if let Some(ref mut timer) = *backspace_delay {
        timer.tick(time.delta()).finished() && keys.pressed(KeyCode::Back)
    } else {
        false
    };
    let node = query.single();
    let mut mutate = false;

    if let Some(Proc(ProcContent::Prompt(read_prompt), ProcState::Active)) = &node.0 {
        if read_prompt.prompt.will_update(&char_events, &keys, backspace) {
            mutate = true;

        }
    }
    if mutate {
    let mut node = query.single_mut();
    let mut proc = node.0.take();
    if let Some(Proc(ProcContent::Prompt(mut read_prompt), ProcState::Active)) = proc {
        match read_prompt.prompt.update(&mut char_events, &keys, backspace) {
            Ok(finished) =>
                if finished {
                    read_prompt.promise.resolve(Cd::take(read_prompt.prompt));
                    return;
                },
            Err(e) => {
                read_prompt.promise.reject(e);
                return;
            }
        }
        proc = Some(Proc(ProcContent::Prompt(read_prompt), ProcState::Active));
    }
    node.0 = proc;
    }
}


/// prints every char coming in; press enter to echo the full string
pub fn state_update(
    prompt_provider: ResMut<ConsoleConfig>,
    mut query: Query<&mut PromptNode>,
) {
    let mut console_state = prompt_provider.state.lock().unwrap();
    let mut node = query.single_mut();

    if ! console_state.unprocessed.is_empty() {
        match node.0.take() {
            Some(x) => {
                console_state.asleep.push(x);
            }
            None => {}
        }
        let mut unprocessed = vec![];
        std::mem::swap(&mut console_state.unprocessed, &mut unprocessed);
        console_state.asleep.extend(unprocessed.drain(0..));
        node.0 = console_state.asleep.pop();
        eprintln!("node.0 set 1 {:?}", node.0);
    } else if node.0.is_none() && ! console_state.asleep.is_empty() {
        node.0 = console_state.asleep.pop();
        eprintln!("node.0 set 2 {:?}", node.0);
    }
}

pub fn prompt_output(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut show_completion: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(&mut Text, &mut PromptNode), Changed<PromptNode>>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    // mut completion: Query<&mut CompletionList, With<ScrollingList>>,
) {
    if let Ok((mut text, mut node)) = query.get_single_mut() {
        let (completion_node, children) = completion.single();
        let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_else(|| vec![]);
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let mut text_prompt = TextPrompt {
            text: &mut text,
            completion: completion_node,
            children: &children,
            font: font,
            commands: &mut commands,
        };

        match &mut node.0 {
            // Some(Proc(ProcContent::Prompt(read_prompt), x @ ProcState::Uninit)) => {
            Some(Proc(ProcContent::Prompt(read_prompt), x)) => {
                // read_prompt.prompt.render(&mut text_prompt);
                eprintln!("setting prompt");
                text_prompt.buf_write(&mut read_prompt.prompt);
                show_prompt.set(PromptState::Visible);
                show_completion.set(
                    if read_prompt.prompt.completion.len() > 0 {
                        CompletionState::Visible
                    } else {
                        CompletionState::Invisible
                    });
                redraw.send(RequestRedraw);
                *x = ProcState::Active;
            },
            None => {
                eprintln!("setting prompt invisible");
                show_prompt.set(PromptState::Invisible);
                show_completion.set(CompletionState::Invisible);
            },
            _ => {}
        };
    } else {
        // eprintln!("quick return");
    }
}


pub fn message_update(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    keys: Res<Input<KeyCode>>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut show_completion: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(&mut Text, &mut PromptNode)>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    // mut completion: Query<&mut CompletionList, With<ScrollingList>>,
) {

    let (text, node) = query.single();
    let mutate = node.0.as_ref().map(|proc| proc.1 == ProcState::Uninit).unwrap_or(false)
        || keys.get_just_pressed().len() > 0;

    if mutate {
    let (mut text, mut node) = query.single_mut();
    let (completion_node, children) = completion.single();
    let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_else(|| vec![]);
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let mut text_prompt = TextPrompt {
        text: &mut text,
        completion: completion_node,
        children: &children,
        font: font,
        commands: &mut commands,
    };

    match &mut node.0 {
        Some(Proc(ProcContent::Message(msg), ProcState::Active)) => {
            if keys.get_just_pressed().len() > 0 {
                // Remove ourselves.
                node.0 = None;
                eprintln!("removing message at {:?}", time.elapsed_seconds());
                return;
            }
        },
        Some(Proc(ProcContent::Message(msg), x @ ProcState::Uninit)) => {
            eprintln!("setting message at {:?}", time.elapsed_seconds());
            *text_prompt.prompt_get_mut() = msg.to_string();
            text_prompt.input_get_mut().clear();
            text_prompt.message_get_mut().clear();
            show_prompt.set(PromptState::Visible);
            show_completion.set(CompletionState::Invisible);
            redraw.send(RequestRedraw);
            *x = ProcState::Active;
        },
        _ => {}
    }
    }
}

pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

#[derive(Component)]
pub struct HideTime {
    pub timer: Timer,
}

pub fn hide_delayed<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    if let Ok(id) = query.get_single() {
        commands.entity(id).insert(HideTime {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
        });
    }
}

pub fn hide_prompt_maybe(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<PromptState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(Entity, &mut Visibility, &mut HideTime)>,
) {
    for (id, mut visibility, mut hide) in query.iter_mut() {
        // eprintln!("checking hide {:?}", time.delta());
        redraw.send(RequestRedraw); // Force ticks to happen when a timer is present.
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == PromptState::Invisible {
                eprintln!("hiding after delay.");
                *visibility = Visibility::Hidden;
            }
            commands.entity(id).remove::<HideTime>();
        }
    }
}

#[allow(dead_code)]
pub fn hide<T: Component>(mut query: Query<&mut Visibility, With<T>>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
    }
}

#[cfg(test)]
mod tests {

    // #[allow(unused_must_use)]
    // #[test]
    // fn test_option_default() {
    //     let a: Option<PromptCel> = default();
    // }
}
