use crate::ecs::components::blocks::{BlockRenderInfo, DerefExt};
use crate::ecs::plugins::camera::{Selection, SelectionRes};
use crate::ecs::plugins::rendering::voxel_pipeline::bind_groups::{
  LightTextureBindGroup, LightTextureHandle, SelectionBindGroup, TextureHandle, VoxelTextureBindGroup,
  VoxelViewBindGroup,
};
use crate::ecs::plugins::rendering::voxel_pipeline::draw_command::DrawVoxelsFull;
use crate::ecs::plugins::rendering::voxel_pipeline::meshing::{ChunkMeshBuffer, RemeshEvent, SingleSide};
use crate::ecs::plugins::rendering::voxel_pipeline::pipeline::VoxelPipeline;
use crate::ecs::plugins::settings::AmbientOcclusion;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{DrawFunctions, RenderPhase};
use bevy::render::render_resource::{BufferUsages, BufferVec, PipelineCache, SpecializedRenderPipelines};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::view::ViewUniforms;
use bevy::render::Extract;
use bevy::utils::hashbrown::HashMap;
use itertools::Itertools;
use shikataganai_common::ecs::components::blocks::Block;
use shikataganai_common::ecs::resources::world::GameWorld;
use shikataganai_common::util::array::{sub_ddd, ArrayIndex, ImmediateNeighbours, DD};
use std::ops::Deref;
use wgpu::util::BufferInitDescriptor;
use wgpu::{BindGroupDescriptor, BindGroupEntry, BindingResource};

#[derive(Resource)]
pub struct ExtractedBlocks {
  pub blocks: HashMap<DD, BufferVec<SingleSide>>,
}

impl Default for ExtractedBlocks {
  fn default() -> Self {
    Self { blocks: HashMap::new() }
  }
}

pub fn extract_chunks(
  mut commands: Commands,
  game_world: Extract<Res<GameWorld>>,
  selection: Extract<Res<SelectionRes>>,
  mut remesh_events: Extract<EventReader<RemeshEvent>>,
  ambient_occlusion: Extract<Res<AmbientOcclusion>>,
  mut extracted_blocks: ResMut<ExtractedBlocks>,
) {
  commands.insert_resource(selection.clone());
  let mut updated: UpdatedVec = UpdatedVec(vec![]);

  for ch in remesh_events
    .iter()
    .filter_map(|p| if let RemeshEvent::Remesh(d) = p { Some(d) } else { None })
    .unique()
  {
    if !game_world.chunks.contains_key(ch) {
      continue;
    }
    updated.push(*ch);
    extracted_blocks
      .blocks
      .insert(*ch, BufferVec::new(BufferUsages::VERTEX));
    let extracted_blocks = extracted_blocks.blocks.get_mut(ch).unwrap();
    let bounds = game_world.chunks[ch].grid.bounds;
    let mut i = bounds.0;
    loop {
      let block: Block = *game_world.get(i).unwrap();
      match block.deref_ext().render_info() {
        BlockRenderInfo::Nothing => {}
        BlockRenderInfo::AsBlock(block_sprites) => {
          if block.visible() {
            for neighbour in i.immediate_neighbours() {
              if game_world.get(neighbour).map_or(true, |b| !b.visible()) {
                let light_level = game_world.get_light_level(neighbour);
                let lighting = match light_level {
                  Some(light_level) => (light_level.heaven, light_level.hearth),
                  None => (0, 0),
                };

                extracted_blocks.push(SingleSide::new(
                  (i.0 as f32, i.1 as f32, i.2 as f32),
                  sub_ddd(neighbour, i),
                  block_sprites,
                  lighting,
                  &game_world,
                  ambient_occlusion.0,
                ));
              }
            }
          }
        }
        BlockRenderInfo::AsMesh(_) => {}
        BlockRenderInfo::AsSkeleton(_) => {}
      }
      i = match i.next(&bounds) {
        None => break,
        Some(i) => i,
      }
    }
  }
  commands.insert_resource(updated);
}

#[derive(Default, Deref, DerefMut, Resource)]
pub struct UpdatedVec(pub Vec<DD>);

