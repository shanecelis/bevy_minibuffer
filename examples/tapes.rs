//! Demonstrate tapes, i.e., macros
//!
//! ## Acknowledgments
//!
//! This example is based off of
//! [mesh_picking.rs](https://bevyengine.org/examples/picking/mesh-picking/)
//! originally written by [Joona Aalto](https://github.com/Jondolf) of
//! [Avian](https://github.com/Jondolf/avian) fame.
//!
use std::f32::consts::PI;

use bevy::{color::palettes::tailwind::*, prelude::*};
use bevy_minibuffer::prelude::*;

#[path = "common/lib.rs"]
mod common;

fn plugin(app: &mut App) {
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        UniversalArgActs::default(),
        TapeActs::default(),
        // unscriptable::set_color,
        Act::new_with_input(set_color),
    ));
}

fn main() {
    App::new()
        .add_plugins((
            common::VideoCapturePlugin::new("tapes")
                .background(Srgba::hex("8ECAE6").unwrap())
                .resolution(Vec2::new(600.0, 400.0)),
            plugin,
            MeshPickingPlugin,
        ))
        .init_resource::<Selected>()
        .init_resource::<Selectables>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (rotate, (update_selected, update_color).chain()))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

#[derive(Resource, Default)]
struct Selected {
    curr: Option<Entity>,
    last: Option<Entity>,
}

impl Selected {
    fn set(&mut self, value: Option<Entity>) {
        self.last = self.curr;
        self.curr = value;
    }
}

#[derive(Resource, Default)]
struct Selectables(Vec<Entity>);

#[derive(Component, Clone, Copy)]
struct Paint {
    base: Color,
    tone: Option<(Color, f32)>,
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
    mut minibuffer: Minibuffer,
) {
    // Set up the materials.
    let ground_color: Color = Srgba::hex("6A994E").unwrap().into();
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

    let mut selectables = vec![];

    let observers = vec![
        Observer::new(update_color_on::<Pointer<Over>>(hover_color)),
        Observer::new(update_color_on::<Pointer<Out>>(None)),
        Observer::new(update_color_on::<Pointer<Down>>(pressed_color)),
        Observer::new(update_color_on::<Pointer<Up>>(hover_color)),
        Observer::new(select),
        Observer::new(rotate_on_drag),
    ];
    // Spawn the shapes. The meshes will be pickable by default.
    for (i, shape) in shapes.into_iter().enumerate() {
        let id = commands
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
            .id();
        selectables.push(id);
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        let id = commands
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
            .id();
        selectables.push(id);
    }
    // Observers
    for mut observer in observers {
        for id in &selectables {
            observer.watch_entity(*id);
        }
        commands.spawn(observer);
    }

    // Selectables
    commands.insert_resource(Selectables(selectables));

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

    // // Instructions
    // commands.spawn((
    //     Text::new("Click a shape. Type ':set_color'."),
    //     Node {
    //         position_type: PositionType::Absolute,
    //         top: Val::Px(12.0),
    //         left: Val::Px(12.0),
    //         ..default()
    //     },
    // ));

    minibuffer.message("Click a shape. Type :set_color.");
    minibuffer.set_visible(true);
}

mod unscriptable {
    pub(crate) use super::*;

    pub(crate) fn set_color(mut minibuffer: Minibuffer, selected: Res<Selected>) {
        if selected.curr.is_none() {
            minibuffer.message("Select a shape first.");
        } else {
            let selection = selected.curr.unwrap();
            minibuffer
                .prompt_map("Hex color: ", bevy_minibuffer::autocomplete::SrgbaHexLookup)
                .observe(
                    move |mut trigger: Trigger<Completed<Srgba>>,
                          mut selected: ResMut<Selected>,
                          mut paints: Query<&mut Paint>,
                          mut commands: Commands,
                          mut minibuffer: Minibuffer,
                          selectables: Res<Selectables>| {
                        if let Completed::Unhandled { result, input: _ } =
                            trigger.event_mut().take()
                        {
                            match result {
                                Ok(color) => {
                                    goto_next_selectable(selection, &selectables, &mut selected);
                                    if let Ok(mut paint) = paints.get_mut(selection) {
                                        minibuffer.message(format!("Set color to {:?}", &color));
                                        paint.base = color.into();
                                        paint.tone = None;
                                    }
                                    commands.entity(trigger.entity()).despawn_recursive();
                                }
                                Err(e) => {
                                    warn!("set_color error: {e}");
                                }
                            }
                        } else {
                            commands.entity(trigger.entity()).despawn_recursive();
                        }
                    },
                );
        }
    }
}

