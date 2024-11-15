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
async fn ask_age(mut asky: MinibufferAsync) -> Result<(), Error> {
    let age = asky.prompt::<Number<u8>>("What's your age? ").await?;
    // asky.delay(Duration::from_secs(2)).await?;
    asky.message(format!("You are {age} years old."));
    Ok(())
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
        Act::new(ask_name.pipe(future_result_sink))
            .named("ask_name")
            .hotkey(keyseq!(ctrl-A N)),
    );
    commands.add(
        Act::new(ask_age.pipe(future_result_sink))
            .named("ask_age")
            .hotkey(keyseq!(ctrl-A A)),
    );

    // Add a builtin act.
    commands.add(Builtin::default().take("exec_act").unwrap());
}
