use asky::{Message, Number};
use bevy::prelude::*;

use bevy_minibuffer::prelude::*;

/// Ask the user for their name. Say hello.
async fn ask_name(mut minibuffer: Minibuffer) -> Result<(), Error> {
    let first_name = minibuffer
        .prompt(asky::Text::new("What's your first name?"))
        .await?;
    let last_name = minibuffer
        .prompt(asky::Text::new("What's your last name?"))
        .await?;
    minibuffer
        .prompt(Message::new(format!("Hello, {first_name} {last_name}!")))
        .await?;
    Ok(())
}

// Ask the user for their age.
async fn ask_age(mut asky: Minibuffer) -> Result<(), Error> {
    let age = asky.prompt(Number::<u8>::new("What's your age?")).await?;
    // asky.delay(Duration::from_secs(2)).await?;
    asky.prompt(Message::new(format!("You are {age} years old.")))
        .await?;
    Ok(())
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MinibufferPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.add_act(
        Act::new().named("ask_name").hotkey(keyseq!(ctrl-A N)),
        ask_name.pipe(future_sink),
    );
    commands.add_act(
        Act::new().named("ask_age").hotkey(keyseq!(ctrl-A A)),
        ask_age.pipe(future_sink),
    );

    // Add a builtin act.
    commands.add_act(
        Act::new().named("exec_act")
                  .hotkey(keyseq!(shift-;)) // For vimmers a `:` key binding
                  .hotkey(keyseq!(alt-X))   // For Emacsers a `M-x` key binding
                  .in_exec_act(false),      // Don't show "exec_act" in its list
                                            // of acts.
        act::exec_act.pipe(future_sink),
    );
}
