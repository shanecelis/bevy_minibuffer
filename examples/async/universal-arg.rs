//! Demonstrate universal argument
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use rand::prelude::*;

#[path = "../common/lib.rs"]
mod common;

fn rnd_vec<R: Rng>(rng: &mut R) -> Vec3 {
    2.0 * Vec3::new(rng.gen(), rng.gen(), rng.gen()) - Vec3::ONE
}

fn make_cube(
    arg: Res<UniversalArg>,
    cubes: Query<Entity, With<Mesh3d>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut minibuffer: Minibuffer,
) {
    let mut rng = rand::thread_rng();

    let cube_handle = meshes.add(Cuboid::new(rng.gen(), rng.gen(), rng.gen()));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(rng.gen(), rng.gen(), rng.gen()),
        ..default()
    });

    let count = arg.0.unwrap_or(1);
    if count < 0 {
        let mut despawned = 0;
        for id in &cubes {
            commands.entity(id).despawn();
            despawned += 1;
            if despawned >= -count {
                break;
            }
        }
        minibuffer.message(format!(
            "Removed {} cube{}.",
            despawned,
            if count == 1 { "" } else { "s" }
        ));
    } else {
        for _ in 0..count {
            let v = 2.0 * rnd_vec(&mut rng);
            commands.spawn((
                Mesh3d(cube_handle.clone()),
                MeshMaterial3d(cube_material_handle.clone()),
                Transform::from_translation(v),
            ));
        }
        minibuffer.message(format!(
            "Made {} cube{}.",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }
}

fn open_door(universal_arg: Res<UniversalArg>, mut minibuffer: Minibuffer) {
    if universal_arg.is_none() {
        minibuffer.message("Open one door.");
    } else {
        minibuffer.message("Open all the doors.");
    }
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        UniversalArgActs::default()
            .use_async()
            .include_display_act(),
        Act::new(make_cube).bind(keyseq! { Space }),
        Act::new(open_door).bind(keyseq! { O D }),
    ));
}

fn setup(mut commands: Commands) {
    // light
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 5.0, -4.0)));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn main() {
    App::new()
        // .add_plugins((DefaultPlugins, plugin))
        .add_plugins((
            common::VideoCapturePlugin::new("universal-arg-async")
                .background(Srgba::hex("7678ed").unwrap()),
            plugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Type 'Ctrl-U 1 0 Space' to make 10 cubes or 'O D' to open a door.");
            minibuffer.set_visible(true);
        })
        .run();
}
