//! Demonstrate two commands.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt::<TextField>("What's your first name? ")
        .await?;
    let last_name = minibuffer
        .prompt::<TextField>("What's your last name? ")
        .await?;
    minibuffer.message(format!("Hello, {first_name} {last_name}!"));
    Ok(())
}

// Ask the user for their age.
async fn ask_age(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let age = minibuffer.prompt::<Number<u8>>("What's your age? ").await?;
    minibuffer.message(format!("You are {age} years old."));
    Ok(())
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_systems(Startup, setup)
        .add_acts((
            Act::new(ask_name.pipe(future_result_sink))
                .named("ask_name")
                .bind(keyseq!(Ctrl-A N)),
            Act::new(ask_age.pipe(future_result_sink))
                .named("ask_age")
                .bind(keyseq!(Ctrl-A A)),
            // Add a basic act but just one of them.
            BasicActs::default().remove("exec_act").unwrap(),
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