pub fn queue_chunks(
  mut commands: Commands,
  mut extracted_blocks: ResMut<ExtractedBlocks>,
  mut views: Query<&mut RenderPhase<Opaque3d>>,
  draw_functions: Res<DrawFunctions<Opaque3d>>,
  mut pipelines: ResMut<SpecializedRenderPipelines<VoxelPipeline>>,
  mut pipeline_cache: ResMut<PipelineCache>,
  chunk_pipeline: Res<VoxelPipeline>,
  (render_device, render_queue): (Res<RenderDevice>, Res<RenderQueue>),
  view_uniforms: Res<ViewUniforms>,
  gpu_images: Res<RenderAssets<Image>>,
  (handle, light_texture_handle): (Res<TextureHandle>, Res<LightTextureHandle>),
  selection: Res<SelectionRes>,
  updated: Res<UpdatedVec>,
) {
  if let Some(gpu_image) = gpu_images.get(&handle.0) {
    commands.insert_resource(VoxelTextureBindGroup {
      bind_group: render_device.create_bind_group(&BindGroupDescriptor {
        entries: &[
          BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&gpu_image.texture_view),
          },
          BindGroupEntry {
            binding: 1,
            resource: BindingResource::Sampler(&gpu_image.sampler),
          },
        ],
        label: Some("block_material_bind_group"),
        layout: &chunk_pipeline.texture_layout,
      }),
    });
  }
  // TODO: only do this once maybe?
  if let Some(gpu_image) = gpu_images.get(&light_texture_handle.0) {
    commands.insert_resource(LightTextureBindGroup {
      bind_group: render_device.create_bind_group(&BindGroupDescriptor {
        entries: &[
          BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&gpu_image.texture_view),
          },
          BindGroupEntry {
            binding: 1,
            resource: BindingResource::Sampler(&gpu_image.sampler),
          },
        ],
        label: Some("light_texture_bind_group"),
        layout: &chunk_pipeline.light_texture_layout,
      }),
    });
  }

  if let Some(view_binding) = view_uniforms.uniforms.binding() {
    commands.insert_resource(VoxelViewBindGroup {
      bind_group: render_device.create_bind_group(&BindGroupDescriptor {
        entries: &[BindGroupEntry {
          binding: 0,
          resource: view_binding,
        }],
        label: Some("block_view_bind_group"),
        layout: &chunk_pipeline.view_layout,
      }),
    });
  }

  let contents = match selection.into_inner().deref() {
    None => [-9999, -9999, -9999, 0, -9999, -9999, -9999, 0],
    Some(Selection { cube, face }) => [cube.0, cube.1, cube.2, 0, face.0, face.1, face.2, 0],
  };
  let selection_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
    label: Some("selection_buffer"),
    contents: bytemuck::bytes_of(&contents),
    usage: BufferUsages::UNIFORM,
  });

  commands.insert_resource(SelectionBindGroup {
    bind_group: render_device.create_bind_group(&BindGroupDescriptor {
      entries: &[BindGroupEntry {
        binding: 0,
        resource: BindingResource::Buffer(selection_buffer.as_entire_buffer_binding()),
      }],
      label: Some("block_view_bind_group"),
      layout: &chunk_pipeline.selection_layout,
    }),
  });

  let draw_function = draw_functions.read().get_id::<DrawVoxelsFull>().unwrap();

  let pipeline = pipelines.specialize(&mut pipeline_cache, &chunk_pipeline, ());

  let buf = &mut extracted_blocks.blocks;
  for i in updated.iter() {
    let buf = buf.get_mut(i).unwrap();
    buf.write_buffer(&render_device, &render_queue);
  }
  for (_, buf) in buf.iter_mut() {
    if !buf.is_empty() {
      let entity = commands
        .spawn(ChunkMeshBuffer(buf.buffer().unwrap().clone(), buf.len()))
        .id();
      for mut view in views.iter_mut() {
        view.add(Opaque3d {
          distance: 2.0,
          draw_function,
          pipeline,
          entity,
        });
      }
    }
  }
}
