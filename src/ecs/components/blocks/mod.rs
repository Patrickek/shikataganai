use crate::ecs::components::blocks::block_id::BlockId;
use crate::ecs::plugins::rendering::mesh_pipeline::loader::Meshes;
use crate::ecs::resources::block::BlockSprite;
use bevy::prelude::*;
use std::ops::Deref;

pub mod block_id;
pub mod regular_blocks;
pub mod regular_meshes;

pub enum BlockRenderInfo {
  Nothing,
  AsBlock([BlockSprite; 6]),
  AsMesh(Meshes),
}

pub trait BlockTrait {
  fn visible(&self) -> bool {
    true
  }
  fn passable(&self) -> bool {
    false
  }
  fn render_info(&self) -> BlockRenderInfo;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BlockMeta {
  pub v: u32,
}

#[derive(Debug, Component, Copy, Clone)]
pub struct Block {
  pub block: BlockId,
  pub meta: BlockMeta,
  pub entity: Entity,
}

impl Block {
  pub fn new(block: BlockId) -> Self {
    Self {
      block,
      meta: BlockMeta { v: 0 },
      entity: Entity::from_bits(0),
    }
  }
}

impl Deref for Block {
  type Target = dyn BlockTrait;

  fn deref(&self) -> &Self::Target {
    self.block.deref()
  }
}