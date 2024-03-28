#![allow(async_fn_in_trait)]
use std::borrow::Cow;
use std::fmt::{Display, Debug};

use bevy::ecs::{component::Tick, system::{SystemParam, SystemMeta, SystemState}, world::unsafe_world_cell::UnsafeWorldCell};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use asky::{Printable, Typeable, Valuable, Error, SetValue, bevy::{Asky, KeyEvent, AskyPrompt}, style::Style, utils::renderer::Renderer};

use std::io;
use std::future::Future;

use bevy_crossbeam_event::CrossbeamEventSender;
use crate::MinibufferStyle;
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
    type Item: Send;
    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError>;
    fn longest_prefix(&self, input: &str) -> Option<String>;
}

// impl<'a, V: Send + Sync> LookUp for &'a map::Trie<u8, V> {
//     type Item = &'a V;

//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         let matches: Vec<String> = self.predictive_search(input).keys().collect();
//         match matches.len() {
//             0 => Err(LookUpError::Message("no matches".into())),
//             1 =>
//                 if matches[0] == input {
//                     Ok(self.exact_match(input).unwrap())
//                 } else {
//                     Err(LookUpError::Incomplete(matches))
//                 },
//             n => Err(LookUpError::Incomplete(matches))
//         }
//     }

//     fn longest_prefix(&self, input: &str) -> Option<String> {
//         map::Trie::<u8, V>::longest_prefix(&self, input)
//     }
// }
impl<V: Send + Sync + Clone> LookUp for map::Trie<u8, V> {
    type Item = V;

    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        let input = input.as_ref();
        let matches: Vec<String> = self.predictive_search(input).keys().collect();
        match matches.len() {
            0 => Err(LookUpError::Message("no matches".into())),
            1 =>
                if matches[0] == input {
                    Ok(self.exact_match(input).cloned().unwrap())
                } else {
                    Err(LookUpError::Incomplete(matches))
                },
            n => Err(LookUpError::Incomplete(matches))
        }
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        map::Trie::<u8, V>::longest_prefix(&self, input)
    }
}

// impl<'a> LookUp for &'a trie_rs::Trie<u8> {
//     type Item = ();

//     fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
//         self.0.look_up(input)
//     }

//     fn longest_prefix(&self, input: &str) -> Option<String> {
//         self.0.longest_prefix(input)
//     }
// }
impl LookUp for trie_rs::Trie<u8> {
    type Item = ();

    fn look_up(&self, input: &str) -> Result<Self::Item, LookUpError> {
        self.0.look_up(input)
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
        self.0.longest_prefix(input)
    }
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
            } else if input == first {
                Ok(first.to_string())
            } else {
                Err(LookUpError::Incomplete(vec![first.to_string()]))
            }
        } else {
            Err(LookUpError::Message(" no matches".into()))
        }
    }

    fn longest_prefix(&self, input: &str) -> Option<String> {
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

#[derive(Debug, Resource, Clone)]
pub struct ConsoleConfig {
    // pub(crate) state: Arc<Mutex<ConsoleState>>,
    pub hide_delay: Option<u64>, // milliseconds
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            // state: Arc::new(Mutex::new(ConsoleState::new())),
            hide_delay: Some(2000), /* milliseconds */
        }
    }
}

pub fn hide_delayed<T: Component>(
    mut commands: Commands,
    config: Res<ConsoleConfig>,
    mut query: Query<(Entity, &mut Visibility, Option<&mut HideTime>), With<T>>,
) {
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
pub fn hide<T: Component>(mut query: Query<&mut Visibility, With<T>>) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
    }
}

pub struct Minibuffer {
    asky: Asky,
    dest: Entity,
    style: MinibufferStyle,
    channel: CrossbeamEventSender<LookUpEvent>,
}

unsafe impl SystemParam for Minibuffer {
    type State = (Asky, Entity, Option<MinibufferStyle>, CrossbeamEventSender<LookUpEvent>);
    type Item<'w, 's> = Minibuffer;

