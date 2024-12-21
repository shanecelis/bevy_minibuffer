//! A simple 3D scene to demonstrate mesh picking.
//!
//! [`bevy::picking::backend`] provides an API for adding picking hit tests to any entity. To get
//! started with picking 3d meshes, the [`MeshPickingPlugin`] is provided as a simple starting
//! point, especially useful for debugging. For your game, you may want to use a 3d picking backend
//! provided by your physics engine, or a picking shader, depending on your specific use case.
//!
//! [`bevy::picking`] allows you to compose backends together to make any entity on screen pickable
//! with pointers, regardless of how that entity is rendered. For example, `bevy_ui` and
//! `bevy_sprite` provide their own picking backends that can be enabled at the same time as this
//! mesh picking backend. This makes it painless to deal with cases like the UI or sprites blocking
//! meshes underneath them, or vice versa.
//!
//! If you want to build more complex interactions than afforded by the provided pointer events, you
//! may want to use [`MeshRayCast`] or a full physics engine with raycasting capabilities.
//!
//! By default, the mesh picking plugin will raycast against all entities, which is especially
//! useful for debugging. If you want mesh picking to be opt-in, you can set
//! [`MeshPickingSettings::require_markers`] to `true` and add a [`RayCastPickable`] component to
//! the desired camera and target entities.

use std::f32::consts::PI;

use bevy::{color::palettes::tailwind::*, picking::pointer::PointerInteraction, prelude::*};
use bevy_minibuffer::prelude::*;

fn main() {
    App::new()
        // MeshPickingPlugin is not a default plugin
        .add_plugins((DefaultPlugins, MeshPickingPlugin, MinibufferPlugins))
        .init_resource::<Selected>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (draw_mesh_intersections, rotate, update_color))
        .add_acts((BasicActs::default(),
        set_color))
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

#[derive(Resource, Default)]
struct Selected(Option<Entity>);

#[derive(Component, Clone, Copy)]
struct Paint {
    base: Color,
    tone: Option<(Color, f32)>
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            base: Color::WHITE,
            tone: None,
        }
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Set up the materials.
    let white_matl = Color::WHITE;
    let ground_color = Color::from(GRAY_300);
    let hover_color = Some(Color::from(CYAN_300));
    let pressed_color = Some(Color::from(YELLOW_300));

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    let num_shapes = shapes.len();

    // Spawn the shapes. The meshes will be pickable by default.
    for (i, shape) in shapes.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(materials.add(Color::WHITE)),
                Transform::from_xyz(
                    -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                    2.0,
                    Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
                Paint::default(),
            ))
            .observe(update_color_on::<Pointer<Over>>(hover_color.clone()))
            .observe(update_color_on::<Pointer<Out>>(None))
            .observe(update_color_on::<Pointer<Down>>(pressed_color.clone()))
            .observe(update_color_on::<Pointer<Up>>(hover_color.clone()))
            .observe(select)
            .observe(rotate_on_drag);
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(materials.add(Color::WHITE)),
                Transform::from_xyz(
                    -EXTRUSION_X_EXTENT / 2.
                        + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                    2.0,
                    -Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
                Paint::default(),
            ))
            .observe(update_color_on::<Pointer<Over>>(hover_color.clone()))
            .observe(update_color_on::<Pointer<Out>>(None))
            .observe(update_color_on::<Pointer<Down>>(pressed_color.clone()))
            .observe(update_color_on::<Pointer<Up>>(hover_color.clone()))
            // .observe(update_color_on::<Pointer<Click>>(click_matl.clone()))
            .observe(select)
            .observe(rotate_on_drag);
    }

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(ground_color)),
        PickingBehavior::IGNORE, // Disable picking for the ground plane.
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Instructions
    commands.spawn((
        Text::new("Hover over the shapes to pick them\nDrag to rotate"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn set_color(mut minibuffer: Minibuffer, selected: Res<Selected>) {
    if selected.0.is_none() {
        minibuffer.message("Select a shape first.");
        return;
    } else {
        let selection = selected.0.unwrap();
        minibuffer.prompt_map("Hex color: ", bevy_minibuffer::autocomplete::HexColorLookup)
            .observe(move |mut trigger: Trigger<Completed<Color>>,
                     mut selected: ResMut<Selected>,
                     mut paints: Query<&mut Paint>,
                     mut commands: Commands, mut minibuffer: Minibuffer| {
                if let Completed::Unhandled { result, input } = trigger.event_mut().take() {
                    match result {
                        Ok(color) => {
                            if let Ok(mut paint) = paints.get_mut(selection) {
                                selected.0 = None;
                                minibuffer.message(format!("Set color to {:?}", &color));
                                paint.base = color;
                                paint.tone = None;
                            }
                            commands.entity(trigger.entity()).despawn_recursive();
                        }
                        Err(e) => {
                        }
                    }
                } else {
                    commands.entity(trigger.entity()).despawn_recursive();
                }
            });
    }
}

fn select(trigger: Trigger<Pointer<Click>>,
          mut selected: ResMut<Selected>,
          mut paints: Query<&mut Paint>,
) {
    info!("click on {:?}", trigger.entity());
    match std::mem::replace(&mut *selected, Selected(Some(trigger.entity()))) {
        Selected(Some(id)) => {
            /// Reset the selection of previous.
            if let Ok(mut paint) = paints.get_mut(id) {
                paint.tone = None;
            }
        }
        Selected(None) => ()
    }
}

fn change_color(mut handle: Handle<StandardMaterial>, materials: &mut Assets<StandardMaterial>, color: Color) {
    if let Some(mut material) = materials.get_mut(&mut handle) {
        material.base_color = color;
    }
}

fn update_color(
    mut query: Query<(&mut MeshMaterial3d<StandardMaterial>, &Paint), Changed<Paint>>,
    mut materials: ResMut<Assets<StandardMaterial>>) {
    for (mut mesh_material, paint) in &mut query {
        if let Some(mut material) = materials.get_mut(&mut mesh_material.0) {
            material.base_color = match paint.tone {
                Some((tone, k)) => paint.base.mix(&tone, k),
                None => paint.base
            };
        }
    }
}

/// Returns an observer that updates the entity's material to the one specified.
fn update_color_on<E>(
    color: Option<Color>,
) -> impl Fn(Trigger<E>,
             Query<&mut Paint>,
             Res<Selected>) {
    let selected_color = Color::from(RED_800);
    move |trigger, mut query, selected| {
        if let Ok(mut paint) = query.get_mut(trigger.entity()) {
            if selected.0.map(|x| x == trigger.entity()).unwrap_or(false) {
                // We're selected.
                paint.tone = Some((selected_color, 0.8));
            } else {
                paint.tone = color.map(|c| (c, 0.9));
            }
        }
    }
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

/// A system that rotates all shapes.
fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/// An observer to rotate an entity when it is dragged
fn rotate_on_drag(drag: Trigger<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    let mut transform = transforms.get_mut(drag.entity()).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
}
