//! Illustrates how to interact with an entity with [Minibuffer].
use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
use std::f32::consts::TAU;

#[path = "common/lib.rs"]
mod common;

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins)
        .add_acts((
            BasicActs::default(),
            // Add commands.
            Act::new(stop).bind(keyseq! { A }),
            Act::new(speed).bind(keyseq! { S }),
            Act::new(start).bind(keyseq! { D }),
            Act::new_with_input(speed_scriptable).bind(keyseq! { F }),
        ))
        .add_systems(Startup, |mut minibuffer: Minibuffer| {
            minibuffer.message("Hit A, S, or D to change cube speed. Hit 'Ctrl-H B' for keys.");
            minibuffer.set_visible(true);
        });
}

fn main() {
    App::new()
        .add_plugins((
            common::VideoCapturePlugin::new("cube").background(Srgba::hex("390099").unwrap()),
            plugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cube)
        .run();
}

/// Start the cube spinning.
fn start(mut query: Query<&mut Rotatable>) {
    if let Ok(mut r) = query.single_mut() {
        r.speed = 0.3;
    }
}

/// Stop the cube spinning. No input.
fn stop(mut query: Query<&mut Rotatable>, mut minibuffer: Minibuffer) {
    minibuffer.clear();
    if let Ok(mut r) = query.single_mut() {
        r.speed = 0.0;
    }
}

/// Set the speed of the spinning cube with input.
fn speed(mut minibuffer: Minibuffer) {
    minibuffer.prompt::<Number<f32>>("speed: ").observe(
        |mut trigger: Trigger<Submit<f32>>, mut query: Query<&mut Rotatable>| {
            let speed = trigger.event_mut().take_result().expect("speed");
            for mut r in &mut query {
                r.speed = speed;
            }
        },
    );
}

/// Set the speed of the spinning cube with input.
fn speed_scriptable(
    In(number_maybe): In<Option<f32>>,
    mut minibuffer: Minibuffer,
    mut query: Query<&mut Rotatable>,
) {
    if let Some(speed) = number_maybe {
        for mut r in &mut query {
            r.speed = speed;
        }
    } else {
        minibuffer.prompt::<Number<f32>>("speed: ").observe(
            |mut trigger: Trigger<Submit<f32>>,
             mut query: Query<&mut Rotatable>,
             mut minibuffer: Minibuffer| {
                let speed = trigger.event_mut().take_result().expect("speed");
                minibuffer.log_input(&Some(speed));
                for mut r in &mut query {
                    r.speed = speed;
                }
            },
        );
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to rotate.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_translation(Vec3::ZERO),
        Rotatable { speed: 0.3 },
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add a light source so we can see clearly.
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// This system will rotate any entity in the scene with a Rotatable component around its y-axis.
fn rotate_cube(mut cubes: Query<(&mut Transform, &Rotatable)>, timer: Res<Time>) {
    for (mut transform, cube) in &mut cubes {
        // The speed is first multiplied by TAU which is a full rotation (360deg) in radians,
        // and then multiplied by delta_secs which is the time that passed last frame.
        // In other words. Speed is equal to the amount of rotations per second.
        transform.rotate_y(cube.speed * TAU * timer.delta_secs());
    }
}
