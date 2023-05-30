use bevy::ecs::component::Tick;
use bevy::ecs::prelude::Commands;
use bevy::ecs::system::{SystemMeta, SystemParam, SystemState};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::utils::Duration;
use promise_out::{pair::Producer, Promise};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use crate::ui::*;

#[allow(dead_code)]
#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(Cow<'static, str>),
}

struct ReadPrompt {
    prompt: PromptBuf,
    active: bool,
    prior: Option<PromptBuf>,
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

pub trait NanoPrompt {
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

    async fn read_crit<T>(
        &mut self,
        prompt: impl Into<PromptBuf>,
        look_up: &impl LookUpObject<Item = T>,
    ) -> Result<T, NanoError> {
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
pub enum PromptState {
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
        if matches.len() == 1 {
            Ok(matches[0].to_string())
        } else if matches.len() > 1 {
            Err(LookUpError::Incomplete(
                matches.into_iter().map(|s| s.to_string()).collect(),
            ))
        } else {
            Err(LookUpError::Message(" no matches".into()))
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

/// prints every char coming in; press enter to echo the full string
pub fn prompt_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    prompt_provider: ResMut<PromptProvider>,
    mut char_evr: EventReader<ReceivedCharacter>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Text, With<PromptNode>>,
    completion: Query<(Entity, &Children), With<ScrollingList>>,
    // mut text_prompt: TextPrompt,
) {
    // eprintln!("chars {:?}", char_evr.iter().map(|ev| ev.char).collect::<Vec<_>>());
    let mut prompts = prompt_provider.prompt_stack.lock().unwrap();
    let (completion_node, children) = completion.single();
    let children: Vec<Entity> = children.to_vec();
    for mut text in query.iter_mut() {
        let len = prompts.len();
        if prompts.len() > 0 {
            let font = asset_server.load("fonts/FiraSans-Bold.ttf");
            let mut text_prompt = TextPrompt {
                text: &mut text,
                completion: completion_node,
                children: &children,
                font: font.clone(),
                commands: &mut commands,
            };

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
                let buf = read_prompt
                    .prior
                    .take()
                    .unwrap_or_else(|| read_prompt.prompt.clone());

                eprintln!("setup new prompt {:?}", buf);
                text_prompt.buf_write(&buf);
                read_prompt.active = true;
                show_prompt.set(PromptState::Visible);
            }
            // if keys.just_pressed(KeyCode::Back) {
            if keys.pressed(KeyCode::Back) {
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

pub fn show_prompt(mut query: Query<&mut Visibility, With<PromptContainer>>) {
    let mut visibility = query.single_mut();
    *visibility = Visibility::Visible;
}

#[derive(Component)]
pub struct HideTime {
    pub timer: Timer,
}

pub fn hide_prompt_delayed(mut commands: Commands, query: Query<Entity, With<PromptContainer>>) {
    let id = query.single();
    commands.entity(id).insert(HideTime {
        timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
    });
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
pub fn hide_prompt(mut query: Query<&mut Visibility, With<PromptContainer>>) {
    let mut visibility = query.single_mut();
    *visibility = Visibility::Hidden;
}

#[cfg(test)]
mod tests {

    #[allow(unused_must_use)]
    #[test]
    fn test_option_default() {
        let a: Option<PromptCel> = default();
    }
}
