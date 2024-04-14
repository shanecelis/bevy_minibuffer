//! Illustrates how to interact with an object with minibuffer.

use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use std::f32::consts::TAU;
use std::future::Future;
use asky::Number;
use bevy_defer::{AsyncAccess, world};
#[path = "common/lib.rs"] mod common;

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn main() {
    App::new()
        // .add_plugins(DefaultPlugins)
        // .add_plugins(MinibufferPlugin::default())
        .add_plugins(common::VideoCaptureSettings {
            title: "Bevy Minibuffer Cube Example".into(),
        })
        .add_systems(Startup, setup)
        .add_systems(Startup, add_builtins)
        .add_systems(Update, rotate_cube)
        .add_act(Act::new()
                 .named("stop"),
                 stop)
        .add_act(Act::new()
                 .named("start"),
                 start)
        .add_act(Act::new()
                 .named("speed"),
                 speed.pipe(future_sink))
        .run();
}

/// Add builtin commands.
fn add_builtins(world: &mut World) {
    let mut builtin = Builtin::new(world);
    for act in [
        builtin.exec_act(),
        builtin.list_acts(),
        builtin.list_key_bindings(),
    ] {
        world.spawn(act);
    }
}

/// Stop the cube spinning.
fn stop(mut query: Query<&mut Rotatable>) {
    let mut r = query.single_mut();
    r.speed = 0.0;
}

/// Start the cube spinning.
fn start(mut query: Query<&mut Rotatable>) {
    let mut r = query.single_mut();
    r.speed = 0.3;
}

/// Set the speed of the spinning cube.
fn speed(mut minibuffer: Minibuffer, query: Query<Entity, With<Rotatable>>) -> impl Future<Output = Result<(), Error>> {
    let id = query.single();
    async move {
        let speed = minibuffer.prompt(Number::new("speed:")).await?;
        let world = world();
        world.entity(id)
             .component::<Rotatable>()
            .set(move |r| r.speed = speed).await?;
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
