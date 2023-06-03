use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use bevy::ecs::component::Tick;
use bevy::ecs::prelude::Commands;
use bevy::ecs::system::{SystemMeta, SystemParam};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::utils::Duration;
use changed::Cd;

use promise_out::{pair::Producer, Promise};

use crate::ui::*;

#[allow(dead_code)]
#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(Cow<'static, str>),
}

struct ReadPrompt {
    prompt: Cd<PromptBuf>,
    // active: bool,
    // prior: Option<PromptBuf>,
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
    pub fn new_prompt(&mut self) -> Prompt {
        Prompt::new(self.prompt_stack.clone())
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

pub struct Message {
    pub content: Cow<'static, str>,
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
            buf: default(),
            prompts,
        }
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
    fn buf_write(&mut self, buf: &mut PromptBuf) {
        self.buf.clone_from(&buf);
    }
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError> {
        let (promise, waiter) = Producer::<PromptBuf, NanoError>::new();
        self.prompts.lock().unwrap().push(ReadPrompt {
            prompt: Cd::new_true(self.buf.clone()),
            promise,
            // active: false,
            // prior: None,
        });
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

trait ConsoleUpdate {
    fn should_update(&self,
              char_events: &EventReader<ReceivedCharacter>,) -> bool;
    /// Return Ok(true) if finished.
    fn update(&mut self,
              char_events: &mut EventReader<ReceivedCharacter>,
              keys: &Res<Input<KeyCode>>,
              backspace: bool) -> Result<bool, NanoError>;

    fn render(&mut self,
              live: &mut TextPrompt);

}

impl ConsoleUpdate for PromptBuf {
    fn should_update(&self,
              char_events: &EventReader<ReceivedCharacter>,
    ) -> bool {
        // ! char_events.is_empty()
        true
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

    fn render(&mut self,
              live: &mut TextPrompt) {
        live.prompt_get_mut().clone_from(&self.prompt);
        live.input_get_mut().clone_from(&self.input);
        live.message_get_mut().clone_from(&self.message);

        let new_children = (*self.completion)
            .iter()
            .map(|label| {
                label.clone()
            })
            .collect::<Vec<String>>();
        live.completion_set(new_children);
    }
}

pub fn prompt_input(
    prompt_provider: ResMut<PromptProvider>,
    mut char_events: EventReader<ReceivedCharacter>,
    keys: Res<Input<KeyCode>>,
    mut backspace_delay: Local<Option<Timer>>,
    time: Res<Time>,
) {

    let backspace: bool = if keys.just_pressed(KeyCode::Back) {
        *backspace_delay = Some(Timer::from_seconds(1., TimerMode::Once));
        true
    } else if let Some(ref mut timer) = *backspace_delay {
        timer.tick(time.delta()).finished() && keys.pressed(KeyCode::Back)
    } else {
        false
    };
    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();

    if ! prompts.last().map(|read_prompt| read_prompt.prompt.should_update(&char_events)).unwrap_or(false) {
        // No update.
    } else if let Some(mut read_prompt) = prompts.pop() {
        match read_prompt.prompt.update(&mut char_events, &keys, backspace) {
            Ok(finished) =>
                if finished {
                    read_prompt.promise.resolve(Cd::take(read_prompt.prompt));
                } else {
                    prompts.push(read_prompt);
                },
            Err(e) => {
                read_prompt.promise.reject(e);
            }
        }
    }
}

/// prints every char coming in; press enter to echo the full string
pub fn prompt_output(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    prompt_provider: ResMut<PromptProvider>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut show_completion: ResMut<NextState<CompletionState>>,
    mut query: Query<&mut Text, With<PromptNode>>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
    // mut completion: Query<&mut CompletionList, With<ScrollingList>>,
) {
    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();

    let (completion_node, children) = completion.single();
    let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_else(|| vec![]);
    let mut text = query.single_mut();
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let mut text_prompt = TextPrompt {
        text: &mut text,
        completion: completion_node,
        children: &children,
        font: font,
        commands: &mut commands,
    };
    if let Some(read_prompt) = prompts.last_mut() {
        read_prompt.prompt.render(&mut text_prompt);
        // text_prompt.buf_write(&mut read_prompt.prompt);
        show_prompt.set(PromptState::Visible);
        show_completion.set(if read_prompt.prompt.completion.len() > 0 {
            CompletionState::Visible
        } else {
            CompletionState::Invisible
        });
    } else {
        show_prompt.set(PromptState::Invisible);
        show_completion.set(CompletionState::Invisible);
    }
}

pub fn show<T: Component>(mut query: Query<&mut Visibility, With<T>>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
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
