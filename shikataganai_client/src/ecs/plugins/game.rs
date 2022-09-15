use crate::ecs::components::blocks::{animate, AnimationInstance, AnimationTrait, ChestAnimations, ChestSkeleton, Skeleton};
use crate::ecs::plugins::camera::{Player, Selection};
use crate::ecs::resources::player::{PlayerInventory, SelectedHotBar};
use crate::ecs::resources::world::ClientGameWorld;
use crate::ecs::systems::input::{action_input, hot_bar_scroll_input};
use crate::ecs::systems::light::religh_system;
use crate::ecs::systems::remesh::remesh_system_auxiliary;
use crate::ecs::systems::user_interface::chest_inventory::{chest_inventory, InventoryOpened};
use crate::ecs::systems::user_interface::connecting::connecting_window;
use crate::ecs::systems::user_interface::game_menu::game_menu;
use crate::ecs::systems::user_interface::hot_bar::hot_bar;
use crate::ecs::systems::user_interface::main_menu::main_menu;
use bevy::prelude::*;
use bevy::render::{Extract, RenderApp, RenderStage};
use bevy_rapier3d::plugin::RapierConfiguration;
use bevy_renet::renet::RenetClient;
use bincode::serialize;
use iyes_loopless::prelude::*;
use num_traits::FloatConst;
use shikataganai_common::ecs::resources::player::PlayerNickname;
use shikataganai_common::ecs::resources::world::GameWorld;
use shikataganai_common::networking::{ClientChannel, PlayerCommand};
use std::time::Duration;
use shikataganai_common::ecs::components::blocks::animation::AnimationType;
use shikataganai_common::ecs::components::blocks::ReverseLocation;

pub struct GamePlugin;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShikataganaiGameState {
  MainMenu,
  PreSimulation,
  Simulation,
  Paused,
  InterfaceOpened
}

#[derive(StageLabel)]
pub struct FixedUpdate;

pub fn in_game(current_state: Res<CurrentState<ShikataganaiGameState>>) -> bool {
  match current_state.0 {
    ShikataganaiGameState::Simulation | ShikataganaiGameState::Paused | ShikataganaiGameState::InterfaceOpened => true,
    _ => false,
  }
}

pub fn in_game_extract(current_state: Extract<Res<CurrentState<ShikataganaiGameState>>>) -> bool {
  match current_state.0 {
    ShikataganaiGameState::Simulation | ShikataganaiGameState::Paused | ShikataganaiGameState::InterfaceOpened => true,
    _ => false,
  }
}

pub fn in_game_input_enabled(current_state: Res<CurrentState<ShikataganaiGameState>>) -> bool {
  match current_state.0 {
    ShikataganaiGameState::Simulation | ShikataganaiGameState::Paused => true,
    _ => false,
  }
}

pub fn init_game(mut commands: Commands) {
  commands.init_resource::<SelectedHotBar>();
  commands.init_resource::<PlayerInventory>();
  commands.init_resource::<GameWorld>();
  commands.init_resource::<Option<Selection>>();
}

pub fn transition_to_simulation(
  mut commands: Commands,
  mut window: ResMut<Windows>,
  mut physics_system: ResMut<RapierConfiguration>,
  mut game_world: ResMut<GameWorld>,
  mut client: ResMut<RenetClient>,
  mut nickname: Res<PlayerNickname>,
) {
  let active_window = window.get_primary_mut().unwrap();
  if client.is_connected() {
    client.send_message(
      ClientChannel::ClientCommand.id(),
      serialize(&PlayerCommand::PlayerAuth {
        nickname: nickname.0.clone(),
      })
      .unwrap(),
    );
    commands.insert_resource(NextState(ShikataganaiGameState::Simulation));
    active_window.set_cursor_lock_mode(true);
    active_window.set_cursor_visibility(false);
    physics_system.physics_pipeline_active = true;
    for i in -5..=5 {
      for j in -5..=5 {
        game_world.get_chunk_or_request((i, j), client.as_mut());
      }
    }
  }
}

pub fn cleanup_game(mut commands: Commands) {
  commands.remove_resource::<SelectedHotBar>();
  commands.remove_resource::<PlayerInventory>();
  commands.remove_resource::<GameWorld>();
  commands.remove_resource::<Option<Selection>>();
}

pub fn extract_loopless_state(mut commands: Commands, state: Extract<Res<CurrentState<ShikataganaiGameState>>>) {
  commands.insert_resource(state.clone());
}

