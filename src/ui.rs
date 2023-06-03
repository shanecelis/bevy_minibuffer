use crate::prompt::*;
use std::borrow::Cow;
use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

// const MARGIN: Val = Val::Px(5.);
const PADDING: Val = Val::Px(3.);
const LEFT_PADDING: Val = Val::Px(6.);

#[derive(Component)]
pub struct PromptContainer;
#[derive(Component)]
pub struct PromptNode;
#[derive(Component)]
pub struct StatusNode;
#[derive(Component)]
pub struct CompletionContainer;

#[derive(Component, Default)]
pub struct ScrollingList {
    position: f32,
    // selection: Option<usize>,
    // last_selection: Option<usize>,
}

pub struct CompletionList(pub Vec<Cow<'static, str>>);

pub fn completion_item(
    label: String,
    color: Color,
    font: Handle<Font>,
) -> (TextBundle, Label, AccessibilityNode) {
    (
        TextBundle::from_section(
            label,
            TextStyle {
                font: font,
                font_size: 20.,
                color,
            },
        ),
        Label,
        AccessibilityNode(NodeBuilder::new(Role::ListItem)),
    )
}

pub fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());

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
        .insert(PromptContainer {})
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
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
                        .insert(CompletionContainer {})
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
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..30 {
                                        parent.spawn(completion_item(
                                            format!("Item {i}"),
                                            Color::WHITE,
                                            font.clone(),
                                        ));
                                    }
                                });

                            builder.spawn(NodeBundle { ..default() });
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
                // .insert(PromptContainer {})
                .with_children(|builder| {
                    builder
                        .spawn(TextBundle::from_sections([
                            TextSection::new(
                                "PromptNode: ",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "input",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    color: Color::GRAY,
                                },
                            ),
                            TextSection::new(
                                " message",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    color: Color::YELLOW,
                                },
                            ),
                            // This is a dummy section to keep the line height stable.
                            TextSection::new(
                                " ",
                                TextStyle {
                                    font,
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                        ]))
                        .insert(PromptNode {});
                });
        });
}

pub fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
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

pub struct TextPrompt<'a, 'w, 's> {
    pub text: &'a mut Text,
    pub completion: Entity,
    pub children: &'a [Entity],
    pub commands: &'a mut Commands<'w, 's>,
    pub font: Handle<Font>,
}

#[allow(dead_code)]
impl<'a, 'w, 's> TextPrompt<'a, 'w, 's> {
    pub fn prompt_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[0].value
    }
    pub fn prompt_get(&self) -> &str {
        &self.text.sections[0].value
    }
    pub fn input_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[1].value
    }
    pub fn input_get(&self) -> &str {
        &self.text.sections[1].value
    }
    pub fn message_get_mut(&mut self) -> &mut String {
        &mut self.text.sections[2].value
    }
    pub fn message_get(&self) -> &str {
        &self.text.sections[2].value
    }
}

impl<'a, 'w, 's> NanoPrompt for TextPrompt<'a, 'w, 's> {
    fn buf_read(&self, buf: &mut PromptBuf) {
        buf.prompt.clone_from(&self.text.sections[0].value);
        buf.input.clone_from(&self.text.sections[1].value);
        buf.message.clone_from(&self.text.sections[2].value);
    }
    fn buf_write(&mut self, buf: &mut PromptBuf) {
        self.text.sections[0].value.clone_from(&buf.prompt);
        self.text.sections[1].value.clone_from(&buf.input);
        self.text.sections[2].value.clone_from(&buf.message);
        if buf.completion.changed() {
            let new_children = (*buf.completion)
                .iter()
                .map(|label| {
                    self.commands
                        .spawn(completion_item(label.into(), Color::WHITE, self.font.clone()))
                        .id()
                })
                .collect::<Vec<Entity>>();

            self.commands
                .entity(self.completion)
                .replace_children(&new_children);
            for child in self.children.iter() {
                self.commands.entity(*child).despawn();
            }
            buf.completion.reset();
        }
    }
    async fn read_raw(&mut self) -> Result<PromptBuf, NanoError> {
        panic!("Not sure this should ever be called.");
    }
}
