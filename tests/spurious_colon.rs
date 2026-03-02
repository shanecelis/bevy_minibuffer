//! Test to reproduce the spurious ':' bug when hitting ':' to open the run-act prompt
//! (cube / BasicActs scenario). Press ':', release ':', press 'a', release 'a';
//! the only value present must be 'a'. Then Escape, repeat 100 times.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy_asky::string_cursor::StringCursor;
use bevy_minibuffer::prelude::*;

#[derive(Resource, Default)]
struct TestState {
    /// Content of the first text field (e.g. the run-act prompt input), if any.
    last_text_value: String,
}

fn track_text_field_value(query: Query<&StringCursor>, mut test_state: ResMut<TestState>) {
    if let Ok(cursor) = query.single() {
        test_state.last_text_value = cursor.value.clone();
    } else {
        test_state.last_text_value.clear();
    }
}

fn send_key_event(app: &mut App, event: KeyboardInput) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(event);
}

/// Shift key press (so ButtonInput has shift down when we press semicolon).
fn send_shift_press(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Shift,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::ShiftLeft,
            text: None,
            repeat: false,
        },
    );
}

fn send_shift_release(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Shift,
            state: ButtonState::Released,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::ShiftLeft,
            text: None,
            repeat: false,
        },
    );
}

/// ':' is Shift+; in BasicActs (run_act). Send Shift press first, then semicolon with character ':'.
fn send_colon_press(app: &mut App) {
    send_shift_press(app);
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Character(":".into()),
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::Semicolon,
            text: Some(":".into()),
            repeat: false,
        },
    );
}

fn send_colon_release(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Character(":".into()),
            state: ButtonState::Released,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::Semicolon,
            text: None,
            repeat: false,
        },
    );
    send_shift_release(app);
}

fn send_escape_press(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::Escape,
            text: None,
            repeat: false,
        },
    );
}

fn send_escape_release(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Escape,
            state: ButtonState::Released,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::Escape,
            text: None,
            repeat: false,
        },
    );
}

fn send_a_press(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Character("a".into()),
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::KeyA,
            text: Some("a".into()),
            repeat: false,
        },
    );
}

fn send_a_release(app: &mut App) {
    send_key_event(
        app,
        KeyboardInput {
            logical_key: Key::Character("a".into()),
            state: ButtonState::Released,
            window: Entity::PLACEHOLDER,
            key_code: KeyCode::KeyA,
            text: None,
            repeat: false,
        },
    );
}

#[test]
fn test_no_spurious_colon_on_run_act() {
    let mut app = App::new();

    // Replicate cube example: MinibufferPlugins + BasicActs (':’ opens run_act)
    app.add_plugins(MinimalPlugins)
        .add_plugins((
            bevy::state::app::StatesPlugin,
            bevy::input::InputPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
        ))
        .init_asset::<bevy::prelude::AudioSource>()
        .init_asset::<bevy::prelude::Image>()
        .add_plugins(MinibufferPlugins)
        .add_acts(BasicActs::default())
        .add_message::<bevy::window::RequestRedraw>()
        .init_resource::<TestState>()
        .add_systems(Update, track_text_field_value)
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Hit ':' for run-act (cube scenario).");
            minibuffer.set_visible(true);
        });

    app.update();

    const ATTEMPTS: u32 = 100;

    for attempt in 0..ATTEMPTS {
        // Ensure Inactive so key sequence can match ':'
        let mut state = app
            .world_mut()
            .resource_mut::<NextState<bevy_minibuffer::prompt::MinibufferState>>();
        state.set(bevy_minibuffer::prompt::MinibufferState::Inactive);
        drop(state);
        app.update();

        app.world_mut()
            .resource_mut::<TestState>()
            .last_text_value
            .clear();

        // Press ':' (hold Shift, press semicolon), release ':', press 'a', release 'a'
        send_colon_press(&mut app); // shift + semicolon in same frame so key sequence matches
        app.update();
        send_colon_release(&mut app); // semicolon release, then shift release
        app.update();
        app.update();
        app.update();

        send_a_press(&mut app);
        app.update();
        send_a_release(&mut app);
        app.update();

        let text_value = app.world().resource::<TestState>().last_text_value.clone();
        assert_eq!(
            text_value, "a",
            "On attempt {}: text field should contain only 'a', got '{}'. \
             Spurious ':' or wrong content (cube bug).",
            attempt, text_value
        );

        // Close prompt with Escape
        send_escape_press(&mut app);
        app.update();
        send_escape_release(&mut app);
        app.update();
    }
}
