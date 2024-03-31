#![allow(async_fn_in_trait)]
use std::borrow::Cow;
use std::fmt::{Debug};

use bevy::ecs::{component::Tick, system::{SystemParam, SystemMeta, SystemState}, world::unsafe_world_cell::UnsafeWorldCell};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use asky::{Printable, Typeable, Valuable, Error, SetValue, bevy::{Asky, KeyEvent, AskyPrompt}, style::Style, utils::renderer::Renderer};

use std::io;
use std::future::Future;

use bevy_crossbeam_event::CrossbeamEventSender;
use crate::MinibufferStyle;
use crate::commands::StartActEvent;
use trie_rs::{iter::KeysExt, map};

use crate::ui::*;

pub type CowStr = Cow<'static, str>;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum PromptState {
    #[default]
    // Uninit,
    Invisible,
    Finished,
    Visible,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum CompletionState {
    // Uninit,
    #[default]
    Invisible,
    Visible,
}

#[derive(Debug)]
pub enum NanoError {
    Cancelled,
    Message(CowStr),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LookUpError {
    Message(Cow<'static, str>),
    NanoError(NanoError),
    Incomplete(Vec<String>),
}

pub trait LookUp {
    // Object-safe
    fn look_up(&self, input: &str) -> Result<(), LookUpError>;
    fn longest_prefix(&self, input: &str) -> Option<String>;
}

pub trait Resolve {
    // Not object-safe
    type Item: Send;
    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError>;
}

impl<V: Send + Sync + Clone> Resolve for map::Trie<u8, V> {
    type Item = V;

    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
        if let Some(value) = self.exact_match(input) {
            return Ok(value.clone());
        }
        let matches: Vec<String> = self.predictive_search(input).keys().collect();
        match matches.len() {
            0 => Err(LookUpError::Message("no matches".into())),
            // 1 =>
            //     if matches[0] == input {
            //         Ok(self.exact_match(input).cloned().unwrap())
            //     } else {
            //         Err(LookUpError::Incomplete(matches))
            //     },
            _ => Err(LookUpError::Incomplete(matches))
        }
    }
}

impl<V: Send + Sync + Clone> LookUp for map::Trie<u8, V> {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        map::Trie::<u8, V>::longest_prefix(self, input)
    }
}

impl Resolve for trie_rs::Trie<u8> {
    type Item = ();

    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
        self.0.look_up(input)
    }
}

impl LookUp for trie_rs::Trie<u8> {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        self.0.longest_prefix(input)
    }
}

/// Handles arrays of &str, String, Cow<'_, str>. Does it all.
impl<T: AsRef<str>> Resolve for &[T] {
    type Item = String;
    fn resolve(&self, input: &str) -> Result<Self::Item, LookUpError> {
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
            } else if input == first {
                Ok(first.to_string())
            } else {
                Err(LookUpError::Incomplete(vec![first.to_string()]))
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }
}

impl<T: AsRef<str>> LookUp for &[T] {
    fn look_up(&self, input: &str) -> Result<(), LookUpError> {
        self.resolve(input).map(|_| ())
    }

    fn longest_prefix(&self, _input: &str) -> Option<String> {
        todo!();
    }

}


// impl<T> LookUp for T
// where
//     T: Parse,
// {
//     type Item = T;
//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         T::parse(input)
//     }
// }

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

#[derive(Debug, Resource, Clone, Default)]
pub struct ConsoleConfig {
    pub auto_hide: bool,
    pub hide_delay: Option<u64>, // milliseconds
    pub style: TextStyle,
}

// impl Default for ConsoleConfig {
//     fn default() -> Self {
//         Self {
//             hide_delay: Some(2000), /* milliseconds */
//         }
//     }
// }

pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<ConsoleConfig>,
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<(Entity, &mut Visibility, Option<&mut HideTime>), With<T>>,
) {
    if ! config.auto_hide {
        return;
    }
    for (id, mut visibility, hide_time_maybe) in query.iter_mut() {
        match config.hide_delay {
            Some(hide_delay) => {
                match hide_time_maybe {
                    Some(mut hide_time) => {
                        hide_time
                            .timer = Timer::new(Duration::from_millis(hide_delay),
                                                TimerMode::Once);
                    }
                    None => {
                        commands.entity(id).insert(HideTime {
                            timer: Timer::new(Duration::from_millis(hide_delay),
                                            TimerMode::Once),
                        });
                    }
                }
            }
            None => {
                *visibility = Visibility::Hidden;
                redraw.send(RequestRedraw);
            }
        }
    }
}

