//! Demonstrates how the `AlignItems` and `JustifyContent` properties can be composed to layout text.
use bevy::prelude::*;

const ALIGN_ITEMS_COLOR: Color = Color::rgb(1., 0.066, 0.349);
const JUSTIFY_CONTENT_COLOR: Color = Color::rgb(0.102, 0.522, 1.);
const MARGIN: Val = Val::Px(5.);

#[derive(Component)]
struct Prompt {
    active: bool
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [870., 1066.].into(),
                title: "Bevy Text Layout Example".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_startup_system(spawn_layout)
        .run();
}

fn prompt_update(mut query: Query<&mut Text, With<Prompt>>) {

}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                // Fill the entire window.
                // Does it have to fill the whole window?
                size: Size::all(Val::Percent(100.)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.,
                        padding: UiRect {
                            top: Val::Px(1.),
                            left: Val::Px(1.),
                            right: Val::Px(1.),
                            bottom: Val::Px(1.),
                        },
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                .with_children(|builder| {

                            builder.spawn(TextBundle::from_sections([
                                TextSection::new(
                                    "Prompt: ",
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: 24.0,
                                        color: Color::WHITE,
                                    },
                                ),
                                TextSection::new(
                                    "input",
                                    TextStyle {
                                        font,
                                        font_size: 24.0,
                                        color: Color::WHITE,
                                    },
                                )
                            ]))
                            .insert(Prompt { active: false });
                });

        });
}

