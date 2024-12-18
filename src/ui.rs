//! UI
use bevy::{
    a11y::AccessibilityNode,
    // input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use accesskit::{Node as Accessible, Role};
// use bevy_a11y::AccessibilityNode;

const PADDING: Val = Val::Px(3.);
const LEFT_PADDING: Val = Val::Px(6.);

/// The Minibuffer root entity resource.
#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub(crate) struct MinibufferRoot(pub Entity);

/// Root minibuffer node
#[derive(Component)]
struct MinibufferNode;

/// Minibuffer prompt parent
#[derive(Component)]
pub(crate) struct PromptContainer;

// /// Mode line
// #[derive(Component)]
// pub struct StatusNode;

/// Autocomplete panel parent
#[derive(Component)]
pub(crate) struct CompletionContainer;

/// Autocomplete scrolling state
#[derive(Component, Default)]
pub(crate) struct ScrollingList {
    // position: f32,
    // selection: Option<usize>,
    // last_selection: Option<usize>,
}

/// Autocomplete item
pub(crate) fn completion_item(label: String) -> (Text, Label, AccessibilityNode) {
    (
        Text::new(label),
        Label,
        AccessibilityNode(Accessible::new(Role::ListItem)),
    )
}

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<MinibufferRoot>()
        .add_systems(PreStartup, spawn_layout);
}

/// Create the UI layout.
fn spawn_layout(mut commands: Commands) {
    let root = commands
        .spawn(Node {
            // visibility: Visibility::Hidden,
            position_type: PositionType::Absolute,
            // top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            right: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,

            // align_items: AlignItems::FlexEnd,
            // justify_content:
            ..Default::default()
        })
        .insert(Name::new("minibuffer"))
        .insert(MinibufferNode)
        .with_children(|builder| {
            builder
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .insert(Name::new("completions"))
                .with_children(|builder| {
                    // List with hidden overflow
                    builder
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::FlexEnd,
                                // height: Val::Percent(50.),
                                min_width: Val::Percent(25.),
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                        ))
                        .insert(CompletionContainer)
                        .with_children(|builder| {
                            builder
                                .spawn((
                                    Node {
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::FlexStart,
                                            flex_grow: 0.,
                                            padding: UiRect {
                                                top: PADDING,
                                                left: LEFT_PADDING,
                                                right: PADDING * 2.,
                                                bottom: PADDING,
                                            },
                                            margin: UiRect {
                                                bottom: PADDING,
                                                ..default()
                                            },
                                            ..default()
                                    },
                                    BackgroundColor(Color::BLACK),
                                    ScrollingList::default(),
                                    // CompletionList(vec![]),
                                    AccessibilityNode(Accessible::new(Role::List)),
                                ))
                                // .with_children(|parent| {
                                //     // List items
                                //     for i in 0..30 {
                                //         parent.spawn(completion_item(
                                //             format!("Item {i}"),
                                //             TextStyle::default(),
                                //         ));
                                //     }
                                // })
                                ;

                            builder.spawn(Node::default());
                        });
                });
            builder
                .spawn((
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.,
                        padding: UiRect {
                            top: PADDING,
                            left: LEFT_PADDING,
                            right: PADDING,
                            bottom: PADDING,
                        },
                        ..Default::default()
                    },
                    Visibility::Hidden,
                    BackgroundColor(Color::BLACK),
                ))
                .insert(Name::new("buffer"))
                .insert(PromptContainer);
        })
        .id();
    commands.insert_resource(MinibufferRoot(root));
}

// Scroll the auto complete panel with mouse.
// pub(crate) fn mouse_scroll(
//     mut mouse_wheel_events: EventReader<MouseWheel>,
//     mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
//     query_node: Query<&Node>,
// ) {
//     for mouse_wheel_event in mouse_wheel_events.read() {
//         for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
//             let items_height = list_node.size().y;
//             let container_height = query_node.get(parent.get()).unwrap().size().y;

//             let max_scroll = (items_height - container_height).max(0.);

//             let dy = match mouse_wheel_event.unit {
//                 MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
//                 MouseScrollUnit::Pixel => mouse_wheel_event.y,
//             };

//             scrolling_list.position += dy;
//             scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
//             info!("scrolling position {}", scrolling_list.position);
//             style.top = Val::Px(scrolling_list.position);
//         }
//     }
// }
