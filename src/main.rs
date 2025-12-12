use bevy::ecs::query;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::prelude::ActionState;
use tactics_exploration::grid::{self, GridManager, GridPosition, grid_to_world, init_grid_to_world_transform };
use tactics_exploration::player::{Player, PlayerInputAction};
use tactics_exploration::{Ground, grid_cursor, player};

fn main() {
    App::new()
        // Add Bevy's default plugins
        .add_plugins(DefaultPlugins)
        // TODO: Dev Mode
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        // Add the bevy_ecs_tiled plugin
        // bevy_ecs_tilemap::TilemapPlugin will be added automatically if needed
        .add_plugins(TiledPlugin::default())
        .add_plugins(TiledDebugPluginGroup)
        .add_plugins(InputManagerPlugin::<PlayerInputAction>::default())
        // Add your startup system and run the app
        .add_systems(Startup, startup)
        .add_systems(Startup, startup_load_overlay_sprite_data.after(startup))
        .add_systems(Startup, create_cursor_for_player_1.after(startup_load_overlay_sprite_data))
        .add_systems(Update, (change_zoom, grid::sync_grid_positions_to_manager, grid::sync_grid_position_to_transform, grid::sync_grid_movement_to_transform, on_asset_event))
        .add_systems(Update, (destroy_overlay_on_map, generate_overlay_on_map))
        .add_systems(Update, grid_cursor::handle_cursor_movement)
        .run();
}

/// Resource because one of them? Split screen maybe would need two?
#[derive(Debug, Resource)]
struct CameraSettings {
    pub zoom_value: f32,
}

pub const SQUARE_GRID_BOUNDS : u32 = 6;

#[derive(Resource, Default)]
struct TileOverlayAssets {
    tile_overlay_image_handle: Handle<Image>,
    tile_overlay_atlas_layout_handle: Handle<TextureAtlasLayout>,
}

// This system reads all AssetEvents for the Image type and attempts to set the ImageSampler values to nearest to stop some texture bleeding
fn on_asset_event(
    mut events: MessageReader<AssetEvent<Image>>,
    asset_handles: Res<TileOverlayAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in events.read() {
        // You can check the type of event and the specific handle
        match event {
            AssetEvent::LoadedWithDependencies { id } => {
                if *id == asset_handles.tile_overlay_image_handle.id() {
                    info!("Our specific image asset and its dependencies are loaded!");
                    if let Some(image) = images.get_mut(*id) {
                        image.sampler = ImageSampler::nearest();
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Component)]
pub struct TileOverlay {}

#[derive(Bundle)]
struct TileOverlayBundle {
    grid_position: grid::GridPosition,
    sprite: Sprite,
    transform: Transform,
    tile_overlay: TileOverlay,
}

impl TileOverlayBundle {
    fn new(
        grid_position: grid::GridPosition,
        spritesheet: Handle<Image>,
        atlas_layout_handle: Handle<TextureAtlasLayout>,
    ) -> Self {
        Self {
            grid_position,
            sprite: Sprite {
                image: spritesheet,
                texture_atlas: Some(TextureAtlas {
                    layout: atlas_layout_handle,
                    index: 1,
                }), 
                custom_size: None,
                color: Color::linear_rgba(1.0, 1.0, 1.0, 0.3),
                ..Default::default()
            },
            transform: init_grid_to_world_transform(&grid_position),
            tile_overlay: TileOverlay {},
        }
    }
}

fn create_cursor_for_player_1(
    commands: Commands,
    tile_overlay_assets: Res<TileOverlayAssets>,
) {
    grid_cursor::spawn_cursor(
        commands,
        tile_overlay_assets.tile_overlay_image_handle.clone(),
        tile_overlay_assets.tile_overlay_atlas_layout_handle.clone(),
        player::Player::One,
    );
}

fn generate_overlay_on_map(
    mut commands: Commands,
    mut grid_manager_res: ResMut<grid::GridManagerResource>,
    tile_overlay_assets: Res<TileOverlayAssets>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
) {
    for (_, action_state) in player_query.iter() {
        if action_state.just_pressed(&PlayerInputAction::CreateOverlayRemoveMe) {
            for grid_pos_y in 1..=SQUARE_GRID_BOUNDS {
                for grid_pos_x in 1..=SQUARE_GRID_BOUNDS {
                    let grid_pos = grid::GridPosition {
                        x: grid_pos_x,
                        y: grid_pos_y,
                    };
                    let e = commands.spawn((
                    TileOverlayBundle::new(grid_pos,
                        tile_overlay_assets.tile_overlay_image_handle.clone(),
                        tile_overlay_assets.tile_overlay_atlas_layout_handle.clone(),
                    ),
                )).id();
                    grid_manager_res.grid_manager.add_entity(e, grid_pos);
                }
            }
        }
    }
}

fn destroy_overlay_on_map(
    mut commands: Commands,
    overlay_query: Query<(Entity, &TileOverlay)>,  
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>,
) {
    for (_, action_state) in player_query.iter() {
        if action_state.just_pressed(&PlayerInputAction::DeleteOverlayRemoveMe) {
            for (entity, _) in overlay_query.iter() {
                commands.entity(entity).despawn();  
            }
        }
    }
}

fn startup_load_overlay_sprite_data(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,

) {
    let debug_color_spritesheet = asset_server.load("random-assets/iso_color.png");
    commands.insert_resource(TileOverlayAssets {
        tile_overlay_image_handle: debug_color_spritesheet.clone(),
        tile_overlay_atlas_layout_handle: {
            let layout = TextureAtlasLayout::from_grid(UVec2::new(64, 32), 6, 1, None, None);
            texture_atlas_layouts.add(layout)
        },
    });
}


fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn a 2D camera
    let t = Transform::default();
    let camera_settings = CameraSettings {
        zoom_value: 1.0
    };
    
    commands.spawn((
        Name::new("Main Camera"),
        Camera2d,
        Projection::from(
            OrthographicProjection { 
                scale: camera_settings.zoom_value,
                ..OrthographicProjection::default_2d()
            }),
        t,
    ));
    
    commands.insert_resource(camera_settings);

    // Load a map asset and retrieve its handle
    let map_handle: Handle<TiledMapAsset> = asset_server.load("random-assets/example-map.tmx");
    // let map_handle: Handle<TiledMapAsset> = asset_server.load("example-map.tmx");

    // Spawn a new entity with the TiledMap component
    commands.spawn(TiledMap(map_handle));
    
    commands.insert_resource(grid::GridManagerResource {
        grid_manager: GridManager::new(SQUARE_GRID_BOUNDS, SQUARE_GRID_BOUNDS)
    });

    commands.spawn((
        Name::new("Player One"),
        player::PlayerBundle::new(Player::One),
    ));
}

fn change_zoom(
    mut camera: Single<&mut Projection, With<Camera>>,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<(&Player, &ActionState<PlayerInputAction>)>
) {
    for (_, action_state) in player_query.iter() {
        if action_state.just_pressed(&PlayerInputAction::ZoomIn) {
            match **camera {
                Projection::Orthographic(ref mut current_projection) => {
                    current_projection.scale += 0.1;
                    camera_settings.zoom_value = current_projection.scale;
                }
                _ => return,
            } 
        } else if action_state.just_pressed(&PlayerInputAction::ZoomOut) {
            match **camera {
                Projection::Orthographic(ref mut current_projection) => {
                    current_projection.scale -= 0.1;
                    camera_settings.zoom_value = current_projection.scale;
                },
                _ => return,
            } 
        }
    }
}
