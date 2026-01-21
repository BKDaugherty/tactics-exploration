use bevy::prelude::*;

use crate::{
    GameState,
    animation::{TinytacticsAssets, animation_db::AnimationDB},
    assets::sprite_db::SpriteDB,
    battle::populate_room,
    grid::GridPosition,
    map_generation::{MapParams, setup_map_data_from_params},
    player::{Player, RegisteredBattlePlayers},
    unit::Unit,
};

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[source(GameState = GameState::Dungeon)]
pub enum DungeonState {
    #[default]
    Initialize,
    LoadRoom,
    InBattle,
    LootRoom,
    UnloadRoom,
}

#[derive(Component)]
pub struct DungeonEntity;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub struct RoomId(pub u32);

#[derive(Resource, Reflect)]
pub struct DungeonManager {
    pub current_room: RoomId,
}

#[derive(Component)]
pub struct Teleporter {
    pub current_room: RoomId,
    pub next_room: RoomId,
}

pub fn init_dungeon_manager(
    mut commands: Commands,
    mut next_state: ResMut<NextState<DungeonState>>,
) {
    commands.insert_resource(DungeonManager {
        current_room: RoomId(0),
    });

    next_state.set(DungeonState::LoadRoom);
}

pub fn load_room(
    mut commands: Commands,
    dungeon_manager: Res<DungeonManager>,
    map_params: Res<MapParams>,
    asset_server: Res<AssetServer>,
    registered_players: Res<RegisteredBattlePlayers>,
    tt_assets: Res<TinytacticsAssets>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
    mut next_state: ResMut<NextState<DungeonState>>,
) {
    let room_id = dungeon_manager.current_room;
    let map_data = setup_map_data_from_params(
        &mut commands,
        map_params.options.seed.clone() + room_id.0.to_string().as_str(),
    );
    populate_room(
        &mut commands,
        &asset_server,
        &map_data,
        &registered_players,
        &tt_assets,
        &anim_db,
        &sprite_db,
        room_id,
    );

    next_state.set(DungeonState::InBattle);
}

pub fn unload_room(
    mut commands: Commands,
    entity_query: Query<Entity, With<DungeonEntity>>,
    mut next_state: ResMut<NextState<DungeonState>>,
) {
    // despawn units
    for entity in entity_query {
        commands.entity(entity).despawn();
    }

    // despawn map

    // TODO: this needs to go through some other flow so it's not endless
    next_state.set(DungeonState::LoadRoom)
}

pub fn check_player_on_teleporter(
    teleporter_query: Query<(&Teleporter, &GridPosition)>,
    unit_query: Query<&GridPosition, (With<Unit>, With<Player>)>,
    mut next_state: ResMut<NextState<DungeonState>>,
    mut dungeon_manager: ResMut<DungeonManager>,
) {
    for (teleporter, teleporter_position) in teleporter_query {
        for player_position in unit_query {
            if player_position == teleporter_position {
                // unload_room
                dungeon_manager.current_room = teleporter.next_room;
                next_state.set(DungeonState::UnloadRoom)
            }
        }
    }
}
