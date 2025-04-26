//! Tab completion functionality
use crate::{event::LookupEvent, prelude::*};
use bevy::{
    diagnostic::FrameCount,
    ecs::system::EntityCommands,
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};
use bevy_asky::{
    AskySet,
    construct::ConstructExt,
    focus::{FocusParam, Focusable},
    string_cursor::*,
    Submitter,
};
use std::borrow::Cow;
mod lookup;
pub use lookup::*;

/// Prompt to get one-line user input.
///
/// # Key Events
///
/// | Key         | Action                       |
/// | ----------- | ---------------------------- |
/// | `Enter`     | Submit current/initial value |
/// | `Backspace` | Delete previous character    |
/// | `Delete`    | Delete current character     |
/// | `Left`      | Move cursor left             |
/// | `Right`     | Move cursor right            |
///
#[derive(Component, Deref)]
// #[reflect(opaque)]
pub(crate) struct AutoComplete(Box<dyn Lookup + Send + Sync>);

/// Means that an auto completing read must match one of its lookups.
#[derive(Component, Debug)]
pub struct RequireMatch;

impl AutoComplete {
    /// Wrap a prompt in autocomplete.
    pub fn new<L>(lookup: L) -> Self
    where
        L: Lookup + Send + Sync + 'static,
    {
        Self(Box::new(lookup))
    }

    /// Construct an autocomplete UI element.
    ///
    /// NOTE: Could not use the `Construct` trait.
    pub fn construct(
        self,
        mut commands: EntityCommands,
        prompt: impl Into<Cow<'static, str>>,
    ) -> EntityCommands {
        commands
            .insert(Prompt(prompt.into()))
            .insert(Node {
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .insert(StringCursor::default())
            .insert(Focusable::default())
            .construct::<crate::view::View>(())
            .insert(self);
        commands
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, autocomplete_controller.in_set(AskySet::Controller))
        .add_systems(Update, crate::view::text_view::<With<AutoComplete>>.in_set(AskySet::View));
}

unsafe impl Submitter for AutoComplete {
    type Out = String;
}

// NOTE: Construct didn't work for AutoComplete because my lookup field could
// not be a property.
//
// impl Construct for AutoComplete {
//     type Props = (Cow<'static, str>, AutoComplete);
//     fn construct(
//         context: &mut ConstructContext,
//         props: Self::Props,
//     ) -> Result<Self, ConstructError> {
//         // Our requirements.
//         let input_state = StringCursor::default();
//         let mut commands = context.world.commands();
//         commands
//             .entity(context.id)
//             .insert(Prompt(props.0))
//             .insert(input_state)
//             .insert(Focusable::default());
//         context.world.flush();
//         Ok(props.1)
//     }
// }

fn autocomplete_controller(
    mut focus: FocusParam,
    mut query: Query<(
        Entity,
        &mut StringCursor,
        &AutoComplete,
        Option<&RequireMatch>,
    )>,
    mut input: EventReader<KeyboardInput>,
    mut commands: Commands,
    mut lookup_events: EventWriter<LookupEvent>,
    frame_count: Res<FrameCount>,
) {
    let mut any_focused_text = false;
    for (id, mut text_state, autocomplete, require_match) in query.iter_mut() {
        if !focus.is_focused(id) {
            continue;
        }
        any_focused_text |= true;
        for ev in input.read() {
            if ev.state != ButtonState::Pressed {
                continue;
            }
            trace!("input {:?} frame {}", &ev.logical_key, frame_count.0);
            match &ev.logical_key {
                Key::Tab => {
                    if let Err(e) = autocomplete.lookup(&text_state.value) {
                        use LookupError::*;
                        match e {
                            NoMatch => {
                                lookup_events.send(LookupEvent::Hide);
                                if let Some(mut ecommands) = commands.get_entity(id).ok() {
                                    ecommands.try_insert(Feedback::info(format!("{}", e)));
                                }
                            }
                            OneMatch(s) => {
                                lookup_events.send(LookupEvent::Hide);
                                text_state.set_value(&s);
                            }
                            ManyMatches => {
                                let matches = autocomplete.all_lookups(&text_state.value);

                                lookup_events.send(LookupEvent::Completions(matches));
                                if let Some(new_input) =
                                    autocomplete.longest_prefix(&text_state.value)
                                {
                                    text_state.set_value(&new_input);
                                }
                            }
                            Message(s) => {
                                lookup_events.send(LookupEvent::Hide);
                                if let Some(mut ecommands) = commands.get_entity(id).ok() {
                                    ecommands.try_insert(Feedback::info(s)); // Err(s),
                                }
                            }
                        }
                    }
                }
                Key::Character(s) => {
                    for c in s.chars() {
                        text_state.insert(c);
                    }
                }
                Key::Space => text_state.insert(' '),
                Key::Backspace => text_state.backspace(),
                Key::Delete => text_state.delete(),
                Key::ArrowLeft => text_state.move_cursor(CursorDirection::Left),
                Key::ArrowRight => text_state.move_cursor(CursorDirection::Right),
                Key::Enter => {
                    if require_match.is_some() {
                        if let Err(e) = autocomplete.lookup(&text_state.value) {
                            use LookupError::*;
                            match e {
                                NoMatch => {
                                    lookup_events.send(LookupEvent::Hide);
                                    if let Some(mut ecommands) = commands.get_entity(id).ok() {
                                        ecommands.try_insert(Feedback::info(format!("{}", e)));
                                    }
                                }
                                Message(s) => {
                                    lookup_events.send(LookupEvent::Hide);
                                    if let Some(mut ecommands) = commands.get_entity(id).ok() {
                                        ecommands.try_insert(Feedback::info(s));
                                    }
                                }
                                OneMatch(s) => {
                                    lookup_events.send(LookupEvent::Hide);
                                    text_state.set_value(&s);
                                }
                                ManyMatches => {
                                    if let Some(mut ecommands) = commands.get_entity(id).ok() {
                                        ecommands.try_insert(Feedback::warn("require match"));
                                    }
                                    let matches = autocomplete.all_lookups(&text_state.value);

                                    lookup_events.send(LookupEvent::Completions(matches));
                                    if let Some(new_input) =
                                        autocomplete.longest_prefix(&text_state.value)
                                    {
                                        text_state.set_value(&new_input);
                                    }
                                }
                            }
                            continue;
                        }
                    }
                    lookup_events.send(LookupEvent::Hide);
                    commands.trigger_targets(Submit::new(Ok(text_state.value.clone())), id);
                    focus.block_and_move(id);
                }
                Key::Escape => {
                    commands
                        .trigger_targets(Submit::<String>::new(Err(bevy_asky::Error::Cancel)), id);
                    if let Some(mut ecommands) = commands.get_entity(id).ok() {
                        ecommands.try_insert(Feedback::error("canceled"));
                    }
                    focus.block(id);
                }
                _x => {
                    // info!("Unhandled key {x:?}");
                }
            }
        }
    }
    focus.set_keyboard_nav(!any_focused_text);
}
