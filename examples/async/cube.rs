//! Illustrates how to interact with an object with minibuffer.

use bevy::prelude::*;
use bevy_defer::{AsyncWorld, AsyncAccess};
use bevy_minibuffer::prelude::*;
use std::f32::consts::TAU;
use std::future::Future;
#[path = "../common/lib.rs"]
mod common;

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn main() {

    let video_settings = common::VideoCaptureSettings {
        title: "Bevy Minibuffer Cube Async Example".into()
    };
    App::new()
        // .add_plugins((DefaultPlugins, MinibufferPlugins))
        .add_plugins((DefaultPlugins.set(video_settings.window_plugin()),
                      MinibufferPlugins.set(video_settings.minibuffer_plugin())))
        // .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugins(Builtin::default().into_plugin())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cube)
        .add_plugins(ActsPlugin::new([
            Act::new(stop),
            Act::new(start),
            Act::new(speed.pipe(future_result_sink)),
        ]).into_plugin())
        .run();
}

/// Start the cube spinning.
fn start(mut query: Query<&mut Rotatable>) {
    let mut r = query.single_mut();
    r.speed = 0.3;
}

/// Stop the cube spinning. No input.
fn stop(mut query: Query<&mut Rotatable>) {
    let mut r = query.single_mut();
    r.speed = 0.0;
}

/// Set the speed of the spinning cube with input.
fn speed(
    mut minibuffer: MinibufferAsync,
    query: Query<Entity, With<Rotatable>>,
) -> impl Future<Output = Result<(), Error>> {
    let id = query.single();
    async move {
        let speed = minibuffer.prompt::<Number<f32>>("speed:").await?;
        let world = AsyncWorld::new();
        world
            .entity(id)
            .component::<Rotatable>()
            .set(move |r| r.speed = speed)?;
        Ok(())
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to rotate.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: materials.add(Color::WHITE),
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
        Rotatable { speed: 0.3 },
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add a light source so we can see clearly.
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// This system will rotate any entity in the scene with a Rotatable component around its y-axis.
fn rotate_cube(mut cubes: Query<(&mut Transform, &Rotatable)>, timer: Res<Time>) {
    for (mut transform, cube) in &mut cubes {
        // The speed is first multiplied by TAU which is a full rotation (360deg) in radians,
        // and then multiplied by delta_seconds which is the time that passed last frame.
        // In other words. Speed is equal to the amount of rotations per second.
        transform.rotate_y(cube.speed * TAU * timer.delta_seconds());
    }
}