pub(crate) fn set_color(
    In(input): In<Option<Srgba>>,
    mut minibuffer: Minibuffer,
    mut paints: Query<&mut Paint>,
    mut selected: ResMut<Selected>,
    selectables: Res<Selectables>,
) {
    if selected.curr.is_none() {
        minibuffer.message("Select a shape first.");
    } else {
        let selection = selected.curr.unwrap();
        if let Some(color) = input {
            goto_next_selectable(selection, &selectables, &mut selected);
            if let Ok(mut paint) = paints.get_mut(selection) {
                minibuffer.message(format!("{:?}", &color));
                paint.base = color.into();
                paint.tone = None;
            }
        } else {
            minibuffer
                .prompt_map("Hex color: ", bevy_minibuffer::autocomplete::SrgbaHexLookup)
                .observe(
                    move |mut trigger: Trigger<Completed<Srgba>>,
                          mut selected: ResMut<Selected>,
                          mut paints: Query<&mut Paint>,
                          mut commands: Commands,
                          mut minibuffer: Minibuffer,
                          selectables: Res<Selectables>| {
                        if let Completed::Unhandled { result, input: _ } =
                            trigger.event_mut().take()
                        {
                            match result {
                                Ok(color) => {
                                    minibuffer.log_input(&Some(color));
                                    goto_next_selectable(selection, &selectables, &mut selected);
                                    if let Ok(mut paint) = paints.get_mut(selection) {
                                        minibuffer.message(format!("Set color to {:?}", &color));
                                        paint.base = color.into();
                                        paint.tone = None;
                                    }
                                    commands.entity(trigger.entity()).despawn_recursive();
                                }
                                Err(e) => {
                                    warn!("set_color error: {e}");
                                }
                            }
                        } else {
                            commands.entity(trigger.entity()).despawn_recursive();
                        }
                    },
                );
        }
    }
}

fn goto_next_selectable(selection: Entity, selectables: &Selectables, selected: &mut Selected) {
    selected.curr = selectables
        .0
        .iter()
        .position(|x| *x == selection)
        .and_then(|index| selectables.0.get(index + 1))
        .or(selectables.0.first())
        .copied();
}

fn select(trigger: Trigger<Pointer<Click>>, mut selected: ResMut<Selected>) {
    selected.set(Some(trigger.entity()));
}

fn update_selected(selected: Res<Selected>, mut paints: Query<&mut Paint>) {
    let selected_color = Color::from(RED_800);
    if selected.is_changed() {
        if let Some(id) = selected.curr {
            if let Ok(mut paint) = paints.get_mut(id) {
                paint.tone = Some((selected_color, 0.8));
            }
        }
        if let Some(id) = selected.last {
            if let Ok(mut paint) = paints.get_mut(id) {
                paint.tone = None;
            }
        }
    }
}

fn update_color(
    mut query: Query<(&mut MeshMaterial3d<StandardMaterial>, &Paint), Changed<Paint>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut mesh_material, paint) in &mut query {
        if let Some(material) = materials.get_mut(&mut mesh_material.0) {
            material.base_color = match paint.tone {
                Some((tone, k)) => paint.base.mix(&tone, k),
                None => paint.base,
            };
        }
    }
}

/// Returns an observer that updates the entity's material to the one specified.
fn update_color_on<E>(
    color: Option<Color>,
) -> impl Fn(Trigger<E>, Query<&mut Paint>, Res<Selected>) {
    move |trigger, mut query, selected| {
        if selected
            .curr
            .map(|x| x == trigger.entity())
            .unwrap_or(false)
        {
            return;
        }
        if let Ok(mut paint) = query.get_mut(trigger.entity()) {
            paint.tone = color.map(|c| (c, 0.7));
        }
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
