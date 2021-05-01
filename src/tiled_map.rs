use bevy_ecs_tilemap::prelude::*;
use anyhow::Result;
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    utils::{HashMap, HashSet},
};
use std::{
    io::BufReader,
    path::{Path, PathBuf},
};
// objects include these by default for now
pub use tiled;
pub use tiled::LayerData;
pub use tiled::ObjectShape;
pub use tiled::Properties;
pub use tiled::PropertyValue;

use crate::layers::TilesetLayer;

// An asset for maps
#[derive(TypeUuid)]
#[uuid = "5f6fbac8-3f52-424e-a928-561667fea074"]
pub struct TiledMap {
    pub map: tiled::Map,
    pub image_folder: std::path::PathBuf,
    pub asset_dependencies: Vec<PathBuf>,
}

impl TiledMap {
    pub fn try_from_bytes(asset_folder: &Path, asset_path: &Path, bytes: Vec<u8>) -> Result<Self> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let root_dir = bevy::asset::FileAssetIo::get_root_path();
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        let root_dir = PathBuf::from("");

        let map = tiled::parse_with_path(
            BufReader::new(bytes.as_slice()),
            &root_dir.join(&asset_folder.join(asset_path)),
        )?;

        // this only works if gids are uniques across all maps used - todo move into ObjectGroup?
        let mut tile_gids: HashMap<u32, u32> = Default::default();

        for tileset in &map.tilesets {
            for i in tileset.first_gid..(tileset.first_gid + tileset.tilecount.unwrap_or(1)) {
                tile_gids.insert(i, tileset.first_gid);
            }
        }
        let image_folder: PathBuf = asset_path.parent().unwrap().into();
        let mut asset_dependencies = Vec::new();

        for layer in map.layers.iter() {
            if !layer.visible {
                continue;
            }

            for tileset in map.tilesets.iter() {
                let tile_path = image_folder.join(tileset.images.first().unwrap().source.as_str());
                asset_dependencies.push(tile_path);
            }
        }

        let map = Self {
            map,
            image_folder,
            asset_dependencies,
        };

        Ok(map)
    }
}

/// A component that keeps track of layers within the tiled map.
pub struct Layers {
    pub map_layer_entities: Vec<Entity>,
}

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapBundle {
    pub map_asset: Handle<TiledMap>,
    pub layers: Layers,
    pub materials: HashMap<u32, Handle<ColorMaterial>>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for TiledMapBundle {
    fn default() -> Self {
        Self {
            map_asset: Handle::default(),
            layers: Layers {
                map_layer_entities: Vec::new(),
            },
            materials: HashMap::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
        }
    }
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut map_events: EventReader<AssetEvent<TiledMap>>,
    mut map_ready_events: EventWriter<MapReadyEvent>,
    mut maps: ResMut<Assets<TiledMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(
        Entity,
        Option<&Children>,
        &Handle<TiledMap>,
        &mut Layers,
        &mut HashMap<u32, Handle<ColorMaterial>>,
    )>,
) {
    let mut changed_maps = HashSet::<Handle<TiledMap>>::default();
    for event in map_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                dbg!("Map added!");
                changed_maps.insert(handle.clone());
            }
            AssetEvent::Modified { handle } => {
                dbg!("Map changed!");
                changed_maps.insert(handle.clone());
            }
            AssetEvent::Removed { handle } => {
                dbg!("Map removed!");
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.remove(handle);
            }
        }
    }



    for changed_map in changed_maps.iter() {
        let tiled_map_asset = maps.get(changed_map).unwrap();

        for (
            entity,
            children,
            map_handle,
            mut layers,
            mut materials_map,
        ) in query.iter_mut() {
            // only deal with currently changed map
            if map_handle != changed_map {
                continue;
            }

            // Clear out child entities
            if let Some(children) = children {
                for child in children.iter() {
                    // TODO: Remove tile entities from map before despawning.
                    commands.entity(*child).despawn_recursive();
                }
            }

            for tileset in &tiled_map_asset.map.tilesets {
                if !materials_map.contains_key(&tileset.first_gid) {
                    let texture_path = tiled_map_asset
                        .image_folder
                        .join(tileset.images.first().unwrap().source.as_str());
                    log::info!("loading image: {:?}", texture_path);
                    let texture_handle = asset_server.load(texture_path);
                    materials_map.insert(
                        tileset.first_gid,
                        materials.add(texture_handle.clone().into()),
                    );
                }

                if let Some(material) = materials_map.get(&tileset.first_gid) {
                    // Once materials have been created/added we need to then create the layers.
                    for layer in tiled_map_asset.map.layers.iter() {
                        TilesetLayer::new(entity, &mut commands, &mut meshes, material.clone(), &tiled_map_asset.map, layer, tileset);
                    }
                }
            }
            
            let evt = MapReadyEvent {
                map_handle: map_handle.clone(),
            };
            map_ready_events.send(evt);
        }
    }
}

// events fired when entity has been created

pub struct MapReadyEvent {
    pub map_handle: Handle<TiledMap>,
}
