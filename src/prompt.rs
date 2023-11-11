use bitflags::bitflags;
use std::borrow::Cow;
use std::fmt::Debug;

use bevy::ecs::prelude::Commands;
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use promise_out::{pair::Producer, Promise};

use crate::proc::*;
use crate::ui::*;

pub type CowStr = Cow<'static, str>;

#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(CowStr),
}

#[derive(Debug)]
pub(crate) struct ReadPrompt {
    pub(crate) prompt: PromptBuf,
    pub(crate) promise: Producer<PromptBuf, NanoError>,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Ord)]
    pub struct Requests: u8 {
        const Submit       = 0b00000001;
        const AutoComplete = 0b00000010;
    }
}

// TODO: Switch to cows or options.
#[derive(Clone, Default, Debug)]
pub struct PromptBuf {
    pub prompt: String,
    pub input: String,
    pub message: String,
    pub completion: Vec<String>,
    pub flags: Requests,
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
            completion: Vec::new(),
            flags: Requests::empty(),
        }
    }
}

fn longest_common_prefix(strings: &Vec<String>) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let first_string = &strings[0];

    for (i, char) in first_string.chars().enumerate() {
        for string in strings.iter().skip(1) {
            if i >= string.len() || char != string.chars().nth(i).unwrap() {
                return first_string[..i].to_string();
            }
        }
    }

    first_string.to_string()
}

enum Update {
    ReturnRaw,
    Continue,
}

impl PromptBuf {
    fn will_update(
        &self,
        char_events: &EventReader<ReceivedCharacter>,
        keys: &Res<Input<KeyCode>>,
        backspace: bool,
    ) -> bool {
        keys.just_pressed(KeyCode::Escape) || backspace || !char_events.is_empty()
    }

    fn update(
        &mut self,
        char_events: &mut EventReader<ReceivedCharacter>,
        keys: &Res<Input<KeyCode>>,
        backspace: bool,
    ) -> Result<Update, NanoError> {
        if keys.just_pressed(KeyCode::Escape) {
            self.message = " Quit".into();
            return Err(NanoError::Cancelled);
        }
        if keys.just_pressed(KeyCode::Return) {
            self.flags |= Requests::Submit;
            return Ok(Update::ReturnRaw);
        }
        if keys.just_pressed(KeyCode::Tab) {
            self.flags |= Requests::AutoComplete;
            return Ok(Update::ReturnRaw);
        }
        if backspace {
            let _ = self.input.pop();
            self.message.clear();
            return Ok(Update::Continue);
        }
        if !char_events.is_empty() {
            self.input.extend(
                char_events
                    .read()
                    .map(|ev| ev.char)
                    .filter(|c| !c.is_ascii_control()),
            );
            self.message.clear();
        }
        Ok(Update::Continue)
    }
}

pub trait NanoPrompt {
    async fn read_raw(&mut self, prompt: PromptBuf) -> Result<PromptBuf, NanoError>;

