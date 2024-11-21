use bevy::prelude::*;
use bevy_minibuffer::prelude::*;

pub struct VideoCaptureSettings {
    pub title: String,
}

impl VideoCaptureSettings {
    pub fn window_plugin(&self) -> WindowPlugin {
        WindowPlugin {
            primary_window: Some(Window {
                resolution: [400., 400.].into(),
                title: self.title.clone(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn minibuffer_plugin(&self) -> MinibufferPlugin {
        MinibufferPlugin {
            config: Config {
                // auto_hide: true,
                auto_hide: false,
                hide_delay: Duration::from_millis(5000),
                text_style: TextStyle {
                    font_size: 20.0,
                    ..default()
                },
            },
        }
    }
}

impl Plugin for VideoCaptureSettings {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            DefaultPlugins.set(self.window_plugin()),
            MinibufferPlugins.set(self.minibuffer_plugin()),
        ));
    }
}
