//! Test to reproduce the spurious input bug where pressing a key that triggers
//! a command (like 'n' or ':') sometimes inserts that character into the newly
//! opened text field.

use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy_asky::string_cursor::StringCursor;

#[derive(Resource, Default)]
struct TestState {
    last_text_value: String,
}


fn track_text_field_value(
    query: Query<&StringCursor>,
    mut test_state: ResMut<TestState>,
) {
    // Count all StringCursor entities
    let count = query.iter().count();
    
    // Get the first text field found (there should only be one)
    if let Ok(cursor) = query.single() {
        test_state.last_text_value = cursor.value.clone();
    } else if count > 1 {
        // Multiple text fields found, get the first one
        if let Some(cursor) = query.iter().next() {
            test_state.last_text_value = cursor.value.clone();
        } else {
            test_state.last_text_value.clear();
        }
    } else {
        // No text field found, clear the value
        test_state.last_text_value.clear();
    }
}

#[test]
fn test_spurious_input_on_command_key() {
    let mut app = App::new();
    
    // Setup similar to two_commands example
    // Use DefaultPlugins but disable window/winit plugins for headless testing
    app
        .add_plugins(MinimalPlugins)
        .add_plugins((bevy::state::app::StatesPlugin, bevy::input::InputPlugin, bevy::asset::AssetPlugin::default(), bevy::text::TextPlugin))
        // .add_plugins(DefaultPlugins.build()
        // .disable::<bevy::window::WindowPlugin>()
        // .disable::<bevy::winit::WinitPlugin>())
        .init_asset::<bevy::prelude::AudioSource>()
        .init_asset::<bevy::prelude::Image>()
        .add_plugins(MinibufferPlugins)
        .add_acts((
            Act::new(ask_name).named("ask_name").bind(keyseq!(N)),
        ))
        .add_message::<bevy::window::RequestRedraw>() // Required for hide/show systems
        // .add_message::<bevy::window::WindowResized>() // Required for hide/show systems
        // .add_message::<bevy::window::WindowScaleFactorChanged>() // Required for hide/show systems
        .init_resource::<TestState>()
        .add_systems(Update, track_text_field_value)
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Hit 'N' for ask_name. Hit 'A' for ask_age.");
            minibuffer.set_visible(true);
        })
        ;
    
    // Run startup systems
    app.update();
    
    // Try multiple times to ensure the fix works
    for attempt in 0..10 {
        // Ensure MinibufferState is Inactive so bevy-input-sequence can process the key
        let mut state = app.world_mut().resource_mut::<NextState<bevy_minibuffer::prompt::MinibufferState>>();
        state.set(bevy_minibuffer::prompt::MinibufferState::Inactive);
        drop(state);
        app.update();
        
        // Reset test state
        app.world_mut().resource_mut::<TestState>().last_text_value.clear();
        
        // Press 'n' key to trigger the command
        simulate_key_press(&mut app, KeyCode::KeyN);
        app.update();
        
        // Release the key
        simulate_key_release(&mut app, KeyCode::KeyN);
        app.update();
        
        // Type 'a' - this should be the ONLY character in the field
        simulate_key_press(&mut app, KeyCode::KeyA);
        app.update();
        simulate_key_release(&mut app, KeyCode::KeyA);
        app.update();

        // Check the text field value - it should contain ONLY 'a', not 'na' or 'n'
        let test_state = app.world().resource::<TestState>();
        let final_value = &test_state.last_text_value;
        
        assert_eq!(
            final_value, "a",
            "Text field should contain only 'a', got '{}' on attempt {}",
            final_value, attempt
        );
        
        // Press Escape to close the text field
        simulate_key_press(&mut app, KeyCode::Escape);
        app.update();
        simulate_key_release(&mut app, KeyCode::Escape);
        app.update();
    }
}

fn ask_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your first name? ")
        .observe(
            |mut trigger: On<Submit<String>>, mut minibuffer: Minibuffer| {
                if let Ok(name) = trigger.event_mut().take_result() {
                    minibuffer.message(format!("Hello, {}!", name));
                } else {
                    minibuffer.clear();
                }
            },
        );
}

// Helper to send keyboard events
fn send_key_event(app: &mut App, event: KeyboardInput) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(event);
}

// Helper to create character events
fn create_char_event(c: char) -> KeyboardInput {
    use bevy::input::keyboard::KeyCode;
    // Map character to KeyCode for the key_code field
    let key_code = match c {
        'n' => KeyCode::KeyN,
        'N' => KeyCode::KeyN,
        'a' => KeyCode::KeyA,
        'A' => KeyCode::KeyA,
        'e' => KeyCode::KeyE,
        'l' => KeyCode::KeyL,
        'o' => KeyCode::KeyO,
        _ => KeyCode::KeyN, // fallback
    };
    KeyboardInput {
        logical_key: Key::Character(c.to_string().into()),
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
        key_code,
        text: Some(c.to_string().into()),
        repeat: false,
    }
}

fn create_key_event(key: Key) -> KeyboardInput {
    use bevy::input::keyboard::KeyCode;
    let key_code = match key {
        Key::Backspace => KeyCode::Backspace,
        Key::Delete => KeyCode::Delete,
        Key::ArrowLeft => KeyCode::ArrowLeft,
        Key::ArrowRight => KeyCode::ArrowRight,
        Key::Space => KeyCode::Space,
        Key::Escape => KeyCode::Escape,
        _ => KeyCode::Backspace, // fallback
    };
    KeyboardInput {
        logical_key: key,
        state: ButtonState::Pressed,
        window: Entity::PLACEHOLDER,
        key_code,
        text: None,
        repeat: false,
    }
}

fn simulate_key_press(app: &mut App, key_code: KeyCode) {
    // Create and send the appropriate KeyboardInput event
    let event = match key_code {
        KeyCode::KeyN => create_char_event('n'),
        KeyCode::KeyA => create_char_event('a'),
        KeyCode::Escape => create_key_event(Key::Escape),
        _ => panic!("Unexpected key code: {:?}", key_code),
    };
    send_key_event(app, event);
}

fn simulate_key_release(app: &mut App, key_code: KeyCode) {
    // Create a release event
    let logical_key = match key_code {
        KeyCode::KeyN => Key::Character("n".into()),
        KeyCode::KeyA => Key::Character("a".into()),
        KeyCode::Escape => Key::Escape,
        _ => panic!("Unexpected key code: {:?}", key_code),
    };
    
    let event = KeyboardInput {
        logical_key,
        key_code,
        state: ButtonState::Released,
        window: Entity::PLACEHOLDER,
        repeat: false,
        text: None,
    };
    send_key_event(app, event);
}