    #[allow(clippy::type_complexity)]
    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        let mut state: SystemState<(
            Asky,
            Query<Entity, With<PromptContainer>>,
            Option<Res<MinibufferStyle>>,
            Res<CrossbeamEventSender<LookUpEvent>>,
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

pub struct AutoComplete<T, L>
where L: LookUp, L::Item: Send {
    inner: T,
    look_up: L,
    channel: CrossbeamEventSender<LookUpEvent>,
    show_completions: bool
}
// pub struct AutoComplete<T> {
//     inner: T,
//     look_up: Box<dyn LookUp<Item = String>>,
//     channel: CrossbeamEventSender<LookUpEvent>
// }

impl<T,L> AutoComplete<T,L> where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    L: LookUp,
    <T as Valuable>::Output: AsRef<str>,
{
    fn new(inner: T, look_up: L, channel: CrossbeamEventSender<LookUpEvent>) -> Self {
        Self {
            inner,
            look_up,
            channel,
            show_completions: false,
        }
    }
}

impl<T,L> Valuable for AutoComplete<T,L> where T: Valuable, T::Output: AsRef<str>, L: LookUp, L::Item: Send {
    type Output = L::Item;
    fn value(&self) -> Result<Self::Output, Error> {
        Ok(self.look_up.look_up(self.inner.value().unwrap().as_ref()).unwrap())
    }
}

impl<T,L> Typeable<KeyEvent> for AutoComplete<T,L> where
    T: Typeable<KeyEvent> + Valuable + SetValue<Output = String>,
    L: LookUp,
    <T as Valuable>::Output: AsRef<str>,
// L::Item: Display
{
    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        use crate::prompt::LookUpError::*;
        // let mut hide = true;
        for code in &key.codes {
            match code {
                KeyCode::Tab => {
                    self.show_completions = true;

                    match self.inner.value() {
                        // What value does the input have?
                        Ok(input) => match self.look_up.look_up(input.as_ref()) {
                            Err(e) => match e {
                                Message(s) => (), // Err(s),
                                Incomplete(v) => {
                                    if let Some(new_input) = self.look_up.longest_prefix(input.as_ref()) {
                                        let _ = self.inner.set_value(new_input);
                                    }
                                },
                                NanoError(e) => (), //Err(format!("Error: {:?}", e).into()),
                            },
                            _ => (),
                        }
                        Err(_) => ()
                    }
                    // hide = false;
                }
                _ => ()
            }
        }
        // if hide {
        //     self.channel.send(LookUpEvent::Hide);
        // }
        let result = self.inner.handle_key(key);
        if self.show_completions {
            match self.inner.value() {
                // What value does the input have?
                Ok(input) => match self.look_up.look_up(input.as_ref()) {
                    Ok(the_match) => self.channel.send(LookUpEvent::Hide),
                    Err(e) => match e {
                        Message(s) => {
                            // TODO: message should go somewhere.
                            self.channel.send(LookUpEvent::Hide);
                        }// Err(s),
                        Incomplete(v) => {
                            self.channel.send(LookUpEvent::Completions(v))
                        },
                        NanoError(e) => (), //Err(format!("Error: {:?}", e).into()),
                    },
                }
                Err(_) => ()
            }
        }
        result
    }

    fn will_handle_key(&self, key: &KeyEvent) -> bool {
        for code in &key.codes {
            match code {
                KeyCode::Tab => return true,
                _ => ()
            }
        }
        self.inner.will_handle_key(key)
    }
}

impl<T,L> Printable for AutoComplete<T,L> where T: Printable, L: LookUp {
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

    pub fn read<L: LookUp + Send + Sync + 'static>(
        &mut self,
        prompt: String,
        lookup: L
    // ) -> impl Future<Output = Result<<asky::Text<'_> as Valuable>::Output, Error>> + '_ where
    ) -> impl Future<Output = Result<L::Item, Error>> + '_ where
        L::Item: Display
    {

        use crate::prompt::LookUpError::*;
        let mut text = asky::Text::new(prompt);
        text
            .validate(move |input|
                      match lookup.look_up(input) {
                          Ok(_) => Ok(()),
                          Err(e) => match e {
                              Message(s) => Err(s),
                              Incomplete(v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
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
                  font: Handle<Font>,
                  commands: &mut Commands) {
    let new_children = labels
        .into_iter()
        .map(|label| {
            commands
                .spawn(completion_item(label, Color::WHITE, font.clone()))
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


pub(crate) fn handle_look_up_event(mut look_up_events: EventReader<LookUpEvent>,
                                   completion: Query<(Entity, Option<&Children>), With<ScrollingList>>,
                                   mut next_completion_state: ResMut<NextState<CompletionState>>,
                                   asset_server: Res<AssetServer>,
                                   mut redraw: EventWriter<RequestRedraw>,
                                   mut commands: Commands,
) {
    for e in look_up_events.read() {
        info!("look up event: {e:?}");
        match e {
            LookUpEvent::Completions(v) => {
                let font = asset_server.load("fonts/FiraSans-Bold.ttf");
                let (completion_node, children) = completion.single();
                completion_set(completion_node, children,
                               v.clone(),
                               font,
                               &mut commands);
                next_completion_state.set(CompletionState::Visible);
                redraw.send(RequestRedraw);
            },
            LookUpEvent::Hide => {
                next_completion_state.set(CompletionState::Invisible);
                redraw.send(RequestRedraw);
            }
        }
    }
}

pub fn listen_prompt_active(mut transitions: EventReader<StateTransitionEvent<AskyPrompt>>,
                            mut next_prompt_state: ResMut<NextState<PromptState>>,
) {
    for transition in transitions.read() {
        match transition.after {
            AskyPrompt::Active => next_prompt_state.set(PromptState::Visible),
            AskyPrompt::Inactive => next_prompt_state.set(PromptState::Finished),
        }
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
        // let lookup: &dyn LookUp<Item = &()> = &&trie;
        // assert_eq!(lookup.longest_prefix("a").unwrap(), "ask");
        // assert_eq!(lookup.longest_prefix("b"), None);
    }
}
