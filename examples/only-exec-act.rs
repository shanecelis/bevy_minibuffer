//! Only bring in exec_act
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
       .add_acts(Builtin::default().take_acts().remove("exec_act").unwrap());
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, plugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());
        })
        .run();
}
