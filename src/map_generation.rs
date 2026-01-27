//! Tools for Map Generation!
//! For now, we assume that a dungeon
//! should be linear, and should be composed
//! of DEMO_DUNGEON rooms where the final room is a boss room.

use std::collections::{BTreeMap, HashMap};

use crate::{animation::Direction, battle::BattleEntity, grid::GridPosition};
pub const DEMO_DUNGEON_ROOMS: u8 = 3;
use rand::distr::{Alphanumeric, SampleString, Uniform};
use rand::prelude::*;
use rand_pcg::Pcg64;
use rand_seeder::Seeder;

// Sadly, all of our Tinytactics tiles have height 0.5, so sometimes it can be a bit hard to work with.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub struct MapData {
    pub grid_size: (u32, u32),
    pub tiles: BTreeMap<LayerId, BTreeMap<GridPosition, TileType>>,
    pub player_start_locations: [GridPosition; 4],
    pub bridge_start_locations: [GridPosition; 2],
    pub bridge_end_locations: [GridPosition; 2],
    pub obstacles: HashMap<GridPosition, Obstacle>,
}

pub enum Obstacle {
    Rock1,
    Rock2,
    Bush,
    Tree,
}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct LayerId(pub u32);

/// Game space has x and y swapped and
/// everything is shifted down by 2 for the water barrier
fn to_game_space(g: GridPosition) -> GridPosition {
    GridPosition {
        x: g.y - 2,
        y: g.x - 2,
    }
}

pub fn build_tilemap_from_map(
    commands: &mut Commands,
    texture_handle: Handle<Image>,
    data: &MapData,
) -> Entity {
    let map_entity = commands.spawn_empty().id();
    // Because of our tiles, we end up with some unusable space at the edges.
    // We fill this with some water and cross that water using bridges!
    let map_size = TilemapSize {
        x: data.grid_size.0,
        y: data.grid_size.1,
    };

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = TilemapGridSize { x: 32., y: 16. };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);

    // TODO: Set layer transforms
    for (layer_id, layer) in &data.tiles {
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = commands.spawn_empty().id();

        for (position, tile) in layer {
            let pos = TilePos {
                x: position.x,
                y: position.y,
            };

            let tile_entity = commands
                .spawn(TileBundle {
                    position: pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: tile.tile_texture_index(),
                    ..Default::default()
                })
                .id();

            tile_storage.set(&pos, tile_entity);
        }

        commands.entity(tilemap_entity).insert((
            Name::new(format!("Map Layer {}", layer_id.0)),
            TilemapBundle {
                grid_size,
                size: map_size,
                storage: tile_storage,
                texture: TilemapTexture::Single(texture_handle.clone()),
                tile_size,
                map_type,
                anchor: TilemapAnchor::Center,
                render_settings: TilemapRenderSettings {
                    y_sort: true,
                    render_chunk_size: UVec2 { x: 3, y: 1 },
                },
                transform: Transform::from_translation(Vec3::new(0., 0., layer_id.0 as f32)),
                ..Default::default()
            },
            // TODO: Remove me? once we are managing level movement more dynamically
            BattleEntity {},
        ));
    }

    map_entity
}

pub enum WaterTileType {
    Corner(Direction),
    Edge(Direction),
    Plain,
}

pub enum BridgeTileType {
    Plain(Direction),
}

pub enum TileType {
    Water(WaterTileType),
    Grass(GrassTileType),
    Bridge(BridgeTileType),
}

impl TileType {
    pub fn tile_texture_index(&self) -> TileTextureIndex {
        let index = match self {
            TileType::Water(water_tile_type) => water_tile_type.get_tt_index().index(),
            TileType::Grass(grass_tile_type) => grass_tile_type.get_tt_index().index(),
            TileType::Bridge(bridge_tile_type) => bridge_tile_type.get_tt_index().index(),
        };
        TileTextureIndex(index - 1)
    }
}

pub struct TtIndex {
    /// 1 indexed
    row: u32,
    col: u32,
}

