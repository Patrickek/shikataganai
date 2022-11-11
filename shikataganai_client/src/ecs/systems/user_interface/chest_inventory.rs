use crate::ecs::plugins::client::Requested;
use crate::ecs::plugins::imgui::GUITextureAtlas;
use crate::ecs::plugins::rendering::inventory_pipeline::inventory_cache::ExtractedItems;
use crate::ecs::systems::user_interface::render_item_grid;
use crate::ImguiState;
use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use bincode::serialize;
use imgui::Condition;
use shikataganai_common::ecs::components::blocks::ReverseLocation;
use shikataganai_common::ecs::components::functors::InternalInventory;
use shikataganai_common::networking::{ClientChannel, FunctorType, PlayerCommand};

pub struct InventoryOpened(pub Entity);

#[derive(Default)]
pub enum InventoryItemMovementStatus {
  #[default]
  Nothing,
  HoldingItemFrom(usize),
}

pub fn chest_inventory(
  mut commands: Commands,
  imgui: NonSendMut<ImguiState>,
  // window: Res<Windows>,
  inventory_opened: Option<ResMut<InventoryOpened>>,
  texture: Res<GUITextureAtlas>,
  // hotbar_items: Res<PlayerInventory>,
  // selected_hotbar: Res<SelectedHotBar>,
  // inventory_item_movement_status: Local<InventoryItemMovementStatus>,
  mut extracted_items: ResMut<ExtractedItems>,
  inventory_query: Query<&mut InternalInventory>,
  requested_query: Query<&Requested>,
  location_query: Query<&ReverseLocation>,
  mut client: ResMut<RenetClient>,
) {
  if let Some(inventory_entity) = inventory_opened.map(|e| e.0) {
    match inventory_query.get(inventory_entity) {
      Ok(internal_inventory) => {
        let ui = imgui.get_current_frame();
        imgui::Window::new("Chest inventory")
          .position([20.0, 20.0], Condition::Appearing)
          .size([800.0, 600.0], Condition::Appearing)
          .build(ui, || {
            render_item_grid(
              ui,
              (5, 2),
              |x, y| (internal_inventory.inventory[y * 5 + x].as_ref(), x + y),
              texture.as_ref(),
              extracted_items.as_mut(),
            );
          });
      }
      Err(_) => {
        if !requested_query.get(inventory_entity).is_ok() {
          let location = location_query.get(inventory_entity).unwrap();
          client.send_message(
            ClientChannel::ClientCommand.id(),
            serialize(&PlayerCommand::RequestFunctor {
              location: location.0,
              functor: FunctorType::InternalInventory,
            })
            .unwrap(),
          );
          commands.entity(inventory_entity).insert(Requested);
        }
      }
    }
  }
}
