//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use trie_rs::map::Trie;

#[path = "common/lib.rs"]
mod common;

#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare,
}

fn hello_name(mut minibuffer: Minibuffer) {
    let trie = Trie::from_iter([
        ("John", Popular::Common),
        ("Sean", Popular::Uncommon),
        ("Shane", Popular::Rare),
    ]);
    minibuffer.resolve("What's your name? ", trie).observe(
        |mut trigger: Trigger<Resolved<Popular>>, mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's a {:?} name.", popular),
                _ => "I don't know what kind of name that is.".into(),
            });
        },
    );
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts(Act::new(hello_name).bind(keyseq! { Space }))
        .add_systems(PostStartup, hello_name);
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("tab-completion")
                .background(Srgba::hex("00f0b5").unwrap()),
            plugin,
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
