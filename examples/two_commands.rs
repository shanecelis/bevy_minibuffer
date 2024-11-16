use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

/// Ask the user for their name. Say hello.
fn ask_name(mut minibuffer: Minibuffer) {
    minibuffer
        .prompt::<TextField>("What's your first name? ")
        .observe(
            |trigger: Trigger<AskyEvent<String>>, mut minibuffer: Minibuffer| {
                let first_name = trigger.event().0.clone().unwrap();
                minibuffer
                    .prompt::<TextField>("What's your last name? ")
                    .observe(
                        move |trigger: Trigger<AskyEvent<String>>, mut minibuffer: Minibuffer| {
                            let last_name = trigger.event().0.clone().unwrap();
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
            |trigger: Trigger<AskyEvent<u8>>, mut minibuffer: Minibuffer| {
                let age = trigger.event().0.clone().unwrap();
                minibuffer.message(format!("You are {age} years old."));
            },
        );
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add(
        Act::new(ask_name)
            .named("ask_name")
            .hotkey(keyseq!(ctrl-A N)),
    );
    commands.add(Act::new(ask_age).named("ask_age").hotkey(keyseq!(ctrl-A A)));

    // Add a builtin act.
    commands.add(Builtin::default().take("exec_act").unwrap());
}
