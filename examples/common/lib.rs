//! Common example facilities like video capture
//!
//! This provides examples with a few commonalities:
//! - A window title name
//! - A settable window size that defaults to 400 x 400
//! - An optional color background
//!
//! The most important part of this code, however, is it captures the frames of
//! an example when "dev-capture" is named as a feature. This changes several
//! aspects of how the examples are run, namely:
//!
//! - A second camera is attached to the primary camera.
//! - The UI is rendered directly to the second camera.
//!
//! - A node that spans the screen is spawned for the primary camera; it renders
//!   the secondary camera's captured image. (This was the only way I found I
//!   could have the UI displayed in two cameras at once.)
//!
//! - The FPS is set to 12 to reduce the number of PNGs generated during
//!   capture.
//!
//! - An AppExit event, we wait for the threads writing images to finish.
//!
use bevy::prelude::*;
#[cfg(feature = "dev-capture")]
use bevy::{
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
};
#[cfg(feature = "dev-capture")]
use bevy_image_export::{ImageExportBundle, ImageExportPlugin, ImageExportSource, ImageExportSettings};

pub struct VideoCapturePlugin {
    pub resolution: Vec2,
    pub title: String,
    pub background: Option<Color>,
}

impl Plugin for VideoCapturePlugin {
    fn build(&self, app: &mut bevy::app::App) {

        let background = self.background;

        app
            .add_plugins(DefaultPlugins.set(self.window_plugin()));
        #[cfg(not(feature = "dev-capture"))]
        {
            if let Some(background) = background {
                app
                    .add_systems(PostStartup, (move || background).pipe(set_background));
            }
        }
        #[cfg(feature = "dev-capture")]
        {
            let res = self.resolution;
            let fps = 12.0;
            let plugin = ImageExportPlugin::default();
            let export_threads = plugin.threads.clone();
            app
                .add_plugins((
                    bevy_framepace::FramepacePlugin,
                    plugin,
                    // MinibufferPlugins.set(self.minibuffer_plugin()),
                ))
                .insert_resource(bevy_framepace::FramepaceSettings {
                    limiter: bevy_framepace::Limiter::from_framerate(fps)
                })
                .add_systems(Update, move |events: EventReader<AppExit>| {
                    if !events.is_empty() {
                        export_threads.finish();
                    }
                });
            if let Some(background) = background {
                app
                    .add_systems(PostStartup, ((move || res ).pipe(setup_capture),
                                               (move || background).pipe(set_background)).chain());
            } else {
                app
                    .add_systems(PostStartup, (move || res ).pipe(setup_capture));
            }
        }
    }
}

impl VideoCapturePlugin {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            title,
            resolution: Vec2::new(400.0, 300.0),
            background: None
        }
    }

    #[allow(dead_code)]
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = Some(color.into());
        self
    }

    #[allow(dead_code)]
    pub fn resolution(mut self, res: Vec2) -> Self {
        self.resolution = res;
        self
    }

    pub fn window_plugin(&self) -> WindowPlugin {
        WindowPlugin {
            primary_window: Some(Window {
                resolution: [self.resolution.x, self.resolution.y].into(),
                title: self.title.clone(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    // pub fn minibuffer_plugin(&self) -> MinibufferPlugin {
    //     MinibufferPlugin {
    //         config: Config {
    //             // auto_hide: true,
    //             auto_hide: false,
    //             hide_delay: Duration::from_millis(5000),
    //             text_style: TextStyle {
    //                 font_size: 20.0,
    //                 ..default()
    //             },
    //         },
    //     }
    // }
}

fn set_background(
    In(background): In<Color>,
    mut cameras: Query<&mut Camera>,
) {
    for mut camera in &mut cameras {
        camera.clear_color = background.into();
    }
}

#[cfg(feature = "dev-capture")]
fn setup_capture(
    In(res): In<Vec2>,
    camera2d: Query<Entity, With<Camera2d>>,
    camera3d: Query<Entity, With<Camera3d>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
) {
    // Create an output texture.
    let output_texture_handle = {
        let size = Extent3d {
            width: res.x as u32,
            height: res.y as u32,
            ..default()
        };
        let mut export_texture = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        export_texture.resize(size);

        images.add(export_texture)
    };

    if let Ok(id) = camera2d.get_single() {
        commands.entity(id)
        // .insert(IsDefaultUiCamera)
                .with_children(|parent| {
                    parent.spawn((Camera2dBundle {
                        camera: Camera {
                            order: 100,
                            // Connect the output texture to a camera as a RenderTarget.
                            target: RenderTarget::Image(output_texture_handle.clone()),
                            ..default()
                        },
                        ..default()
                    },
                                  IsDefaultUiCamera
                    ));
                });
        commands.spawn((
            ImageBundle {
                image: UiImage {
                    texture: output_texture_handle.clone(),
                    ..default()
                },
                style: Style {
                    // Cover the whole image
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    // flex_direction: FlexDirection::Column,
                    // justify_content: JustifyContent::Center,
                    // align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            TargetCamera(id),
        ));
    } else if let Ok(id) = camera3d.get_single() {
        commands.entity(id)
                .with_children(|parent| {
                    parent
                        .spawn((Camera3dBundle {
                            camera: Camera {
                                // Connect the output texture to a camera as a RenderTarget.
                                target: RenderTarget::Image(output_texture_handle.clone()),
                                ..default()
                            },
                            ..default()
                        },
                                IsDefaultUiCamera
                        ));
                });
        commands.spawn((
            ImageBundle {
                image: UiImage {
                    texture: output_texture_handle.clone(),
                    ..default()
                },
                style: Style {
                    // Cover the whole image
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    // flex_direction: FlexDirection::Column,
                    // justify_content: JustifyContent::Center,
                    // align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            TargetCamera(id),
        ));
    } else {
        panic!("No camera found!");
    }

    // Spawn the ImageExportBundle to initiate the export of the output texture.
    commands.spawn(ImageExportBundle {
        source: export_sources.add(output_texture_handle),
        settings: ImageExportSettings {
            // Frames will be saved to "./out/[#####].png".
            output_dir: "out".into(),
            // Choose "exr" for HDR renders.
            extension: "png".into(),
        },
    });
}
