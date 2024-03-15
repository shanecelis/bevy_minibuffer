use bevy::prelude::*;
use bevy_nano_console::commands::*;
use bevy_nano_console::prompt::*;
use bevy_nano_console::tasks::*;
// use bevy_nano_console::ui::*;
use bevy_nano_console::proc::*;
use bevy_nano_console::*;
use bevy_input_sequence::*;
use std::f32::consts::TAU;
use std::future::Future;

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn ask_name<'a>(mut prompt: Prompt) -> impl Future<Output = ()> {
    async move {
        if let Ok(first_name) = prompt.read::<String>("What's your first name? ").await {
            if let Ok(last_name) = prompt.read::<String>("What's your last name? ").await {
                prompt.message(format!("Hello, {first_name} {last_name}!"));
            }
        } else {
            eprintln!("Got err in ask name");
        }
    }
}

fn ask_age(mut prompt: Prompt) -> impl Future<Output = ()> {
    async move {
        if let Ok(age) = prompt.read::<i32>("What's your age? ").await {
            prompt.message(format!("You are {age} years old."));
        } else {
            eprintln!("Got err in ask age");
        }
    }
}

fn main() {
    App::new()
        // When spinning the cube, the frames will stop when prompting
        // if we're in desktop_app mode.
        // .insert_resource(WinitSettings::desktop_app()) // Lower CPU usage.
        .add_plugins(NanoPromptPlugin)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: "Bevy NanoPrompt Basic Example".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        // .add_command("ask_name", ask_name.pipe(task_sink))
        .add_command(
            // Command::new("ask_name", Some(vec![KeyCode::Key1])),
            Command::new("ask_name", keyseq!(1)),
            ask_name.pipe(task_sink),
        )
        .add_command(
            Command::new("ask_age", keyseq!(A A)),
            ask_age.pipe(task_sink),
        )
        .add_command(
            Command::new("exec_command", keyseq!(shift-;)).autocomplete(false),
            exec_command.pipe(task_sink),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cube)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to rotate.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0))),
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
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::ONE * 3.0),
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
