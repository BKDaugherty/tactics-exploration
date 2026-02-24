use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    GameState,
    animation::{TinytacticsAssets, animation_db::AnimationDB},
    assets::sprite_db::SpriteDB,
    battle::populate_room,
    interactable::{Interactable, InteractionMenuLabel},
    map_generation::{DungeonGenerationParams, MapData, RoomType, setup_map_data_from_params},
    player::RegisteredBattlePlayers,
    unit::{UnitExecuteAction, UnitExecuteActionMessage},
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

#[derive(Resource)]
pub struct DungeonManager {
    pub current_room: RoomId,
    rooms: HashMap<RoomId, DungeonRoomData>,
}

pub struct DungeonRoomData {
    map_data: MapData,
}

#[derive(Component)]
#[require(Interactable, InteractionMenuLabel {
    label: "Advance to Next Room"
})]
pub struct Teleporter {
    pub current_room: RoomId,
    pub next_room: RoomId,
}

pub const DUNGEON_ROOM_COUNT: u32 = 3;

pub fn init_dungeon_manager(
    mut commands: Commands,
    dungeon_params: Res<DungeonGenerationParams>,
    mut next_state: ResMut<NextState<DungeonState>>,
) {
    let mut rooms = HashMap::new();
    for room_id in 0..DUNGEON_ROOM_COUNT {
        let map_data = setup_map_data_from_params(
            &mut commands,
            dungeon_params.options.seed.clone() + room_id.to_string().as_str(),
            RoomType::Standard,
        );

        rooms.insert(RoomId(room_id), DungeonRoomData { map_data });
    }

    commands.insert_resource(DungeonManager {
        current_room: RoomId(0),
        rooms,
    });

    next_state.set(DungeonState::LoadRoom);
}

pub fn load_room(
    mut commands: Commands,
    dungeon_manager: Res<DungeonManager>,
    asset_server: Res<AssetServer>,
    registered_players: Res<RegisteredBattlePlayers>,
    tt_assets: Res<TinytacticsAssets>,
    anim_db: Res<AnimationDB>,
    sprite_db: Res<SpriteDB>,
    mut next_state: ResMut<NextState<DungeonState>>,
) {
    let room_id = dungeon_manager.current_room;
    let Some(room) = dungeon_manager.rooms.get(&room_id) else {
        panic!("Dungeon is mis-initialized!!");
    };

    populate_room(
        &mut commands,
        &asset_server,
        &room.map_data,
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

/// Watches for [`UnitExecuteActionMessage`]s that use [`Teleporter`]s.
///
/// When one is seen, updates the [`DungeonManager`] accordingly, and unloads the current room, and loads the next room.
pub fn handle_teleporter_interaction(
    mut reader: MessageReader<UnitExecuteActionMessage>,
    teleporter_query: Query<&Teleporter>,
    mut next_state: ResMut<NextState<DungeonState>>,
    mut dungeon_manager: ResMut<DungeonManager>,
) {
    for message in reader.read() {
        let UnitExecuteAction::Interact {
            interactable_entity,
        } = message.action
        else {
            continue;
        };

        let Some(teleporter) = teleporter_query.get(interactable_entity).ok() else {
            continue;
        };

        dungeon_manager.current_room = teleporter.next_room;
        next_state.set(DungeonState::UnloadRoom)
    }
}
