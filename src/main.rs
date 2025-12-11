use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use tactics_exploration::grid::{self, GridManager, GridPosition };
use tactics_exploration::Ground;

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
        // Add your startup system and run the app
        .add_systems(Startup, startup)
        .add_systems(Update, (change_zoom, grid::sync_grid_positions_to_manager, grid::sync_grid_position_to_transform, grid::sync_grid_movement_to_transform, on_asset_event))
        .run();
}


/// Resource because one of them? Split screen maybe would need two?
#[derive(Debug, Resource)]
struct CameraSettings {
    pub zoom_value: f32,
}

pub const SQUARE_GRID_BOUNDS : u32 = 6;

// Store handles of assets we care about in a resource or component
#[derive(Resource, Default)]
struct AssetsHandles {
    image_handle: Handle<Image>,
}

// This system reads all AssetEvents for the Image type and attempts to set the ImageSampler values to nearest to stop some texture bleeding
fn on_asset_event(
    mut events: MessageReader<AssetEvent<Image>>,
    asset_handles: Res<AssetsHandles>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in events.read() {
        // You can check the type of event and the specific handle
        match event {
            AssetEvent::LoadedWithDependencies { id } => {
                if *id == asset_handles.image_handle.id() {
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
fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
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

    
    let debug_color_spritesheet = asset_server.load("random-assets/iso_color.png");
    // Fair bit of jankness here to try to get rid of texture bleeding
    commands.insert_resource(AssetsHandles {
        image_handle: debug_color_spritesheet.clone(),
    });


    let layout = TextureAtlasLayout::from_grid(UVec2::new(64, 32), 6, 1, None, None);
    let atlas_layout_handle = texture_atlas_layouts.add(layout);
    
    // TODO: Tie TiledMap generation to GridManager! 
    // (I want to attach the component Ground / GridPosition to the tiles built)
    // Or at least get those into the GridManager
    let mut grid_manager = GridManager::new(SQUARE_GRID_BOUNDS, SQUARE_GRID_BOUNDS);
    for grid_pos_y in 1..=SQUARE_GRID_BOUNDS {
        for grid_pos_x in 1..=SQUARE_GRID_BOUNDS {
            let grid_pos = grid::GridPosition {
                x: grid_pos_x,
                y: grid_pos_y,
            };
            let e = commands.spawn((
            grid_pos,
            Ground{},
            Sprite {
                image: debug_color_spritesheet.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: atlas_layout_handle.clone(),
                    index: 1,
                }), 
                custom_size: None,
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(
                0.0,
                0.0,
                // TODO: How to better manage this z value with respect to bevy_ecs_tiled layers?
                // In general, I just want my values to be above the tilemap layers
                600.0,
            )),
        )).id();

        grid_manager.add_entity(e, grid_pos);
        }
    }
        
    commands.insert_resource(grid::GridManagerResource {
        grid_manager,
    });
}

fn change_zoom(
    mut camera: Single<&mut Projection, With<Camera>>,
    mut camera_settings: ResMut<CameraSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyZ) {
        // Switch projection type
        match **camera {
            Projection::Orthographic(ref mut current_projection) => {
                current_projection.scale += 0.1;
                camera_settings.zoom_value = current_projection.scale;
            }
            _ => return,
        } 
    } else if keyboard_input.just_pressed(KeyCode::KeyX) {
        match **camera {
            Projection::Orthographic(ref mut current_projection) => {
                current_projection.scale -= 0.1;
                camera_settings.zoom_value = current_projection.scale;
            },
            _ => return,
        } 
    }
}
