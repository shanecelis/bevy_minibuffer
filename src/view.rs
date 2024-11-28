use bevy::{
    ecs::{query::QueryEntityError, system::SystemParam},
    prelude::*,
};
use bevy_asky::{construct::*, prelude::*, string_cursor::*};

#[derive(Component, Reflect, Default)]
pub struct View;

#[derive(Debug, Reflect)]
#[repr(u8)]
enum ViewPart {
    Focus = 0,
    PreQuestion = 1,
    Question = 2,
    Answer = 3,
    Options = 4,
    Feedback = 5,
}

#[derive(Debug, Component, Reflect)]
pub struct Cursor;

#[derive(Resource, Deref, DerefMut, Reflect)]
pub struct CursorBlink(pub Timer);

impl Construct for View {
    type Props = ();

    fn construct(
        context: &mut ConstructContext,
        _props: Self::Props,
    ) -> Result<Self, ConstructError> {
        if let Some(mut eref) = context.world.get_entity_mut(context.id) {
            if !eref.contains::<Node>() {
                eref.insert(NodeBundle {
                    style: Style {
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    ..default()
                });
            }
        }
        Ok(View)
    }
}

#[derive(SystemParam)]
pub(crate) struct Inserter<'w, 's, C: Component> {
    roots: Query<'w, 's, &'static mut C>,
    children: Query<'w, 's, &'static Children>,
    commands: Commands<'w, 's>,
}

impl<'w, 's, C: Component> Inserter<'w, 's, C> {
    fn insert_or_get_child(
        &mut self,
        root: Entity,
        index: usize,
    ) -> Result<Entity, Option<Entity>> {
        match self.children.get(root) {
            Ok(children) => {
                if index < children.len() {
                    Ok(children[index])
                } else {
                    let mut id = None;
                    if let Some(mut ecommands) = self.commands.get_entity(root) {
                        ecommands.with_children(|parent| {
                            for _ in children.len()..index {
                                parent.spawn(TextBundle::default());
                            }
                            id = Some(parent.spawn(TextBundle::default()).id());
                        });
                    }
                    Err(id)
                }
            }
            _ => {
                let mut id = None;
                if let Some(mut ecommands) = self.commands.get_entity(root) {
                    ecommands.with_children(|parent| {
                        for _ in 0..index {
                            parent.spawn(TextBundle::default());
                        }
                        id = Some(parent.spawn(TextBundle::default()).id());
                    });
                }
                Err(id)
            }
        }
    }

