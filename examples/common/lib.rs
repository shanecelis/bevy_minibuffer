use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        RenderPlugin,
    },
};
use bevy_minibuffer::prelude::*;
#[cfg(feature = "dev-capture")]
use bevy_image_export::{ImageExportBundle, ImageExportPlugin, ImageExportSource, ImageExportSettings};
// use std::collections::hash_map::DefaultHasher;
// use std::hash::{Hash, Hasher};
// use rand_chacha::ChaCha8Rng;
// use rand::{Rng, SeedableRng};

pub struct VideoCapturePlugin {
    pub resolution: Vec2,
    pub title: String,
    pub background: Option<Color>,
}

impl Plugin for VideoCapturePlugin {
    fn build(&self, app: &mut bevy::app::App) {

        let res = self.resolution;
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

// impl PluginGroup for VideoCapturePlugin {
//     fn build(self) -> PluginGroupBuilder {
//         PluginGroupBuilder::start::<Self>()
//             .add(self.export_plugin.expect("export plugin called with start"))
//             .add_group(DefaultPlugins.set(self.window_plugin()))
//             .add_group(MinibufferPlugins.set(self.minibuffer_plugin()))
//     }
// }
// fn hash<T: Hash>(obj: &T) -> u64 {
//     let mut hasher = DefaultHasher::new();
//     obj.hash(&mut hasher);
//     hasher.finish()
// }


impl VideoCapturePlugin {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        // let h = hash(&title);
        // let mut rng = ChaCha8Rng::seed_from_u64(h);
        // let value: f32 = rng.gen_range(0.5..=1.0);
        // let hue: f32 = rng.gen_range(0.0..=1.0);
        // // let hue: f32 = h as f32 / u64::MAX as f32;
        // // let value: f32 = ((h >> 1) as f32 / u64::MAX as f32) / 2.0 + 0.5;
        // eprintln!("Using hash {h} to make hue {hue} and value {value}");
        // let background = Hsva::new(hue, 0.9, value, 1.0).into();
        Self {
            title,
            resolution: Vec2::new(400.0, 300.0),
            background: None
            // export_plugin: None,
        }
    }

    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = Some(color.into());
        self
    }

    pub fn resolution(mut self, res: Vec2) -> Self {
        self.resolution = res;
        self
    }

    // pub fn start(&mut self) -> ExportThreads {
    //     let plugin = ImageExportPlugin::default();
    //     let export_threads = plugin.threads.clone();
    //     self.export_plugin = Some(plugin);
    //     export_threads
    // }

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

// fn frame_duration(fps: u32) -> Duration {
//     Duration::from_millis(((1.0 / fps as f32) * 1000.0) as u64)
// }
//
//

fn set_background(
    In(background): In<Color>,
    mut cameras: Query<&mut Camera>,
    mut commands: Commands,
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
                // .insert(IsDefaultUiCamera)
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