impl TtIndex {
    const TT_COLS: u32 = 16;

    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    // 1 Indexed
    pub fn index(&self) -> u32 {
        (self.row - 1) * Self::TT_COLS + self.col
    }
}

impl WaterTileType {
    fn get_tt_index(&self) -> TtIndex {
        match self {
            WaterTileType::Corner(direction) => match direction {
                Direction::NE => TtIndex::new(3, 6),
                Direction::NW => TtIndex::new(5, 6),
                Direction::SE => TtIndex::new(3, 8),
                Direction::SW => TtIndex::new(5, 8),
            },
            WaterTileType::Edge(direction) => match direction {
                Direction::NE => TtIndex::new(3, 7),
                Direction::NW => TtIndex::new(4, 6),
                Direction::SE => TtIndex::new(4, 8),
                Direction::SW => TtIndex::new(5, 7),
            },
            WaterTileType::Plain => TtIndex::new(3, 2),
        }
    }
}

pub enum GrassTileType {
    Grass,
    DeadGrass,
}

impl GrassTileType {
    fn get_tt_index(&self) -> TtIndex {
        match self {
            GrassTileType::Grass => TtIndex::new(1, 2),
            GrassTileType::DeadGrass => TtIndex::new(2, 15),
        }
    }
}

impl BridgeTileType {
    fn get_tt_index(&self) -> TtIndex {
        match self {
            BridgeTileType::Plain(d) => match d {
                Direction::NE | Direction::SW => TtIndex::new(6, 4),
                Direction::SE | Direction::NW => TtIndex::new(7, 7),
            },
        }
    }
}

