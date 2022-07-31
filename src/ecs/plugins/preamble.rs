use crate::ecs::plugins::camera::Selection;
use crate::ecs::plugins::settings::{AmbientOcclusion, FullScreen, MouseSensitivity, Resolution, Settings, VSync};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::winit::WinitWindows;
use std::fs::OpenOptions;
use std::io::Write;
use bevy::render::texture::ImageSettings;

pub struct Preamble;

impl Plugin for Preamble {
  fn build(&self, app: &mut App) {
    let resolution = app.world.resource::<Resolution>();
    let vsync = app.world.resource::<VSync>();
    let fullscreen = app.world.resource::<FullScreen>();
    app
      .insert_resource(WindowDescriptor {
        width: resolution.width,
        height: resolution.height,
        resizable: true,
        title: "仕方がない、ね？".to_string(),
        present_mode: vsync.as_present_mode(),
        mode: fullscreen.as_mode(),
        ..default()
      })
      .insert_resource(ImageSettings::default_nearest())
      .insert_resource(Msaa { samples: 1 })
      .init_resource::<Option<Selection>>()
      .add_system_to_stage(CoreStage::Last, exit);
  }
}

fn exit(
  mut events: EventReader<AppExit>,
  w: NonSend<WinitWindows>,
  sensitivity: Res<MouseSensitivity>,
  resolution: Res<Resolution>,
  vsync: Res<VSync>,
  fullscreen: Res<FullScreen>,
  ambient_occlusion: Res<AmbientOcclusion>,
) {
  if events.iter().next().is_some() || w.windows.is_empty() {
    let mut file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open("shikataganai.toml")
      .unwrap();

    let toml = toml::to_string(&Settings {
      sensitivity: sensitivity.0,
      height: resolution.height,
      width: resolution.width,
      vsync: vsync.0,
      fullscreen: fullscreen.0,
      ambient_occlusion: ambient_occlusion.0,
    })
    .unwrap();

    file.write(toml.as_bytes()).unwrap();

    std::process::exit(0)
  }
}