pub fn hide_prompt_maybe(
    mut commands: Commands,
    // mut tasks: Query<(Entity, &mut TaskSink<T>)>,
    time: Res<Time>,
    state: Res<State<AskyPrompt>>,
    mut redraw: EventWriter<RequestRedraw>,
    mut next_prompt_state: ResMut<NextState<PromptState>>,
    mut next_completion_state: ResMut<NextState<CompletionState>>,
    mut query: Query<(Entity, &mut HideTime)>,
) {
    for (id, mut hide) in query.iter_mut() {
        // eprintln!("checking hide {:?}", time.delta());
        redraw.send(RequestRedraw); // Force ticks to happen when a timer is present.
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == AskyPrompt::Inactive {

                next_prompt_state.set(PromptState::Invisible);
                next_completion_state.set(CompletionState::Invisible);
                // eprintln!("hiding after delay.");
                // *visibility = Visibility::Hidden;
            }
            commands.entity(id).remove::<HideTime>();
        }
    }
}

#[allow(dead_code)]
pub fn hide<T: Component>(mut query: Query<&mut Visibility, With<T>>,
                          mut redraw: EventWriter<RequestRedraw>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
        redraw.send(RequestRedraw);
    }
}

pub struct Minibuffer {
    asky: Asky,
    dest: Entity,
    style: MinibufferStyle,
    channel: CrossbeamEventSender<DispatchEvent>,
}

unsafe impl SystemParam for Minibuffer {
    type State = (Asky, Entity, Option<MinibufferStyle>, CrossbeamEventSender<DispatchEvent>);
    type Item<'w, 's> = Minibuffer;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Asky,
            Query<Entity, With<PromptContainer>>,
            Option<Res<MinibufferStyle>>,
            Res<CrossbeamEventSender<DispatchEvent>>,
        )> = SystemState::new(world);
        let (asky, query, res, channel) = state.get_mut(world);
        // let asky_param_config = world
        //     .get_resource_mut::<AskyParamConfig>()
        //     .expect("No AskyParamConfig setup.")
        //     .clone();
        (asky, query.single(), res.map(|x| *x), channel.clone())
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let state = state.clone();
        Minibuffer {
            asky: state.0,
            dest: state.1,
            style: state.2.unwrap_or_default(),
            channel: state.3,
        }
    }
}

#[derive(Debug, Clone, Event)]
pub enum LookUpEvent {
    Hide,
    Completions(Vec<String>)
}

#[derive(Debug, Clone, Event)]
pub enum DispatchEvent {
    LookUpEvent(LookUpEvent),
    StartActEvent(StartActEvent),
}

impl From<LookUpEvent> for DispatchEvent {
    fn from(e: LookUpEvent) -> Self {
        Self::LookUpEvent(e)
    }
}
impl From<StartActEvent> for DispatchEvent {
    fn from(e: StartActEvent) -> Self {
        Self::StartActEvent(e)
    }
}



pub struct AutoComplete<T> {
    inner: T,
    look_up: Box<dyn LookUp + Send + Sync>,
    channel: CrossbeamEventSender<DispatchEvent>,
    show_completions: bool
}

impl<T> AutoComplete<T> where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    <T as Valuable>::Output: AsRef<str>,
{
    fn new<L>(inner: T, look_up: L, channel: CrossbeamEventSender<DispatchEvent>) -> Self
    where L: LookUp + Send + Sync + 'static {
        Self {
            inner,
            look_up: Box::new(look_up),
            channel,
            show_completions: false,
        }
    }
}

impl<T> Valuable for AutoComplete<T> where T: Valuable {
    type Output = T::Output;
    fn value(&self) -> Result<Self::Output, Error> {
        self.inner.value()
    }
}

