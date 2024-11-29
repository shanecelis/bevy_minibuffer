//! Demonstrate universal argument
use rand::prelude::*;
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

fn rnd_vec<R: Rng>(rng: &mut R) -> Vec3 {
    2.0 * Vec3::new(rng.gen(), rng.gen(), rng.gen()) - Vec3::ONE
}

fn make_cube(
    arg: Res<UniversalArg>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut minibuffer: Minibuffer
) {
    let mut rng = rand::thread_rng();

    let cube_handle = meshes.add(Cuboid::new(rng.gen(), rng.gen(), rng.gen()));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(rng.gen(), rng.gen(), rng.gen()),
        ..default()
    });

    let count = arg.0.unwrap_or(1);
    for _ in 0..count {

        let v = 2.0 * rnd_vec(&mut rng);
        commands
            .spawn((
                PbrBundle {
                    mesh: cube_handle.clone(),
                    material: cube_material_handle.clone(),
                    transform: Transform::from_translation(v),
                    ..default()
                },
            ));
    }
    minibuffer.message(format!("Made {} cubes.", count));
}
pub fn check_arg(arg: Res<UniversalArg>, mut minibuffer: Minibuffer) {
    match arg.0 {
        Some(x) => minibuffer.message(format!("Univeral argument {x}")),
        None => minibuffer.message("No universal argument set"),
    }
}

fn plugin(app: &mut App) {
    app
        .add_acts((Builtin::default(),
                   UniversalArgPlugin::default(),
                   Act::new(make_cube)
                   .bind(keyseq! { Space }),
                   Act::new(check_arg)
                  .bind(keyseq! { C })
                  .add_flags(ActFlags::Show),
    ));
}

fn setup(mut commands: Commands) {
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, -4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(5.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn main() {
    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Universal Argument Example".into(),
    };
    App::new()
        // Normally, you'd do this:
        // .add_plugins((DefaultPlugins, MinibufferPlugins, plugin))
        // For the demo, we do it slightly differently like this:
        .add_plugins((
            DefaultPlugins.set(video_settings.window_plugin()),
            MinibufferPlugins.set(video_settings.minibuffer_plugin()),
            plugin
        ))
        .add_systems(Startup, setup)
        .run();
}
