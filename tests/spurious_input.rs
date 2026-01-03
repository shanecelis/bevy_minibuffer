//! Test to reproduce the spurious input bug where pressing a key that triggers
//! a command (like 'n' or ':') sometimes inserts that character into the newly
//! opened text field.

use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy_asky::string_cursor::StringCursor;
use bevy_minibuffer::ui::MinibufferNode;

#[derive(Resource, Default)]
struct TestState {
    attempts: u32,
    found_spurious: bool,
    last_text_value: String,
}

fn check_text_field(
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
        .add_plugins(MinibufferPlugins)
        .add_acts((
            Act::new(ask_name).named("ask_name").bind(keyseq!(N)),
            BasicActs::default().remove("run_act").unwrap(),
        ))
        .add_message::<bevy::window::RequestRedraw>() // Required for hide/show systems
        // .add_message::<bevy::window::WindowResized>() // Required for hide/show systems
        // .add_message::<bevy::window::WindowScaleFactorChanged>() // Required for hide/show systems

        .init_resource::<TestState>()
        .add_systems(Update, check_text_field)
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Hit 'N' for ask_name. Hit 'A' for ask_age.");
            minibuffer.set_visible(true);
        })
        ;
    
    // Run startup systems (PreStartup runs before Startup)
    app.update();
    
    // Check if MinibufferNode entity was created by spawn_layout
    let mut minibuffer_node_query = app.world_mut().query_filtered::<Entity, With<MinibufferNode>>();
    let minibuffer_node_count = minibuffer_node_query.iter(app.world()).count();
    println!("MinibufferNode entities found: {}", minibuffer_node_count);
    
    if minibuffer_node_count == 0 {
        panic!("MinibufferNode entity was not created! spawn_layout may not have run or failed.");
    } else if minibuffer_node_count > 1 {
        panic!("Multiple MinibufferNode entities found (expected 1): {}", minibuffer_node_count);
    }
    
    // Get the MinibufferNode entity
    let minibuffer_node_entity = minibuffer_node_query.single(app.world())
        .expect("Should have exactly one MinibufferNode");
    println!("MinibufferNode entity: {:?}", minibuffer_node_entity);
    
    // Check if it has children (the UI structure should have been created)
    if let Some(children) = app.world().get::<Children>(minibuffer_node_entity) {
        println!("MinibufferNode has {} children", children.len());
        for (i, child) in children.iter().enumerate() {
            println!("  Child {}: {:?}", i, child);
        }
    } else {
        println!("MinibufferNode has no children");
    }
    
    // Check if minibuffer is visible
    let prompt_state = app.world().resource::<State<bevy_minibuffer::prompt::PromptState>>();
    let is_visible = matches!(**prompt_state, bevy_minibuffer::prompt::PromptState::Visible);
    println!("Minibuffer visible: {}", is_visible);
    if !is_visible {
        panic!("Minibuffer is not visible! Cannot run test.");
    }
    
    // Try multiple times to provoke the bug
    for attempt in 0..10 {
        let mut test_state = app.world_mut().resource_mut::<TestState>();
        test_state.attempts = attempt;
        test_state.found_spurious = false;
        test_state.last_text_value.clear();
        drop(test_state);
        
        println!("Attempt {}", attempt);
        
        // Ensure MinibufferState is Inactive so bevy-input-sequence can process the key
        // (InputSequenceSet only runs when MinibufferState::Inactive)
        let mut state = app.world_mut().resource_mut::<NextState<bevy_minibuffer::prompt::MinibufferState>>();
        state.set(bevy_minibuffer::prompt::MinibufferState::Inactive);
        drop(state);
        
        // Update to apply state transition
        app.update();
        
        // Verify state is actually Inactive before proceeding
        let minibuffer_state_before = app.world().resource::<State<bevy_minibuffer::prompt::MinibufferState>>();
        let is_inactive_before = matches!(**minibuffer_state_before, bevy_minibuffer::prompt::MinibufferState::Inactive);
        println!("  Before 'n' key: MinibufferState is Inactive: {}", is_inactive_before);
        drop(minibuffer_state_before);
        
        if !is_inactive_before {
            println!("  WARNING: MinibufferState is not Inactive! InputSequenceSet will not run!");
            continue;
        }
        
        // Step 1: Simulate pressing 'n' key (this should trigger the command and open text field)
        // Ensure key is released first so InputPlugin can detect the press transition
        let mut button_input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if button_input.pressed(KeyCode::KeyN) {
            button_input.release(KeyCode::KeyN);
        }
        drop(button_input);
        app.update(); // Let InputPlugin process the release
        
        // Now press the key - InputPlugin should detect this transition
        simulate_key_press(&mut app, KeyCode::KeyN);
        
        // Update once so InputPlugin can process the press and set just_pressed
        app.update();
        
        // Update again to let bevy-input-sequence process the just_pressed key
        app.update();
        
        // Now release the key
        simulate_key_release(&mut app, KeyCode::KeyN);
        app.update();
        
        // Give it a few updates to process the command and create the text field
        for _ in 0..20 {
            app.update();
        }
        
        // Check that minibuffer went into Active state after 'n' was pressed
        let minibuffer_state = app.world().resource::<State<bevy_minibuffer::prompt::MinibufferState>>();
        let is_active = matches!(**minibuffer_state, bevy_minibuffer::prompt::MinibufferState::Active);
        let current_state = **minibuffer_state;
        drop(minibuffer_state);
        
        // Check for Focusable entities (text fields should have this)
        let mut focusable_query = app.world_mut().query_filtered::<Entity, With<bevy_asky::focus::Focusable>>();
        let focusable_count = focusable_query.iter(app.world()).count();
        drop(focusable_query);
        
        // Check for GetKeyChord components (used for key input)
        let mut get_key_chord_query = app.world_mut().query_filtered::<Entity, With<bevy_minibuffer::prompt::GetKeyChord>>();
        let get_key_chord_count = get_key_chord_query.iter(app.world()).count();
        drop(get_key_chord_query);
        
        println!("After 'n' key: MinibufferState is Active: {}", is_active);
        println!("  Focusable entities found: {}", focusable_count);
        println!("  GetKeyChord entities found: {}", get_key_chord_count);
        
        if !is_active {
            println!("WARNING: MinibufferState did not transition to Active after pressing 'n'!");
            println!("  Current state: {:?}", current_state);
            println!("  This suggests the command may not have triggered or the text field wasn't created/focused");
        }
        
        // Step 2: Check if text field exists, if not, skip this attempt
        let test_state_before = app.world().resource::<TestState>();
        
        if test_state_before.last_text_value.is_empty() {
            println!("No text field found after 'n' key, skipping attempt");
            continue;
        }
        
        println!("Text field created, initial value: '{}'", test_state_before.last_text_value);
        
        // Step 3: Type 'a' - this should be the ONLY character in the field
        // If 'n' is also there, that's the bug!
        simulate_key_press(&mut app, KeyCode::KeyA);
        app.update();
        
        // Release the key
        simulate_key_release(&mut app, KeyCode::KeyA);
        app.update();
        
        // Give it a few updates to process the 'a' key
        for _ in 0..20 {
            app.update();
        }
        
        // Step 4: Check the text field value
        let test_state_after = app.world().resource::<TestState>();
        let final_value = &test_state_after.last_text_value;
        
        println!("After typing 'a', text field value: '{}'", final_value);
        
        // The text field should contain ONLY 'a', not 'na' or 'n'
        if final_value == "a" {
            println!("✓ Correct: Text field contains only 'a'");
        } else if final_value.starts_with('n') {
            panic!(
                "BUG REPRODUCED on attempt {}: Text field has spurious 'n' character! Expected 'a', got '{}'",
                attempt, final_value
            );
        } else if final_value.is_empty() {
            println!("Text field is empty (might have been closed)");
        } else {
            panic!(
                "Unexpected text field value on attempt {}: Expected 'a', got '{}'",
                attempt, final_value
            );
        }
        
        // Step 5: Press Escape to close the text field
        simulate_key_press(&mut app, KeyCode::Escape);
        app.update();
        
        // Release the key
        simulate_key_release(&mut app, KeyCode::Escape);
        app.update();
        
        // Give it a few updates to process
        for _ in 0..20 {
            app.update();
        }
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
        .resource_mut::<Events<KeyboardInput>>()
        .send(event);
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

