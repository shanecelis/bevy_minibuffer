//! Ask the user a question with tab completion.
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use trie_rs::map::Trie;

#[derive(Debug, Clone)]
enum Popular {
    Common,
    Uncommon,
    Rare
}

fn hello_name(mut minibuffer: Minibuffer) {
    let trie = Trie::from_iter([("John", Popular::Common),
                                ("Sean", Popular::Uncommon),
                                ("Shane", Popular::Rare)]);
    minibuffer.resolve("What's your name? ", trie)
        .observe(|mut trigger: Trigger<Mapped<Popular>>, mut minibuffer: Minibuffer| {
            let popular = trigger.event_mut().take_result();
            minibuffer.message(match popular {
                Ok(popular) => format!("That's an {:?} name.", popular),
                _ => "I don't know what kind of name that is.".into(),
            });
        });
}

fn plugin(app: &mut App) {
    app
        .add_systems(PostStartup, hello_name);
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MinibufferPlugins, plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
