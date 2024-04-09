use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

pub struct VideoCaptureSettings {
    pub title: String,
}

impl bevy::app::Plugin for VideoCaptureSettings {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: self.title.clone(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(MinibufferPlugin {
            config: Config {
                auto_hide: true,
                // auto_hide: false,
                hide_delay: Some(3000),
                text_style: TextStyle {
                    font_size: 20.0,
                    ..default()
                },
            },
        });
    }
}
