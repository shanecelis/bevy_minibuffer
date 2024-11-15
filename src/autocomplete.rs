//! Provides autocomplete.
use crate::{
    prelude::*,
    lookup::LookUp,
};
use bevy_asky::{Submitter, string_cursor::*, construct::*, focus::{FocusParam, Focusable}};
use bevy::{
    ecs::world::{Command},
    ecs::system::{EntityCommands},
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};
use std::borrow::Cow;



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
/// # Examples
///
/// ```no_run
/// use asky::prelude::*;
///
/// # fn main() -> Result<(), Error> {
/// # #[cfg(feature = "terminal")]
/// let name = Input::new("What is your name?").prompt()?;
///
/// # #[cfg(feature = "terminal")]
/// println!("Hello, {}!", name);
///
/// # Ok(())
/// # }
/// ```
#[derive(Component)]
pub struct AutoComplete {
    look_up: Box<dyn LookUp + Send + Sync>,
    show_completions: bool,
}

impl AutoComplete
{
    /// Wrap a prompt in autocomplete.
    pub fn new<L>(look_up: L) -> Self
    where
        L: LookUp + Send + Sync + 'static,
    {
        Self {
            look_up: Box::new(look_up),
            show_completions: false,
        }
    }

    /// Construct an autocomplete UI element.
    pub fn construct<L: LookUp + Send + Sync + 'static>(mut commands: EntityCommands, prompt: impl Into<Cow<'static, str>>, lookup: L) -> EntityCommands {
        // let prompt = prompt.into();
        // move |world: &mut World| {
            // let mut commands = world.commands();
            // let mut commands = match entity {
            //     None => commands.spawn_empty(),
            //     Some(id) => commands.entity(id)
            // };
            commands
                .insert(Prompt(prompt.into()))
                .insert(NodeBundle::default())
                .insert(StringCursor::default())
                .insert(Focusable::default())
                // .insert(TextField)
                .insert(AutoComplete::new(lookup));
            commands
        // }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app
        .add_systems(PreUpdate, autocomplete_controller);
}

unsafe impl Submitter for AutoComplete {
    type Out = String;
}

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
//             .insert(NodeBundle::default())
//             .insert(input_state)
//             .insert(Focusable::default());

//         context.world.flush();

//         Ok(props.1)
//     }
// }

fn autocomplete_controller(
    mut focus: FocusParam,
    mut query: Query<
        (Entity, &mut StringCursor),
        With<AutoComplete>>,
    mut input: EventReader<KeyboardInput>,
    mut commands: Commands,
) {
    let mut any_focused_text = false;
    for (id, mut text_state) in query.iter_mut() {
        if !focus.is_focused(id) {
            continue;
        }
        any_focused_text |= true;
            for ev in input.read() {
                if ev.state != ButtonState::Pressed {
                    continue;
                }
                match &ev.logical_key {
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
                        warn!("trigger");
                        commands.trigger_targets(AskyEvent(Ok(text_state.value.clone())), id);
                        warn!("block&move");
                        focus.block_and_move(id);
                    }
                    Key::Escape => {
                        commands.trigger_targets(AskyEvent::<String>(Err(asky::Error::Cancel)), id);
                        commands.entity(id).insert(Feedback::error("canceled"));
                        focus.block(id);
                    }
                    x => info!("Unhandled key {x:?}"),
                }
        }
    }
    focus.set_keyboard_nav(!any_focused_text);
}
