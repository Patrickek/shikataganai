use crate::ecs::components::blocks::BlockRenderInfo;
use crate::ecs::components::blocks::DerefExt;
use crate::ecs::plugins::rendering::mesh_pipeline::loader::GltfMeshStorageHandle;
use crate::ecs::plugins::rendering::mesh_pipeline::systems::MeshMarker;
use crate::ecs::plugins::rendering::voxel_pipeline::meshing::RemeshEvent;
use crate::GltfMeshStorage;
use bevy::prelude::*;
use itertools::Itertools;
use num_traits::FloatConst;
use shikataganai_common::ecs::resources::world::GameWorld;
use shikataganai_common::util::array::{from_ddd, ArrayIndex};

pub fn remesh_system_auxiliary(
  mut commands: Commands,
  mesh_query: Query<&Handle<Mesh>>,
  mut transform_query: Query<&mut Transform>,
  mut game_world: ResMut<GameWorld>,
  mut remesh_events: EventReader<RemeshEvent>,
  storage: Res<GltfMeshStorageHandle>,
  mesh_storage_assets: Res<Assets<GltfMeshStorage>>,
) {
  for ch in remesh_events
    .iter()
    .filter_map(|p| if let RemeshEvent::Remesh(d) = p { Some(d) } else { None })
    .unique()
  {
    if !game_world.chunks.contains_key(ch) {
      continue;
    }
    let bounds = game_world.chunks.get(ch).unwrap().grid.bounds;
    let mut i = bounds.0;
    loop {
      let mut block = game_world.get_mut(i).unwrap();
      match block.deref_ext().render_info() {
        BlockRenderInfo::AsMesh(mesh) => {
          if block.entity == Entity::from_bits(0) {
            if let Some(mesh_assets_hash_map) = mesh_storage_assets.get(&storage.0) {
              let mesh = &mesh_assets_hash_map[&mesh];
              let render_mesh: &Handle<Mesh> = mesh.render.as_ref().unwrap();
              let rotation = block.meta.get_rotation();
              let e = commands
                .spawn()
                .insert(render_mesh.clone())
                .insert(MeshMarker)
                .insert(
                  Transform::from_translation(from_ddd(i) + Vec3::new(0.5, 0.5, 0.5))
                    .with_rotation(Quat::from_rotation_y(f32::PI() / 2.0 * rotation as i32 as f32)),
                )
                .insert(GlobalTransform::default())
                .id();
              block.entity = e;
            }
          } else {
            if mesh_query.get(block.entity).is_ok() {
              transform_query.get_mut(block.entity).unwrap().translation = from_ddd(i) + Vec3::new(0.5, 0.5, 0.5);
            } else {
              if let Some(mesh_assets_hash_map) = mesh_storage_assets.get(&storage.0) {
                let mesh = &mesh_assets_hash_map[&mesh];
                let render_mesh = mesh.render.as_ref().unwrap();
                commands
                  .entity(block.entity)
                  .insert(MeshMarker)
                  .insert(render_mesh.clone())
                  .insert(Transform::from_translation(from_ddd(i) + Vec3::new(0.5, 0.5, 0.5)))
                  .insert(GlobalTransform::default());
              }
            }
          }
        }
        _ => {}
      }
      i = match i.next(&bounds) {
        None => break,
        Some(i) => i,
      }
    }
  }
}