    async fn read<T: Parse>(&mut self, prompt: impl Into<PromptBuf>) -> Result<T, NanoError> {
        let mut buf = prompt.into();
        loop {
            match self.read_raw(buf.clone()).await {
                Ok(mut new_buf) => match T::parse(&new_buf.input) {
                    Ok(v) => {
                        if new_buf.flags.contains(Requests::Submit) {
                            return Ok(v);
                        } else {
                            buf = new_buf
                        }
                    }
                    Err(LookUpError::Message(m)) => {
                        new_buf.message = m.to_string();
                        buf = new_buf;
                    }
                    Err(LookUpError::Incomplete(v)) => {
                        if new_buf.flags.contains(Requests::AutoComplete) {
                            new_buf.completion.clone_from_slice(&v[..]);
                        }
                        buf = new_buf;
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
            buf.flags = Requests::empty();
        }
    }

    async fn read_crit<T>(
        &mut self,
        prompt: impl Into<PromptBuf>,
        look_up: &impl LookUp<Item = T>,
    ) -> Result<T, NanoError> {
        let mut buf = prompt.into();
        loop {
            match self.read_raw(buf.clone()).await {
                Ok(mut new_buf) => match look_up.look_up(&new_buf.input) {
                    Ok(v) => {
                        if new_buf.flags.contains(Requests::Submit) {
                            return Ok(v);
                        } else {
                            buf = new_buf
                        }
                    }
                    Err(LookUpError::Message(m)) => {
                        new_buf.completion.clear();
                        new_buf.message = m.to_string();
                        buf = new_buf;
                    }
                    Err(LookUpError::Incomplete(v)) => {
                        if new_buf.flags.contains(Requests::AutoComplete) {
                            new_buf.completion.clear();
                            new_buf.completion.extend_from_slice(&v[..]);

                            if !new_buf.completion.is_empty() {
                                let prefix = longest_common_prefix(&new_buf.completion);
                                if prefix.len() > new_buf.input.len() {
                                    new_buf.input = prefix;
                                }
                                new_buf.message.clear();
                            }
                        }
                        buf = new_buf;
                    }
                    Err(LookUpError::NanoError(e)) => return Err(e),
                },
                Err(e) => return Err(e),
            }
            buf.flags = Requests::empty();
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
    async fn read_raw(&mut self, buf: PromptBuf) -> Result<PromptBuf, NanoError> {
        let (promise, waiter) = Producer::<PromptBuf, NanoError>::new();
        self.config.state.lock().unwrap().push(Proc(
            ProcContent::Prompt(ReadPrompt {
                prompt: buf,
                promise,
            }),
            ProcState::Uninit,
        ));
        waiter.await
    }
}

#[allow(dead_code)]
pub enum LookUpError {
    Message(Cow<'static, str>),
    NanoError(NanoError),
    Incomplete(Vec<String>),
}

/// Handles arrays of &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> LookUp for &[T] {
    type Item = String;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        // Collecting and matching is nice expressively. But manually iterating
        // avoids that allocation.

        // let matches: Vec<&str> = self
        //     .iter()
        //     .map(|word| word.as_ref())
        //     .filter(|word| word.starts_with(input))
        //     .collect();
        // match matches[..] {
        //     [a] => Ok(a.to_string()),
        //     [_a, _b, ..] => Err(LookUpError::Incomplete(
        //         matches.into_iter().map(|s| s.to_string()).collect(),
        //     )),
        //     [] => Err(LookUpError::Message(" no matches".into())),
        // }

        let mut matches = self
            .iter()
            .map(|word| word.as_ref())
            .filter(|word| word.starts_with(input));

        if let Some(first) = matches.next() {
            if let Some(second) = matches.next() {
                let mut result = vec![first.to_string(), second.to_string()];
                for item in matches {
                    result.push(item.to_string());
                }
                Err(LookUpError::Incomplete(result))
            } else {
                if input == first {
                    Ok(first.to_string())
                } else {
                    Err(LookUpError::Incomplete(vec![first.to_string()]))
                }
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }
}

pub trait LookUp: Sized {
    type Item;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<T> LookUp for T
where
    T: Parse,
{
    type Item = T;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        T::parse(input)
    }
}

pub trait Parse: Debug + Sized {
    fn parse(input: &str) -> Result<Self, LookUpError>;
}

impl Parse for () {
    fn parse(_: &str) -> Result<Self, LookUpError> {
        Ok(())
    }
}

impl Parse for String {
    fn parse(input: &str) -> Result<Self, LookUpError> {
        Ok(input.to_owned())
    }
}

impl Parse for i32 {
    fn parse(input: &str) -> Result<Self, LookUpError> {
        match input.parse::<i32>() {
            Ok(int) => Ok(int),
            Err(e) => Err(LookUpError::Message(format!(" expected int: {}", e).into())),
        }
    }
}

// [[https://bevy-cheatbook.github.io/programming/local.html][Local Resources - Unofficial Bevy Cheat Book]]
pub fn prompt_input(
    mut char_events: EventReader<ReceivedCharacter>,
    keys: Res<Input<KeyCode>>,
    mut backspace_delay: Local<Option<Timer>>,
    config: Res<ConsoleConfig>,
    time: Res<Time>,
    mut query: Query<&mut PromptNode>,
) {
    let backspace: bool = if keys.just_pressed(KeyCode::Back) {
        *backspace_delay = Some(Timer::new(
            Duration::from_millis(300),
            TimerMode::Once,
        ));
        true
    } else if let Some(ref mut timer) = *backspace_delay {
        timer.tick(time.delta()).finished() && keys.pressed(KeyCode::Back)
    } else {
        false
    };
    let node = query.single();
    let mut mutate = false;

    // We want to be careful about when we trigger mutation.
    if let Some(Proc(ProcContent::Prompt(read_prompt), ProcState::Active)) = &node.0 {
        mutate = read_prompt
            .prompt
            .will_update(&char_events, &keys, backspace);
    }
    if mutate {
        let mut node = query.single_mut();
        let mut proc = node.0.take();
        if let Some(Proc(ProcContent::Prompt(mut read_prompt), ProcState::Active)) = proc {
            match read_prompt
                .prompt
                .update(&mut char_events, &keys, backspace)
            {
                Ok(update) => {
                    match update {
                        Update::ReturnRaw => {
                            // This returns to the raw_read
                            // dbg!(&read_prompt.prompt.input);
                            read_prompt.promise.resolve(read_prompt.prompt);
                            // eprintln!("leaving 1");
                            return;
                        }
                        Update::Continue => {}
                    }
                }
                Err(e) => match e {
                    NanoError::Message(msg) => read_prompt.prompt.message = msg.to_string(),
                    NanoError::Cancelled => {
                        // XXX: This does not work. Would like to show "Quit" or some message when cancelled.
                        node.0 = Some(Proc(ProcContent::Message(format!("{:?}", e).into()), ProcState::Active));
                        read_prompt.promise.reject(e);
                        eprintln!("leaving 2");
                        return;
                    }
                }
            }
            proc = Some(Proc(ProcContent::Prompt(read_prompt), ProcState::Active));
        }
        node.0 = proc;
    }
}

pub fn state_update(prompt_provider: ResMut<ConsoleConfig>, mut query: Query<&mut PromptNode>) {
    let mut console_state = prompt_provider.state.lock().unwrap();
    let mut node = query.single_mut();

    if !console_state.unprocessed.is_empty() {
        if let Some(x) = node.0.take() {
            console_state.asleep.push(x);
        }
        let mut unprocessed = vec![];
        std::mem::swap(&mut console_state.unprocessed, &mut unprocessed);
        console_state.asleep.extend(unprocessed.drain(0..));
        node.0 = console_state.asleep.pop();
        // eprintln!("node.0 set 1 {:?}", node.0);
    } else if node.0.is_none() && !console_state.asleep.is_empty() {
        node.0 = console_state.asleep.pop();
        // eprintln!("node.0 set 2 {:?}", node.0);
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
) {
    if let Ok((mut text, mut node)) = query.get_single_mut() {
        let (completion_node, children) = completion.single();
        let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_else(Vec::new);
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let mut text_prompt = TextPrompt {
            text: &mut text,
            completion: completion_node,
            children: &children,
            font
        };

        match &mut node.0 {
            // Some(Proc(ProcContent::Prompt(read_prompt), x @ ProcState::Uninit)) => {
            Some(Proc(ProcContent::Prompt(read_prompt), x)) => {
                // eprintln!("setting prompt");
                text_prompt.buf_write(&read_prompt.prompt, &mut commands);
                show_prompt.set(PromptState::Visible);
                show_completion.set(if !read_prompt.prompt.completion.is_empty() {
                    CompletionState::Visible
                } else {
                    CompletionState::Invisible
                });
                redraw.send(RequestRedraw);
                *x = ProcState::Active;
            }
            None => {
                // eprintln!("setting prompt invisible");
                show_prompt.set(PromptState::Invisible);
                show_completion.set(CompletionState::Invisible);
                redraw.send(RequestRedraw);
            }
            _ => {}
        };
    } else {
        // eprintln!("quick return");
    }
}

pub fn message_update(
    // mut commands: Commands,
    // time: Res<Time>,
    asset_server: Res<AssetServer>,
    keys: Res<Input<KeyCode>>,
    mut show_prompt: ResMut<NextState<PromptState>>,
    mut show_completion: ResMut<NextState<CompletionState>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(&mut Text, &mut PromptNode)>,
    completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
) {
    let (_text, node) = query.single();
    let mutate = node
        .0
        .as_ref()
        .map(|proc| proc.1 == ProcState::Uninit)
        .unwrap_or(false)
        || keys.get_just_pressed().len() > 0;

    if mutate {
        let (mut text, mut node) = query.single_mut();
        let (completion_node, children) = completion.single();
        let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_else(Vec::new);
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let mut text_prompt = TextPrompt {
            text: &mut text,
            completion: completion_node,
            children: &children,
            font
        };

        match &mut node.0 {
            Some(Proc(ProcContent::Message(_msg), ProcState::Active)) => {
                if keys.get_just_pressed().len() > 0 {
                    // Remove ourselves.
                    node.0 = None;
                    eprintln!("removing message");
                }
            }
            Some(Proc(ProcContent::Message(msg), x @ ProcState::Uninit)) => {
                eprintln!("setting message");
                *text_prompt.prompt_get_mut() = msg.to_string();
                text_prompt.input_get_mut().clear();
                text_prompt.message_get_mut().clear();
                show_prompt.set(PromptState::Visible);
                show_completion.set(CompletionState::Invisible);
                redraw.send(RequestRedraw);
                *x = ProcState::Active;
            }
            _ => {}
        }
    }
}

pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

#[derive(Component)]
pub struct HideTime {
    pub timer: Timer,
}

pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<ConsoleConfig>,
    mut query: Query<(Entity, Option<&mut HideTime>), With<T>>,
) {
    if let Ok((id, hide_time_maybe)) = query.get_single_mut() {
        match hide_time_maybe {
            Some(mut hide_time) => { hide_time.timer = Timer::new(Duration::from_millis(config.hide_delay),
                                                              TimerMode::Once); }
            None => {
                commands.entity(id).insert(HideTime {
                    timer: Timer::new(Duration::from_millis(config.hide_delay), TimerMode::Once),
                });
            }
        }
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
    use crate::prompt::LookUpError;
    use crate::prompt::Parse;

    #[derive(Debug)]
    struct TomDickHarry(String);

    impl Parse for TomDickHarry {
        fn parse(input: &str) -> Result<Self, LookUpError> {
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

    // #[allow(unused_must_use)]
    // #[test]
    // fn test_option_default() {
    //     let a: Option<PromptCel> = default();
    // }
}