impl<T> Typeable<KeyEvent> for AutoComplete<T> where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    <T as Valuable>::Output: AsRef<str>,
// L::Item: Display
{
    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        use crate::prompt::LookUpError::*;
        // let mut hide = true;
        for code in &key.codes {
            if code == &KeyCode::Tab {
                self.show_completions = true;

                if let Ok(input) = self.inner.value() {
                    // What value does the input have?
                    if let Err(e) = self.look_up.look_up(input.as_ref()) {
                        match e {
                            Message(_s) => (), // Err(s),
                            Incomplete(_v) => {
                                if let Some(new_input) = self.look_up.longest_prefix(input.as_ref()) {
                                    let _ = self.inner.set_value(new_input);
                                }
                            },
                            NanoError(_e) => (), //Err(format!("Error: {:?}", e).into()),
                        }
                    }
                }
                // hide = false;
            }
        }
        // if hide {
        //     self.channel.send(LookUpEvent::Hide);
        // }
        let result = self.inner.handle_key(key);
        if self.show_completions {
            if let Ok(input) = self.inner.value() {
                // What value does the input have?
                match self.look_up.look_up(input.as_ref()) {
                    Ok(_) => self.channel.send(LookUpEvent::Hide),
                    Err(e) => match e {
                        Message(_s) => {
                            // TODO: message should go somewhere.
                            self.channel.send(LookUpEvent::Hide);
                        }// Err(s),
                        Incomplete(v) => {
                            self.channel.send(LookUpEvent::Completions(v))
                        },
                        NanoError(_e) => (), //Err(format!("Error: {:?}", e).into()),
                    },
                }
            }
        }
        result
    }

    fn will_handle_key(&self, key: &KeyEvent) -> bool {
        for code in &key.codes {
            if code == &KeyCode::Tab {
                return true;
            }
        }
        self.inner.will_handle_key(key)
    }
}

impl<T> Printable for AutoComplete<T> where T: Printable {
    fn draw_with_style<R: Renderer>(&self, renderer: &mut R, style: &dyn Style)
        -> io::Result<()> {
        self.inner.draw_with_style(renderer, style)
    }
}

impl Minibuffer {
    pub fn prompt<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static>(
        &mut self,
        prompt: T
    ) -> impl Future<Output = Result<T::Output, Error>> + '_ {
        self.prompt_styled(prompt, self.style)
    }

    pub fn read<L>(
        &mut self,
        prompt: String,
        lookup: L
    // ) -> impl Future<Output = Result<<asky::Text<'_> as Valuable>::Output, Error>> + '_ where
    ) -> impl Future<Output = Result<String, Error>> + '_ where
        L: LookUp + Clone + Send + Sync + 'static,
    {

        use crate::prompt::LookUpError::*;
        let mut text = asky::Text::new(prompt);
        let l = lookup.clone();
        text
            .validate(move |input|
                      match l.look_up(input) {
                          Ok(_) => Ok(()),
                          Err(e) => match e {
                              Message(s) => Err(s),
                              // Incomplete(_v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
                              Incomplete(_v) => Err("Incomplete".into()),
                              NanoError(e) => Err(format!("Error: {:?}", e).into()),
                          },
                      });
        let text = AutoComplete::new(text,
                                     lookup,
                                     self.channel.clone());
        self.prompt_styled(text, self.style)
    }

    pub async fn prompt_styled<T: Typeable<KeyEvent> + Valuable + Send + Sync + 'static, S>(
        &mut self,
        prompt: T,
        style: S
    ) -> Result<T::Output, Error>
    where S: Style + Send + Sync + 'static {
        let _ = self.asky.clear(self.dest).await;
        self.asky.prompt_styled(prompt, self.dest, style).await
    }

    pub fn clear(&mut self) -> impl Future<Output = Result<(), Error>> {
        self.asky.clear(self.dest)
    }

    pub fn delay(&mut self, duration: Duration) -> impl Future<Output = Result<(), Error>> {
        self.asky.delay(duration)
    }
}

// async fn read_crit<T>(
//     &mut self,
//     prompt: impl Into<PromptBuf>,
//     look_up: &impl LookUp<Item = T>,
// ) -> Result<T, NanoError> {
//     let mut buf = prompt.into();
//     loop {
//         match self.read_raw(buf.clone()).await {
//             Ok(mut new_buf) => match look_up.look_up(&new_buf.input) {
//                 Ok(v) => {
//                     if new_buf.flags.contains(Requests::Submit) {
//                         return Ok(v);
//                     } else {
//                         buf = new_buf
//                     }
//                 }
//                 Err(LookUpError::Message(m)) => {
//                     new_buf.completion.clear();
//                     new_buf.message = m.to_string();
//                     buf = new_buf;
//                 }
//                 Err(LookUpError::Incomplete(v)) => {
//                     if new_buf.flags.contains(Requests::AutoComplete) {
//                         new_buf.completion.clear();
//                         new_buf.completion.extend_from_slice(&v[..]);

