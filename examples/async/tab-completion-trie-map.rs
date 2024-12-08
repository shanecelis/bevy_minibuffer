//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use trie_rs::map::Trie;

#[path = "../common/lib.rs"]
mod common;

#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare,
}

async fn hello_name(mut minibuffer: MinibufferAsync) -> Result<(), Error> {
    let trie = Trie::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    let result = minibuffer.prompt_map("What's your name? ", trie).await;
    minibuffer.message(match result {
        Ok(popular) => format!("That's a {:?} name.", popular),
        _ => "I don't know what kind of name that is.".to_string(),
    });
    Ok(())
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts(Act::new(hello_name.pipe(future_result_sink)).bind(keyseq! { Space }))
        .add_systems(Startup, hello_name.pipe(future_result_sink));
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion-trie-map-async")
                .background(Srgba::hex("00f0b5").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
