//! Test to understand how keyboard events persist across frames in Bevy

use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput, KeyCode};
use bevy::input::ButtonState;

#[derive(Resource, Default)]
struct EventTracker {
    press_events_seen: Vec<u32>,
    release_events_seen: Vec<u32>,
    current_frame: u32,
}

fn track_keyboard_events(
    mut input: MessageReader<KeyboardInput>,
    mut tracker: ResMut<EventTracker>,
) {
    tracker.current_frame += 1;
    let current_frame = tracker.current_frame;
    
    for ev in input.read() {
        match ev.state {
            ButtonState::Pressed => {
                tracker.press_events_seen.push(current_frame);
                println!("Frame {}: Saw PRESS event for {:?} (key_code: {:?}, logical_key: {:?})", 
                    current_frame, ev.key_code, ev.logical_key, ev.logical_key);
            }
            ButtonState::Released => {
                tracker.release_events_seen.push(current_frame);
                println!("Frame {}: Saw RELEASE event for {:?} (key_code: {:?}, logical_key: {:?})", 
                    current_frame, ev.key_code, ev.logical_key, ev.logical_key);
            }
        }
    }
}

#[test]
fn test_keyboard_event_persistence() {
    let mut app = App::new();
    
    app
        .add_plugins(MinimalPlugins)
        .add_plugins((bevy::state::app::StatesPlugin, bevy::input::InputPlugin))
        .add_message::<KeyboardInput>()
        .add_message::<bevy::window::RequestRedraw>()
        .init_resource::<EventTracker>()
        .add_systems(Update, track_keyboard_events);
    
    // Initial update
    app.update();
    println!("\n=== Initial frame ===");
    
    // Press 'n' key
    println!("\n=== Pressing 'n' key ===");
    send_key_press(&mut app, KeyCode::KeyN);
    
    // Update multiple times to see how long the event persists
    for i in 1..=10 {
        println!("\n--- Frame {} after press ---", i);
        app.update();
    }
    
    // Release 'n' key
    println!("\n=== Releasing 'n' key ===");
    send_key_release(&mut app, KeyCode::KeyN);
    
    // Update multiple times to see how long the release event persists
    for i in 1..=10 {
        println!("\n--- Frame {} after release ---", i);
        app.update();
    }
    
    // Print summary
    let tracker = app.world().resource::<EventTracker>();
    println!("\n=== Summary ===");
    println!("Press events seen at frames: {:?}", tracker.press_events_seen);
    println!("Release events seen at frames: {:?}", tracker.release_events_seen);
    println!("Total frames: {}", tracker.current_frame);
    
    // Assertions
    assert!(!tracker.press_events_seen.is_empty(), "Should have seen at least one press event");
    assert!(!tracker.release_events_seen.is_empty(), "Should have seen at least one release event");
    
    // Check that events are only seen once (MessageReader should consume them)
    assert_eq!(tracker.press_events_seen.len(), 1, 
        "Press event should only be seen once (consumed by MessageReader)");
    assert_eq!(tracker.release_events_seen.len(), 1, 
        "Release event should only be seen once (consumed by MessageReader)");
}

fn send_key_press(app: &mut App, key_code: KeyCode) {
    let event = create_char_event('n', key_code, ButtonState::Pressed);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(event);
}

fn send_key_release(app: &mut App, key_code: KeyCode) {
    let logical_key = match key_code {
        KeyCode::KeyN => Key::Character("n".into()),
        _ => Key::Character("n".into()),
    };
    
    let event = KeyboardInput {
        logical_key,
        key_code,
        state: ButtonState::Released,
        window: Entity::PLACEHOLDER,
        repeat: false,
        text: None,
    };
    
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(event);
}

fn create_char_event(c: char, key_code: KeyCode, state: ButtonState) -> KeyboardInput {
    KeyboardInput {
        logical_key: Key::Character(c.to_string().into()),
        state,
        window: Entity::PLACEHOLDER,
        key_code,
        text: if state == ButtonState::Pressed {
            Some(c.to_string().into())
        } else {
            None
        },
        repeat: false,
    }
}