#[derive(Resource)]
pub struct MapParams {
    pub options: BattleMapOptions,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct BattleMapOptions {
    #[arg(long, default_value = "hello world")]
    seed: String,
}

pub fn setup_map_data_from_params(mut commands: Commands, res: Res<MapParams>) {
    let grid_size = (17, 17);
    let game_grid_space_x = 2..(grid_size.0 - 2);
    let game_grid_space_y = 2..(grid_size.1 - 2);
    let seed = res.options.seed.clone();
    let mut rng: Pcg64 = Seeder::from(seed).into_rng();

    let mut water_layer = BTreeMap::new();
    let mut ground_layer = BTreeMap::new();
    let bounds_max_x = grid_size.0 - 1;
    let bounds_max_y = grid_size.1 - 1;

    for x in 0..=bounds_max_x {
        for y in 0..=bounds_max_y {
            let rendered_tile = match (x, y) {
                (x, y) if x == bounds_max_x && y == bounds_max_y => {
                    WaterTileType::Corner(Direction::SE)
                }
                (x, 0) if x == bounds_max_x => WaterTileType::Corner(Direction::SW),
                (0, y) if y == bounds_max_y => WaterTileType::Corner(Direction::NE),
                (0, 0) => WaterTileType::Corner(Direction::NW),
                (x, _) if x == bounds_max_x => WaterTileType::Edge(Direction::SE),
                (_, y) if y == bounds_max_y => WaterTileType::Edge(Direction::NE),
                (_, 0) => WaterTileType::Edge(Direction::SW),
                (0, _) => WaterTileType::Edge(Direction::NW),
                _ => WaterTileType::Plain,
            };

            water_layer.insert(GridPosition { x, y }, TileType::Water(rendered_tile));
        }
    }

    for x in 2..=(bounds_max_x - 2) {
        for y in 2..=(bounds_max_x - 2) {
            let tile = if rng.random::<f32>() < 0.05 {
                GrassTileType::DeadGrass
            } else {
                GrassTileType::Grass
            };
            ground_layer.insert(GridPosition { x, y }, TileType::Grass(tile));
        }
    }

    // Need to tell someone about the bridge location we've chosen
    let bridge_location_x_1 = rng.random_range(2..=(bounds_max_x - 2 - 1));
    let bridge_location_x_2 = rng.random_range(2..=(bounds_max_x - 2 - 1));

    for i in bridge_location_x_1..=(bridge_location_x_1 + 1) {
        for y in 0..=2 {
            ground_layer.insert(
                GridPosition { x: i, y },
                TileType::Bridge(BridgeTileType::Plain(Direction::NE)),
            );
        }
    }

    for i in bridge_location_x_2..=(bridge_location_x_2 + 1) {
        for y in (bounds_max_y - 2)..=bounds_max_y {
            ground_layer.insert(
                GridPosition { x: i, y },
                TileType::Bridge(BridgeTileType::Plain(Direction::NE)),
            );
        }
    }

    let player_start_positions = [
        to_game_space(GridPosition {
            x: bridge_location_x_1,
            y: 2,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_1 + 1,
            y: 2,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_1,
            y: 3,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_1 + 1,
            y: 3,
        }),
    ];

    let bridge_start_positions = [player_start_positions[0], player_start_positions[1]];

    let bridge_end_no_block_locations = [
        to_game_space(GridPosition {
            x: bridge_location_x_2,
            y: bounds_max_y,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_2 + 1,
            y: bounds_max_y,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_2,
            y: bounds_max_y - 1,
        }),
        to_game_space(GridPosition {
            x: bridge_location_x_2 + 1,
            y: bounds_max_y - 1,
        }),
    ];

    let on_bridge_end_locations = [
        bridge_end_no_block_locations[0],
        bridge_end_no_block_locations[1],
    ];

    let mut obstacles = HashMap::new();
    for x in game_grid_space_x.clone() {
        for y in game_grid_space_y.clone() {
            let candidate_tile_pos = GridPosition { x, y };
            let game_position = to_game_space(candidate_tile_pos);

            if player_start_positions.contains(&game_position)
                || bridge_end_no_block_locations.contains(&game_position)
            {
                continue;
            }

            let sample = rng.sample(Uniform::new(0.0, 1.0).expect("0 is less than 1"));
            if sample > 0.05 {
                continue;
            }

            // Spawn an obstacle
            let obstacle = match rng.random_range(0..=1) {
                0 => Obstacle::Rock2,
                1 => Obstacle::Bush,
                _ => unreachable!(),
            };

            obstacles.insert(game_position, obstacle);
        }
    }

    // TODO: Can't put trees in as a layer actually as I have those Z Index problems
    // I need to manage these as their own entities
    for y in game_grid_space_y {
        let candidate_pos = to_game_space(GridPosition { x: 2, y });

        if obstacles.contains_key(&candidate_pos) {
            continue;
        }

        if player_start_positions.contains(&candidate_pos)
            || bridge_end_no_block_locations.contains(&candidate_pos)
        {
            continue;
        }

        if rng.random::<f32>() < 0.1 {
            obstacles.insert(candidate_pos, Obstacle::Tree);
        }
    }

    for x in game_grid_space_x {
        let candidate_pos = to_game_space(GridPosition {
            x,
            y: bounds_max_y - 2,
        });

        if obstacles.contains_key(&candidate_pos) {
            continue;
        }

        if player_start_positions.contains(&candidate_pos)
            || bridge_end_no_block_locations.contains(&candidate_pos)
        {
            continue;
        }

        if rng.random::<f32>() < 0.1 {
            obstacles.insert(candidate_pos, Obstacle::Tree);
        }
    }

    commands.insert_resource(MapResource {
        data: MapData {
            grid_size,
            tiles: BTreeMap::from([(LayerId(0), water_layer), (LayerId(1), ground_layer)]),
            player_start_locations: player_start_positions,
            bridge_start_locations: bridge_start_positions,
            bridge_end_locations: on_bridge_end_locations,
            obstacles,
        },
    });
}

#[derive(Resource)]
pub struct MapResource {
    pub data: MapData,
}

pub fn init_map_params(mut commands: Commands) {
    let seed = Alphanumeric.sample_string(&mut rand::rng(), 16);
    info!("Running with seed: {:?}", seed);
    commands.insert_resource(MapParams {
        options: BattleMapOptions { seed },
    })
}