pub fn process_animations(
  mut commands: Commands,
  mut transform_query: Query<&mut Transform>,
  mut animations: Query<(Entity, &mut AnimationInstance, &Skeleton)>,
  time: Res<Time>
) {
  for (entity, mut animation, skeleton) in animations.iter_mut() {
    let bone_entity = *skeleton.skeleton.get(&animation.animation.bone).unwrap();
    let mut transform = transform_query.get_mut(bone_entity).unwrap();
    match animation.animation.animation {
      AnimationType::LinearMovement { .. } => {}
      AnimationType::LinearRotation { from, to } => {
        transform.rotation = from.lerp(to, animation.t / animation.animation.duration);
        if animation.t >= animation.animation.duration {
          transform.rotation = to;
        }
      }
    }
    if animation.t >= animation.animation.duration {
      commands.entity(entity).remove::<AnimationInstance>();
    }
    animation.t+=time.delta().as_secs_f32();
  }
}

pub fn interface_input(
  mut commands: Commands,
  inventory_opened: Res<InventoryOpened>,
  key: Res<Input<KeyCode>>,
  // mut physics_system: ResMut<RapierConfiguration>,
  mut windows: ResMut<Windows>,
  mut client: ResMut<RenetClient>,
  reverse_location: Query<&ReverseLocation>
) {
  let window = windows.get_primary_mut().unwrap();

  if key.just_pressed(KeyCode::Escape) {
    commands.remove_resource::<InventoryOpened>();
    animate(&mut commands, inventory_opened.0, ChestAnimations::Close.get_animation());
    client.send_message(ClientChannel::ClientCommand.id(), serialize(&PlayerCommand::AnimationStart { location: reverse_location.get(inventory_opened.0).unwrap().0, animation: ChestAnimations::Close.get_animation() }).unwrap());
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);
    commands.insert_resource(NextState(ShikataganaiGameState::Simulation));
  }
}

impl Plugin for GamePlugin {
  fn build(&self, app: &mut App) {
    let on_main_menu = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::MainMenu)
      .with_system(main_menu)
      .into();
    let on_game_enter = SystemStage::parallel().with_system(init_game); //.with_system(spawn_mesh);
    let on_game_exit = SystemStage::parallel().with_system(cleanup_game);
    let on_game_pre_simulation_update = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::PreSimulation)
      .with_system(transition_to_simulation)
      .with_system(connecting_window)
      .into();
    let on_game_simulation_continuous = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::Simulation)
      // .with_system(action_input)
      .with_system(hot_bar_scroll_input)
      .with_system(hot_bar)
      // .with_system(recalculate_light_map)
      .into();
    let on_in_game_input_enabled = ConditionSet::new()
      .run_if(in_game_input_enabled)
      .with_system(action_input)
      .into();
    let on_in_game_interface_opened = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::InterfaceOpened)
      .with_system(chest_inventory)
      .with_system(interface_input)
      .into();
    let on_game_simulation_continuous_post_update = ConditionSet::new()
      .run_if(in_game)
      .with_system(process_animations)
      .with_system(remesh_system_auxiliary)
      .into();
    let on_pause = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::Paused)
      .with_system(game_menu)
      .into();
    let on_fixed_step_simulation: SystemSet = ConditionSet::new()
      .run_in_state(ShikataganaiGameState::Simulation)
      // .with_system(|| println!("kek"))
      .into();
    let on_fixed_step_simulation_stage = SystemStage::parallel().with_system_set(on_fixed_step_simulation);
    let on_post_update_simulation = ConditionSet::new().run_if(in_game).with_system(religh_system).into();

    app.world.spawn().insert(Player);

    app
      .add_loopless_state(ShikataganaiGameState::MainMenu)
      .add_stage_before(
        CoreStage::Update,
        FixedUpdate,
        FixedTimestepStage::from_stage(Duration::from_millis(125), on_fixed_step_simulation_stage),
      )
      .add_system_set(on_game_simulation_continuous)
      .add_system_set(on_main_menu)
      .add_system_set(on_game_pre_simulation_update)
      .add_system_set(on_pause)
      .add_system_set(on_in_game_input_enabled)
      .add_system_set(on_in_game_interface_opened)
      .add_system_set_to_stage(CoreStage::PostUpdate, on_post_update_simulation)
      .add_system_set_to_stage(CoreStage::PostUpdate, on_game_simulation_continuous_post_update)
      .set_enter_stage(ShikataganaiGameState::MainMenu, on_game_exit)
      .set_enter_stage(ShikataganaiGameState::PreSimulation, on_game_enter);

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app.add_system_to_stage(RenderStage::Extract, extract_loopless_state);
  }
}
