//! UI
use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use std::borrow::Cow;
const PADDING: Val = Val::Px(3.);
const LEFT_PADDING: Val = Val::Px(6.);

/// Root minibuffer node
#[derive(Component)]
struct MinibufferNode;

/// Minibuffer prompt parent
#[derive(Component)]
pub struct PromptContainer;

/// Mode line
#[derive(Component)]
pub struct StatusNode;

/// Autocomplete panel parent
#[derive(Component)]
pub struct CompletionContainer;

/// Autocomplete scrolling state
#[derive(Component, Default)]
pub struct ScrollingList {
    position: f32,
    // selection: Option<usize>,
    // last_selection: Option<usize>,
}

/// Autocomplete list
pub struct CompletionList(pub Vec<Cow<'static, str>>);

/// Autocomplete item
pub(crate) fn completion_item(label: String, style: TextStyle) -> (TextBundle, Label, AccessibilityNode) {
    (
        TextBundle::from_section(label, style),
        Label,
        AccessibilityNode(NodeBuilder::new(Role::ListItem)),
    )
}

/// Create the UI layout.
pub(crate) fn spawn_layout(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            // visibility: Visibility::Hidden,
            style: Style {
                position_type: PositionType::Absolute,
                // top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                right: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,

                // align_items: AlignItems::FlexEnd,
                // justify_content:
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(MinibufferNode {})
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    visibility: Visibility::Hidden,
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|builder| {
                    // List with hidden overflow
                    builder
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::FlexEnd,
                                // height: Val::Percent(50.),
                                min_width: Val::Percent(25.),
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                            background_color: Color::rgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        })
                        .insert(CompletionContainer)
                        .with_children(|builder| {
                            builder
                                .spawn((
                                    NodeBundle {
                                        style: Style {
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
                                        background_color: BackgroundColor(Color::BLACK),
                                        ..default()
                                    },
                                    ScrollingList::default(),
                                    // CompletionList(vec![]),
                                    AccessibilityNode(NodeBuilder::new(Role::List)),
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

                            builder.spawn(NodeBundle::default());
                        });
                });
            builder
                .spawn(NodeBundle {
                    // visibility: Visibility::Hidden,
                    style: Style {
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
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                .insert(PromptContainer {});
        });
}

/// Scroll the auto complete panel with mouse.
fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;

            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.top = Val::Px(scrolling_list.position);
        }
    }
}