    fn insert_or_get_mut<F>(
        &mut self,
        root: Entity,
        index: usize,
        apply: F,
    ) -> Result<(), QueryEntityError>
    where
        F: Fn(&mut C),
        C: Default,
    {
        match self.children.get(root) {
            Ok(children) => {
                if index < children.len() {
                    self.roots
                        .get_mut(children[index])
                        .map(|mut t: Mut<C>| apply(&mut t))
                } else {
                    // dbg!(index, children.len());
                    if let Some(mut ecommands) = self.commands.get_entity(root) {
                        ecommands.with_children(|parent| {
                            for _ in children.len()..index {
                                parent.spawn(TextBundle::default());
                            }
                            let mut text = C::default();
                            apply(&mut text);
                            parent.spawn(TextBundle::default()).insert(text);
                        });
                    }
                    Ok(())
                }
            }
            _ => {
                if let Some(mut ecommands) = self.commands.get_entity(root) {
                    ecommands.with_children(|parent| {
                        for _ in 0..index {
                            parent.spawn(TextBundle::default());
                        }
                        let mut text = C::default();
                        apply(&mut text);
                        parent.spawn(TextBundle::default()).insert(text);
                    });
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Resource, Component, Reflect)]
pub struct Palette {
    pub text_color: Srgba,
    pub background: Option<Srgba>,
    pub highlight: Srgba,
    pub complete: Srgba,
    pub answer: Srgba,
    pub lowlight: Srgba,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            text_color: Srgba::WHITE,
            background: None,
            highlight: Srgba::hex("80ADFA").unwrap(),
            complete: Srgba::hex("94DD8D").unwrap(),
            answer: Srgba::hex("FFB9E8").unwrap(),
            lowlight: Srgba::hex("5A607A").unwrap(),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.register_type::<View>()
        .register_type::<ViewPart>()
        .register_type::<Cursor>()
        .register_type::<CursorBlink>()
        .register_type::<Palette>()
        .add_systems(
            Update,
            (
                (
                    // focus_view,
                    group_view,
                    radio_view,
                    checkbox_view,
                    prompt_view,
                    text_view::<Without<Password>>,
                    password_view,
                    confirm_view,
                    toggle_view,
                    feedback_view,
                )
                    .chain(),
                blink_cursor,
            )
                .in_set(crate::plugin::MinibufferSet::Output),
        )
        .add_systems(
            Update,
            (
                clear_feedback::<StringCursor>,
                clear_feedback::<Toggle>,
                clear_feedback::<Radio>,
            )
                .in_set(crate::plugin::MinibufferSet::Process),
        )
        // .add_systems(PostUpdate, replace_view)
        .insert_resource(CursorBlink(Timer::from_seconds(
            1.0 / 3.0,
            TimerMode::Repeating,
        )))
        .insert_resource(Palette::default());
}

pub(crate) fn prompt_view(
    mut query: Query<(Entity, &Prompt), Or<(Changed<View>, Changed<Prompt>)>>,
    mut writer: Inserter<Text>,
) {
    for (id, prompt) in query.iter_mut() {
        writer
            .insert_or_get_mut(id, ViewPart::Question as usize, |text| {
                replace_or_insert(text, 0, prompt);
            })
            .expect("prompt");
    }
}

pub(crate) fn feedback_view(
    mut query: Query<(Entity, &Feedback), Or<(Changed<View>, Changed<Feedback>)>>,
    mut writer: Inserter<Text>,
) {
    for (id, feedback) in query.iter_mut() {
        writer
            .insert_or_get_mut(id, ViewPart::Feedback as usize, |text| {
                replace_or_insert(text, 0, &format!(" {}", feedback.message));
            })
            .expect("feedback");
    }
}

pub(crate) fn clear_feedback<T: Component>(
    mut query: Query<&mut Feedback, Or<(Changed<View>, Changed<T>)>>,
) {
    for mut feedback in query.iter_mut() {
        feedback.clear();
    }
}

pub(crate) fn focus_view(
    focus: Focus,
    query: Query<Entity, Or<(Changed<View>, Changed<Focusable>)>>,
    writer: Inserter<Text>,
    palette: Res<Palette>,
) {
    // for id in query.iter_mut() {
    //     writer
    //         .insert_or_get_mut(id, ViewPart::Focus as usize, |text| {
    //             replace_or_insert(text, 0, if focus.is_focused(id) { "> " } else { "  " });
    //             text.sections[0].style.color = palette.highlight.into();
    //         })
    //         .expect("focus");
    // }
}

pub fn text_view<F: bevy::ecs::query::QueryFilter>(
    query: Query<
        (Entity, &StringCursor, &Children, Option<&Placeholder>),
        (
            F,
            // Without<Password>,
            Or<(Changed<View>, Changed<StringCursor>, Changed<Focusable>)>,
        ),
    >,
    mut texts: Query<&mut Text>, //, &mut BackgroundColor)>,
    sections: Query<&Children>,
    palette: Res<Palette>,
    mut commands: Commands,
    focus: Focus,
) {
    for (root, text_state, children, placeholder) in query.iter() {
        let index = ViewPart::Answer as usize;
        let id = if index < children.len() {
            children[index]
        } else {
            let mut new_node = None;
            commands.entity(root).with_children(|parent| {
                for _ in children.len()..index {
                    parent.spawn(TextBundle::default());
                }
                new_node = Some(parent.spawn(TextBundle::default()).id());
            });
            new_node.unwrap()
        };
        if let Ok(cursor_parts) = sections.get(id) {
            // Update the parts.
            let mut parts = texts.iter_many_mut(cursor_parts);
            if focus.is_focused(root) {
                let mut pre_cursor = parts.fetch_next().expect("pre cursor");
                replace_or_insert(&mut pre_cursor, 0, &text_state.value[0..text_state.index]);
                let mut cursor = parts.fetch_next().expect("cursor");
                replace_or_insert(
                    &mut cursor,
                    0,
                    if text_state.index >= text_state.value.len() {
                        " "
                    } else {
                        &text_state.value[text_state.index..text_state.next_index()]
                    },
                );
                let mut post_cursor = parts.fetch_next().expect("post cursor");
                if text_state.value.is_empty() && placeholder.is_some() {
                    // Use placeholder.
                    replace_or_insert(&mut post_cursor, 0, &placeholder.unwrap().0);
                    post_cursor.sections[0].style.color = palette.lowlight.into();
                } else {
                    replace_or_insert(
                        &mut post_cursor,
                        0,
                        &text_state.value[text_state.next_index()..],
                    );
                    post_cursor.sections[0].style.color = palette.text_color.into();
                }
            } else {
                let mut pre_cursor = parts.fetch_next().expect("pre cursor");
                replace_or_insert(&mut pre_cursor, 0, &text_state.value);
                let mut cursor = parts.fetch_next().expect("cursor");
                replace_or_insert(&mut cursor, 0, "");
                let mut post_cursor = parts.fetch_next().expect("post cursor");
                replace_or_insert(&mut post_cursor, 0, "");
            }
        } else {
            // Make the parts.
            commands.entity(id).with_children(|parent| {
                // pre cursor
                parent.spawn(TextBundle::from_section(
                    &text_state.value[text_state.next_index()..],
                    TextStyle::default(),
                ));
                // cursor
                parent
                    .spawn(TextBundle::from_section(
                        if text_state.index >= text_state.value.len() {
                            " "
                        } else {
                            &text_state.value[text_state.index..text_state.next_index()]
                        },
                        TextStyle::default(),
                    ))
                    .insert(Cursor);
                // post cursor
                match placeholder {
                    Some(placeholder) if text_state.value.is_empty() => {
                        parent.spawn(TextBundle::from_section(
                            placeholder.0.clone(),
                            TextStyle {
                                color: palette.lowlight.into(),
                                ..default()
                            },
                        ));
                    }
                    _ => {
                        parent.spawn(TextBundle::from_section(
                            &text_state.value[0..text_state.index],
                            TextStyle::default(),
                        ));
                    }
                }
            });
        }
    }
}

pub(crate) fn password_view(
    mut query: Query<
        (Entity, &StringCursor, &Children, Option<&Placeholder>),
        (
            With<Password>,
            Or<(Changed<View>, Changed<StringCursor>, Changed<Focusable>)>,
        ),
    >,
    mut texts: Query<&mut Text>, //, &mut BackgroundColor)>,
    sections: Query<&Children>,
    // palette: Res<Palette>,
    mut commands: Commands,
    focus: Focus,
) {
    for (root, text_state, children, _placeholder) in query.iter_mut() {
        let glyph = "*";
        let index = ViewPart::Answer as usize;
        let id = if index < children.len() {
            children[index]
        } else {
            let mut new_node = None;
            commands.entity(root).with_children(|parent| {
                for _ in children.len()..index {
                    parent.spawn(TextBundle::default());
                }
                new_node = Some(parent.spawn(TextBundle::default()).id());
            });
            new_node.unwrap()
        };
        if let Ok(cursor_parts) = sections.get(id) {
            let mut parts = texts.iter_many_mut(cursor_parts);
            if focus.is_focused(root) {
                let mut pre_cursor = parts.fetch_next().expect("pre cursor");
                replace_or_insert_rep(&mut pre_cursor, 0, glyph, text_state.index);
                let mut cursor = parts.fetch_next().expect("cursor");
                replace_or_insert_rep(
                    &mut cursor,
                    0,
                    if text_state.index >= text_state.value.len() {
                        " "
                    } else {
                        glyph
                    },
                    1,
                );
                let mut post_cursor = parts.fetch_next().expect("post cursor");
                replace_or_insert_rep(
                    &mut post_cursor,
                    0,
                    glyph,
                    text_state.value.len().saturating_sub(text_state.index + 1),
                );
            } else {
                let mut pre_cursor = parts.fetch_next().expect("pre cursor");
                replace_or_insert_rep(&mut pre_cursor, 0, glyph, text_state.value.len());
                let mut cursor = parts.fetch_next().expect("cursor");
                replace_or_insert(&mut cursor, 0, "");
                let mut post_cursor = parts.fetch_next().expect("post cursor");
                replace_or_insert(&mut post_cursor, 0, "");
            }
        } else {
            // Make the parts.
            commands.entity(id).with_children(|parent| {
                // pre cursor
                parent.spawn(TextBundle::from_section(
                    glyph.repeat(text_state.index),
                    TextStyle::default(),
                ));
                // cursor
                parent
                    .spawn(TextBundle::from_section(
                        if text_state.index >= text_state.value.len() {
                            " "
                        } else {
                            glyph
                        },
                        TextStyle::default(),
                    ))
                    .insert(Cursor);
                // post cursor
                parent.spawn(TextBundle::from_section(
                    glyph.repeat(text_state.value.len().saturating_sub(text_state.index)),
                    TextStyle::default(),
                ));
            });
        }
    }
}

pub(crate) fn toggle_view(
    mut query: Query<(Entity, &Toggle), Or<(Changed<View>, Changed<Focusable>, Changed<Toggle>)>>,
    palette: Res<Palette>,
    mut commands: Commands,
    mut writer: Inserter<BackgroundColor>,
) {
    // TODO: Shouldn't this just show the answer when it is not in focus?
    for (root, toggle) in query.iter_mut() {
        match writer.insert_or_get_child(root, ViewPart::Options as usize) {
            Ok(options) => {
                writer
                    .insert_or_get_mut(options, 1, |color| {
                        *color = if toggle.index == 0 {
                            palette.highlight.into()
                        } else {
                            palette.lowlight.into()
                        };
                    })
                    .expect("option 0");

                writer
                    .insert_or_get_mut(options, 3, |color| {
                        *color = if toggle.index == 1 {
                            palette.highlight.into()
                        } else {
                            palette.lowlight.into()
                        };
                    })
                    .expect("option 1");
            }
            Err(Some(new)) => {
                commands.entity(new).with_children(|parent| {
                    let style = TextStyle::default();
                    parent.spawn(TextBundle::from_section(" ", style.clone())); // 0
                    parent.spawn(
                        TextBundle::from_section(format!(" {} ", toggle.options[0]), style.clone())
                            .with_background_color(if toggle.index == 0 {
                                palette.highlight.into()
                            } else {
                                palette.lowlight.into()
                            }),
                    ); // 1
                    parent.spawn(TextBundle::from_section(" ", style.clone())); // 2
                    parent.spawn(
                        TextBundle::from_section(format!(" {} ", toggle.options[1]), style) // 3
                            .with_background_color(if toggle.index == 1 {
                                palette.highlight.into()
                            } else {
                                palette.lowlight.into()
                            }),
                    );
                });
            }
            _ => (),
        };
    }
}

pub(crate) fn confirm_view(
    mut query: Query<(Entity, &Confirm), Or<(Changed<View>, Changed<Focusable>, Changed<Confirm>)>>,
    palette: Res<Palette>,
    mut commands: Commands,
    mut writer: Inserter<BackgroundColor>,
) {
    for (root, confirm) in query.iter_mut() {
        match writer.insert_or_get_child(root, ViewPart::Options as usize) {
            Ok(options) => {
                writer
                    .insert_or_get_mut(options, 1, |color| {
                        *color = if !confirm.yes {
                            palette.highlight.into()
                        } else {
                            palette.lowlight.into()
                        };
                    })
                    .expect("option 0");

                writer
                    .insert_or_get_mut(options, 3, |color| {
                        *color = if confirm.yes {
                            palette.highlight.into()
                        } else {
                            palette.lowlight.into()
                        };
                    })
                    .expect("option 1");
            }
            Err(Some(new)) => {
                commands.entity(new).with_children(|parent| {
                    let style = TextStyle::default();
                    parent.spawn(TextBundle::from_section(" ", style.clone())); // 0
                    parent.spawn(
                        TextBundle::from_section(" No ", style.clone()).with_background_color(
                            if !confirm.yes {
                                palette.highlight.into()
                            } else {
                                palette.lowlight.into()
                            },
                        ),
                    ); // 1
                    parent.spawn(TextBundle::from_section(" ", style.clone())); // 2
                    parent.spawn(
                        TextBundle::from_section(" Yes ", style) // 3
                            .with_background_color(if confirm.yes {
                                palette.highlight.into()
                            } else {
                                palette.lowlight.into()
                            }),
                    );
                });
            }
            _ => (),
        };
    }
}

/// Use a column layout for the group views.
pub(crate) fn group_view(
    query: Query<Entity, (With<View>, Or<(Added<RadioGroup>, Added<CheckboxGroup>)>)>,
    mut commands: Commands,
) {
    for id in &query {
        commands.entity(id).column();
    }
}

pub(crate) fn checkbox_view(
    mut query: Query<
        (Entity, &Checkbox),
        Or<(Changed<View>, Changed<Checkbox>, Changed<Focusable>)>,
    >,
    palette: Res<Palette>,
    mut writer: Inserter<Text>,
    focus: Focus,
) {
    for (id, checkbox) in query.iter_mut() {
        writer
            .insert_or_get_mut(id, ViewPart::PreQuestion as usize, |text| {
                replace_or_insert(text, 0, if checkbox.checked { "[x] " } else { "[ ] " });
                // text.sections[0].style.color = if focusable.state() == FocusState::Focused {
                text.sections[0].style.color = if focus.is_focused(id) {
                    palette.highlight.into()
                } else {
                    palette.text_color.into()
                };
            })
            .expect("prequestion");
    }
}

pub(crate) fn radio_view(
    mut query: Query<(Entity, &Radio), Or<(Changed<View>, Changed<Radio>, Changed<Focusable>)>>,
    palette: Res<Palette>,
    mut writer: Inserter<Text>,
    focus: Focus,
) {
    for (id, radio) in query.iter_mut() {
        writer
            .insert_or_get_mut(id, ViewPart::PreQuestion as usize, |text| {
                replace_or_insert(text, 0, if radio.checked { "(x) " } else { "( ) " });
                text.sections[0].style.color = if focus.is_focused(id) {
                    palette.highlight.into()
                } else {
                    palette.text_color.into()
                };
            })
            .expect("prequestion");
    }
}

fn blink_cursor(
    mut query: Query<(Entity, &mut BackgroundColor, &mut Text), With<Cursor>>,
    mut timer: ResMut<CursorBlink>,
    time: Res<Time>,
    mut count: Local<u8>,
    focus: Focus,
    palette: Res<Palette>,
    parent: Query<&Parent>,
) {
    if timer.tick(time.delta()).just_finished() {
        *count = count.checked_add(1).unwrap_or(0);
        for (root, mut color, mut text) in &mut query {
            if focus.is_focused(root) || parent.iter_ancestors(root).any(|id| focus.is_focused(id))
            {
                color.0 = if *count % 2 == 0 {
                    Color::WHITE
                } else {
                    Color::NONE
                };
                text.sections[0].style.color = if *count % 2 == 0 {
                    Color::BLACK
                } else {
                    palette.text_color.into()
                };
            }
        }
    }
}