//                         if !new_buf.completion.is_empty() {
//                             let prefix = longest_common_prefix(&new_buf.completion);
//                             if prefix.len() > new_buf.input.len() {
//                                 new_buf.input = prefix;
//                             }
//                             new_buf.message.clear();
//                         }
//                     }
//                     buf = new_buf;
//                 }
//                 Err(LookUpError::NanoError(e)) => return Err(e),
//             },
//             Err(e) => return Err(e),
//         }
//         buf.flags = Requests::empty();
//     }
// }

fn completion_set(completion: Entity,
                  children: Option<&Children>,
                  labels: Vec<String>,
                  style: TextStyle,
                  commands: &mut Commands) {
    let new_children = labels
        .into_iter()
        .map(|label| {
            commands
                .spawn(completion_item(label, style.clone()))
                .id()
        })
        .collect::<Vec<Entity>>();
    commands
        .entity(completion)
        .replace_children(&new_children);
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(*child).despawn();
        }
    }
}


pub(crate) fn handle_dispatch_event(
    mut dispatch_events: EventReader<DispatchEvent>,
    mut look_up_events: EventWriter<LookUpEvent>,
    mut request_act_events: EventWriter<StartActEvent>,
) {
    use crate::prompt::DispatchEvent::*;
    for e in dispatch_events.read() {
        match e {
            LookUpEvent(l) => { look_up_events.send(l.clone()); }
            StartActEvent(s) => { request_act_events.send(s.clone()); }
        }
    }
}
pub(crate) fn handle_look_up_event(mut look_up_events: EventReader<LookUpEvent>,
                                   completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
                                   mut next_completion_state: ResMut<NextState<CompletionState>>,
                                   mut redraw: EventWriter<RequestRedraw>,
                                   mut commands: Commands,
                                   mut last_hash: Local<Option<u64>>,
                                   config: Res<ConsoleConfig>,
) {
    let text_style = &config.style;
    for e in look_up_events.read() {
        info!("look up event: {e:?}");
        match e {
            LookUpEvent::Completions(v) => {
                let rnd_state = bevy::utils::RandomState::with_seed(0);
                let hash = rnd_state.hash_one(v);
                eprintln!("hash {hash}");
                if last_hash.unwrap_or(0) != hash {
                    let (completion_node, children) = completion.single();
                    completion_set(completion_node, children,
                                   v.clone(),
                                   text_style.clone(),
                                   &mut commands);
                    next_completion_state.set(CompletionState::Visible);
                    redraw.send(RequestRedraw);
                }
                *last_hash = Some(hash);
            },
            LookUpEvent::Hide => {
                eprintln!("hide");
                *last_hash = None;
                next_completion_state.set(CompletionState::Invisible);
                redraw.send(RequestRedraw);
            }
        }
    }
}

pub fn listen_prompt_active(mut transitions: EventReader<StateTransitionEvent<AskyPrompt>>,
                            mut next_prompt_state: ResMut<NextState<PromptState>>,
                            mut redraw: EventWriter<RequestRedraw>,
) {
    for transition in transitions.read() {
        eprintln!("transition.after {:?}", &transition.after);
        match transition.after {
            AskyPrompt::Active => next_prompt_state.set(PromptState::Visible),
            AskyPrompt::Inactive => next_prompt_state.set(PromptState::Finished),
        }
        redraw.send(RequestRedraw);
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

    #[test]
    fn test_tom_dick_parse() {
        let a = TomDickHarry::parse("Tom").unwrap();
        assert_eq!(a.0, "Tom");
    }

    #[test]
    fn test_lookup() {
        use trie_rs::Trie;
        use super::LookUp;
        let trie: Trie<u8> = ["ask_name", "ask_name2", "asky_age"].into_iter().collect();
        assert_eq!(trie.longest_prefix::<String, _>("a").unwrap(), "ask");
        let lookup: &dyn LookUp = &trie;
        assert_eq!(lookup.longest_prefix("a").unwrap(), "ask");
        assert_eq!(lookup.longest_prefix("b"), None);
        // let lookup: &dyn LookUp = &trie;
    }
}
