use crate::ecs::components::blocks::Block;
use crate::util::array::DDD;
use bevy::prelude::*;
use bevy_renet::renet::{
  BlockChannelConfig, ChannelConfig, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// -------------------------------------------------------------------------------------------
// -- ###  #    #   ###   ####   #####  #     #  #####  #    #  #####        #     #  ##### --
// --  #   #    #  #   #  #   #  #      ##   ##  #      #    #    #          ##   ##  #     --
// --  #   ##   #  #      #   #  #      # # # #  #      ##   #    #          # # # #  #     --
// --  #   # #  #  #      #   #  #      #  #  #  #      # #  #    #          #  #  #  #     --
// --  #   #  # #  #      ####   ####   #     #  ####   #  # #    #          #     #  ####  --
// --  #   #   ##  #      ##     #      #     #  #      #   ##    #          #     #  #     --
// --  #   #    #  #      # #    #      #     #  #      #    #    #          #     #  #     --
// --  #   #    #  #   #  #  #   #      #     #  #      #    #    #          #     #  #     --
// -- ###  #    #   ###   #   #  #####  #     #  #####  #    #    #          #     #  ##### --
// -------------------------------------------------------------------------------------------
pub const PROTOCOL_ID: u64 = 42;

pub enum ServerChannel {
  GameEvent,
  GameFrame,
  ChunkTransfer,
}

impl ServerChannel {
  pub fn id(&self) -> u8 {
    match self {
      Self::GameEvent => 0,
      Self::GameFrame => 1,
      Self::ChunkTransfer => 2,
    }
  }

  pub fn channels_config() -> Vec<ChannelConfig> {
    vec![
      ReliableChannelConfig {
        channel_id: Self::GameEvent.id(),
        message_resend_time: Duration::ZERO,
        // message_resend_time: Duration::from_millis(200),
        ..Default::default()
      }
      .into(),
      UnreliableChannelConfig {
        channel_id: Self::GameFrame.id(),
        ..Default::default()
      }
      .into(),
      BlockChannelConfig {
        channel_id: Self::ChunkTransfer.id(),
        ..Default::default()
      }
      .into(),
    ]
  }
}

#[derive(Component, Debug, Deserialize, Serialize, Clone)]
pub struct PolarRotation {
  pub phi: f32,
  pub theta: f32,
}

type TranslationRotation = (Vec3, PolarRotation);

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessage {
  PlayerSpawn {
    entity: Entity,
    id: u64,
    translation: TranslationRotation,
  },
  PlayerDespawn {
    id: u64,
  },
  BlockRemove {
    location: DDD,
  },
  BlockPlace {
    location: DDD,
    block: Block,
  },
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
  pub players: Vec<u64>,
  pub translations: Vec<TranslationRotation>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkFrame {
  pub tick: u32,
  pub entities: NetworkedEntities,
}

pub enum ClientChannel {
  ClientCommand,
}

impl ClientChannel {
  pub fn id(&self) -> u8 {
    match self {
      Self::ClientCommand => 0,
    }
  }

  pub fn channels_config() -> Vec<ChannelConfig> {
    vec![ReliableChannelConfig {
      channel_id: Self::ClientCommand.id(),
      message_resend_time: Duration::ZERO,
      ..Default::default()
    }
    .into()]
  }
}

pub fn server_connection_config() -> RenetConnectionConfig {
  RenetConnectionConfig {
    send_channels_config: ServerChannel::channels_config(),
    receive_channels_config: ClientChannel::channels_config(),
    ..Default::default()
  }
}

pub fn client_connection_config() -> RenetConnectionConfig {
  RenetConnectionConfig {
    send_channels_config: ClientChannel::channels_config(),
    receive_channels_config: ServerChannel::channels_config(),
    ..Default::default()
  }
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum PlayerCommand {
  PlayerMove { translation: TranslationRotation },
  BlockRemove { location: DDD },
  BlockPlace { location: DDD, block: Block },
}