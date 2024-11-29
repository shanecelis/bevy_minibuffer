use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

/// Ask the user for their name. Say hello.
fn ask_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your first name? ")
        .observe(
            |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                let first_name = trigger.event_mut().take_result().unwrap();
                minibuffer
                    .prompt::<TextField>("What's your last name? ")
                    .observe(
                        move |mut trigger: Trigger<Submit<String>>, mut minibuffer: Minibuffer| {
                            let last_name = trigger.event_mut().take_result().unwrap();
                            minibuffer.message(format!("Hello, {first_name} {last_name}!"));
                        },
                    );
            },
        );
}

// Ask the user for their age.
fn ask_age(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<Number<u8>>("What's your age? ")
        .observe(
            |mut trigger: Trigger<Submit<u8>>, mut minibuffer: Minibuffer| {
                let age = trigger.event_mut().take_result().unwrap();
                minibuffer.message(format!("You are {age} years old."));
            },
        );
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_systems(Startup, setup)
        .add_acts((
            Act::new(ask_name)
                .named("ask_name")
                .hotkey(keyseq!(Ctrl-A N)),
            Act::new(ask_age).named("ask_age").hotkey(keyseq!(Ctrl-A A)),
            // Add a builtin act but just one of them.
            Builtin::default().remove("exec_act").unwrap(),
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
