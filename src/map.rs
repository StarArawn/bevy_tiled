use bevy::{
    prelude::*,
    render::{
        pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline},
        render_graph::base::MainPass,
    },
};

use crate::{TileMapChunk, TILE_MAP_PIPELINE_HANDLE};
use glam::{Vec2, Vec4};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Tile {
    pub tile_id: u32,
    pub pos: Vec2,
    pub vertex: Vec4,
    pub uv: Vec4,
}

#[derive(Debug)]
pub struct Chunk {
    pub position: Vec2,
    pub tiles: Vec<Vec<Tile>>,
}

#[derive(Debug)]
pub struct TilesetLayer {
    pub tile_size: Vec2,
    pub chunks: Vec<Vec<Chunk>>,
    pub tileset_guid: u32,
}

#[derive(Debug)]
pub struct Layer {
    pub tileset_layers: Vec<TilesetLayer>,
}

// An asset for maps
#[derive(Debug)]
pub struct Map {
    pub map: tiled::Map,
    pub meshes: Vec<(u32, u32, Mesh)>,
    pub layers: Vec<Layer>,
    pub tile_size: Vec2,
    pub image_folder: String,
}

impl Map {
    pub fn center(&self) -> Translation {
        let tile_size = Vec2::new(self.map.tile_width as f32, self.map.tile_height as f32);
        let width = self.map.width as f32;
        let height = self.map.height as f32;
        match self.map.orientation {
            tiled::Orientation::Orthogonal => Translation::new(
                -tile_size.x() * width * 2.0,
                tile_size.y() * height * 2.0,
                0.0,
            ),
            tiled::Orientation::Isometric => Translation::new(
                ((tile_size.x() * (width * 2.0) / 2.0) + (height * tile_size.x() / 2.0)
                    - (height / 4.0 * tile_size.x() / 2.0))
                    * -2.0,
                (((height - (height / 4.0) - 1.0) * tile_size.y() / 2.0)
                    + (width * tile_size.y() / 2.0)
                    - (width / 4.0 * tile_size.y() / 2.0))
                    * -4.0,
                0.0,
            ),

            _ => panic!("Unsupported orientation {:?}", self.map.orientation),
        }
    }
}

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapComponents {
    pub map_asset: Handle<Map>,
    pub materials: HashMap<u32, Handle<ColorMaterial>>,
    pub center: bool,
}

impl Default for TiledMapComponents {
    fn default() -> Self {
        Self {
            map_asset: Handle::default(),
            materials: HashMap::default(),
            center: false,
        }
    }
}

#[derive(Default)]
pub struct MapResourceProviderState {
    map_event_reader: EventReader<AssetEvent<Map>>,
}

#[derive(Bundle)]
pub struct ChunkComponents {
    pub chunk: TileMapChunk,
    pub main_pass: MainPass,
    pub material: Handle<ColorMaterial>,
    pub render_pipeline: RenderPipelines,
    pub draw: Draw,
    pub mesh: Handle<Mesh>,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for ChunkComponents {
    fn default() -> Self {
        Self {
            chunk: TileMapChunk::default(),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            mesh: Handle::default(),
            material: Handle::default(),
            render_pipeline: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                TILE_MAP_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0,
                        },
                        // Tile map chunk data
                        DynamicBinding {
                            bind_group: 2,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: Local<MapResourceProviderState>,
    map_events: Res<Events<AssetEvent<Map>>>,
    mut maps: ResMut<Assets<Map>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(
        Entity,
        &bool,
        &Handle<Map>,
        &mut HashMap<u32, Handle<ColorMaterial>>,
    )>,
) {
    let mut changed_maps = HashSet::<Handle<Map>>::new();
    for event in state.map_event_reader.iter(&map_events) {
        match event {
            AssetEvent::Created { handle } => {
                changed_maps.insert(*handle);
            }
            AssetEvent::Modified { handle } => {
                changed_maps.insert(*handle);
            }
            AssetEvent::Removed { handle } => {
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.remove(handle);
            }
        }
    }

    let mut new_meshes = HashMap::<&Handle<Map>, Vec<(u32, u32, Handle<Mesh>)>>::new();
    for changed_map in changed_maps.iter() {
        let map = maps.get_mut(changed_map).unwrap();

        for (_, _, _, mut materials_map) in &mut query.iter() {
            for tileset in &map.map.tilesets {
                if !materials_map.contains_key(&tileset.first_gid) {
                    let texture_path =
                        map.image_folder.clone() + "/" + &tileset.images.first().unwrap().source;
                    let texture_handle = asset_server.load(texture_path).unwrap();
                    materials_map.insert(tileset.first_gid, materials.add(texture_handle.into()));
                }
            }
        }

        for mesh in map.meshes.drain(0..map.meshes.len()) {
            let handle = meshes.add(mesh.2);

            if new_meshes.contains_key(changed_map) {
                let mesh_list = new_meshes.get_mut(changed_map).unwrap();
                mesh_list.push((mesh.0, mesh.1, handle));
            } else {
                let mut mesh_list = Vec::new();
                mesh_list.push((mesh.0, mesh.1, handle));
                new_meshes.insert(changed_map, mesh_list);
            }
        }
    }

    for (_, center, map_handle, materials_map) in &mut query.iter() {
        if new_meshes.contains_key(map_handle) {
            let map = maps.get(map_handle).unwrap();

            let translation = if *center {
                map.center()
            } else {
                Translation::default()
            };

            let mesh_list = new_meshes.get_mut(map_handle).unwrap();

            for (layer_id, layer) in map.layers.iter().enumerate() {
                for tileset_layer in layer.tileset_layers.iter() {
                    let material_handle = materials_map.get(&tileset_layer.tileset_guid).unwrap();
                    // let mut mesh_list = mesh_list.iter_mut().filter(|(mesh_layer_id, _)| *mesh_layer_id == layer_id as u32).drain(0..mesh_list.len()).collect::<Vec<_>>();
                    let chunk_mesh_list = mesh_list
                        .iter()
                        .filter(|(mesh_layer_id, tileset_guid, _)| {
                            *mesh_layer_id == layer_id as u32
                                && *tileset_guid == tileset_layer.tileset_guid
                        })
                        .collect::<Vec<_>>();

                    for (_, _, mesh) in chunk_mesh_list.iter() {
                        // TODO: Sadly bevy doesn't support multiple meshes on a single entity with multiple materials.
                        // Change this once it does.

                        // Instead for now spawn a new entity per chunk.
                        commands.spawn(ChunkComponents {
                            chunk: TileMapChunk {
                                // TODO: Support more layers here..
                                layer_id: layer_id as f32,
                            },
                            material: material_handle.clone(),
                            mesh: mesh.clone(),
                            translation: translation.clone(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}
