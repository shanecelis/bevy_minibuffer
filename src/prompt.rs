#![allow(async_fn_in_trait)]
use std::borrow::Cow;
use std::fmt::{Display, Debug};

use bevy::ecs::{component::Tick, system::{SystemParam, SystemMeta, SystemState}, world::unsafe_world_cell::UnsafeWorldCell};
use bevy::prelude::*;
use bevy::utils::Duration;
use bevy::window::RequestRedraw;

use asky::{Printable, Typeable, Valuable, Error, bevy::{Asky, KeyEvent, AskyPrompt}, style::Style, utils::renderer::Renderer};

use std::io;
use std::future::Future;

use bevy_crossbeam_event::CrossbeamEventSender;
use crate::MinibufferStyle;

use crate::ui::*;

pub type CowStr = Cow<'static, str>;

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
    pub hide_delay: Option<u64>,
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
    mut query: Query<(Entity, &mut Visibility, &mut HideTime)>,
) {
    for (id, mut visibility, mut hide) in query.iter_mut() {
        // eprintln!("checking hide {:?}", time.delta());
        redraw.send(RequestRedraw); // Force ticks to happen when a timer is present.
        hide.timer.tick(time.delta());
        if hide.timer.finished() {
            if *state == AskyPrompt::Inactive {
                // eprintln!("hiding after delay.");
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

// #[derive(Deref, DerefMut)]
pub struct AutoComplete<T, L>{
    inner: T,
    look_up: L,
    channel: CrossbeamEventSender<LookUpEvent>
}

impl<T,L> Valuable for AutoComplete<T,L> where T: Valuable {
    type Output = T::Output;
    fn value(&self) -> Result<Self::Output, Error> {
        self.inner.value()
    }
}

impl<T,L> Typeable<KeyEvent> for AutoComplete<T,L> where
    T: Typeable<KeyEvent> + Valuable,
    L: LookUp,
    <T as Valuable>::Output: AsRef<str>,
    T: AsMut<String>
// L::Item: Display
{
    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        use crate::prompt::LookUpError::*;
        for code in &key.codes {
            match code {
                KeyCode::Tab => {
                    eprintln!("tab");
                      match self.inner.value() {
                          Ok(input) => match self.look_up.look_up(input.as_ref()) {
                              Ok(the_match) => self.channel.send(LookUpEvent::Hide),
                              Err(e) => match e {
                                  Message(s) => (), // Err(s),
                                  Incomplete(v) => {
                                      if v.len() == 1 {
                                          let mut text = self.inner.as_mut();
                                          *text = v.into_iter().next().unwrap()
                                      } else {
                                          self.channel.send(LookUpEvent::Completions(v))
                                      }
                                  },
                                  NanoError(e) => (), //Err(format!("Error: {:?}", e).into()),
                              },
                          }
                          Err(_) => ()
                      }
                }
                _ => ()
            }
        }
        self.inner.handle_key(key)
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

impl<T,L> Printable for AutoComplete<T,L> where T: Printable {
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
    ) -> impl Future<Output = Result<<asky::Text<'_> as Valuable>::Output, Error>> + '_ where
        L::Item: Display
    {

        use crate::prompt::LookUpError::*;
        let mut text = AutoComplete { inner: asky::Text::new(prompt),
                                      look_up: lookup,
                                      channel: self.channel.clone() };
        // text
        //     .validate(move |input|
        //               match lookup.look_up(input) {
        //                   Ok(_) => Ok(()),
        //                   Err(e) => match e {
        //                       Message(s) => Err(s),
        //                       Incomplete(v) => Err(format!("Incomplete: {}", v.join(", ")).into()),
        //                       NanoError(e) => Err(format!("Error: {:?}", e).into()),
        //                   },
        //               });
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

            }
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
}
