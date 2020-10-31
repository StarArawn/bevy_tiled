use anyhow::Result;
use bevy::{
    prelude::*,
    render::mesh::Indices,
    render::{
        mesh::VertexAttributeValues,
        pipeline::PrimitiveTopology,
        pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline},
        render_graph::base::MainPass,
    },
    utils::HashMap,
};
use bevy_type_registry::TypeUuid;

use crate::{loader::TiledMapLoader, TileMapChunk, TILE_MAP_PIPELINE_HANDLE};
use glam::Vec2;
use std::{collections::HashSet, io::BufReader, path::Path};

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
#[derive(Debug, TypeUuid)]
#[uuid = "5f6fbac8-3f52-424e-a928-561667fea074"]
pub struct Map {
    pub map: tiled::Map,
    pub meshes: Vec<(u32, u32, Mesh)>,
    pub layers: Vec<Layer>,
    pub tile_size: Vec2,
    pub image_folder: std::path::PathBuf,
}

impl Map {
    pub fn project_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = tile_width * pos.x();
        let y = tile_height * pos.y();
        Vec2::new(x, -y)
    }
    pub fn unproject_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = pos.x() / tile_width;
        let y = -(pos.y()) / tile_height;
        Vec2::new(x, y)
    }
    pub fn project_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = (pos.x() - pos.y()) * tile_width / 2.0;
        let y = (pos.x() + pos.y()) * tile_height / 2.0;
        Vec2::new(x, -y)
    }
    pub fn unproject_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let half_width = tile_width / 2.0;
        let half_height = tile_height / 2.0;
        let x = ((pos.x() / half_width) + (-(pos.y()) / half_height)) / 2.0;
        let y = ((-(pos.y()) / half_height) - (pos.x() / half_width)) / 2.0;
        Vec2::new(x.round(), y.round())
    }
    pub fn center(&self, origin: Transform) -> Transform {
        let tile_size = Vec2::new(self.map.tile_width as f32, self.map.tile_height as f32);
        let map_center = Vec2::new(self.map.width as f32 / 2.0, self.map.height as f32 / 2.0);
        match self.map.orientation {
            tiled::Orientation::Orthogonal => {
                let center = Map::project_ortho(map_center, tile_size.x(), tile_size.y());
                Transform::from_matrix(
                    origin.compute_matrix() * Mat4::from_translation(-center.extend(0.0)),
                )
            }
            tiled::Orientation::Isometric => {
                let center = Map::project_iso(map_center, tile_size.x(), tile_size.y());
                Transform::from_matrix(
                    origin.compute_matrix() * Mat4::from_translation(-center.extend(0.0)),
                )
            }
            _ => panic!("Unsupported orientation {:?}", self.map.orientation),
        }
    }

    pub fn try_from_bytes(asset_path: &Path, bytes: Vec<u8>) -> Result<Map> {
        let map = tiled::parse_with_path(BufReader::new(bytes.as_slice()), asset_path).unwrap();

        let mut layers = Vec::new();

        let target_chunk_x = 32;
        let target_chunk_y = 32;

        let chunk_size_x = (map.width as f32 / target_chunk_x as f32).ceil().max(1.0) as usize;
        let chunk_size_y = (map.height as f32 / target_chunk_y as f32).ceil().max(1.0) as usize;
        let tile_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);

        for layer in map.layers.iter() {
            if !layer.visible {
                continue;
            }
            let mut tileset_layers = Vec::new();

            for tileset in map.tilesets.iter() {
                let tile_width = tileset.tile_width as f32;
                let tile_height = tileset.tile_height as f32;
                let image = tileset.images.first().unwrap();
                let texture_width = image.width as f32;
                let texture_height = image.height as f32;
                let columns = (texture_width / tile_width).floor();

                let mut chunks = Vec::new();
                // 32 x 32 tile chunk sizes
                for chunk_x in 0..chunk_size_x {
                    let mut chunks_y = Vec::new();
                    for chunk_y in 0..chunk_size_y {
                        let mut tiles = Vec::new();

                        for tile_x in 0..target_chunk_x {
                            let mut tiles_y = Vec::new();
                            for tile_y in 0..target_chunk_y {
                                let lookup_x = (chunk_x * target_chunk_x) + tile_x;
                                let lookup_y = (chunk_y * target_chunk_y) + tile_y;

                                // Get chunk tile.
                                let chunk_tile = if lookup_x < map.width as usize
                                    && lookup_y < map.height as usize
                                {
                                    // New Tiled crate code:
                                    let map_tile = match &layer.tiles {
                                        tiled::LayerData::Finite(tiles) => {
                                            &tiles[lookup_y][lookup_x]
                                        }
                                        _ => panic!("Infinte maps not supported"),
                                    };

                                    let tile = map_tile.gid;
                                    if tile < tileset.first_gid
                                        || tile >= tileset.first_gid + tileset.tilecount.unwrap()
                                    {
                                        continue;
                                    }

                                    let tile = (TiledMapLoader::remove_tile_flags(tile) as f32)
                                        - tileset.first_gid as f32;

                                    // This calculation is much simpler we only care about getting the remainder
                                    // and multiplying that by the tile width.
                                    let sprite_sheet_x: f32 = (tile % columns * tile_width).floor();

                                    // Calculation here is (tile / columns).round_down * tile_height
                                    // Example: tile 30 / 28 columns = 1.0714 rounded down to 1 * 16 tile_height = 16 Y
                                    // which is the 2nd row in the sprite sheet.
                                    // Example2: tile 10 / 28 columns = 0.3571 rounded down to 0 * 16 tile_height = 0 Y
                                    // which is the 1st row in the sprite sheet.
                                    let sprite_sheet_y: f32 =
                                        (tile / columns).floor() * tile_height;

                                    // Calculate positions
                                    let (start_x, end_x, start_y, end_y) = match map.orientation {
                                        tiled::Orientation::Orthogonal => {
                                            let center = Map::project_ortho(
                                                Vec2::new(lookup_x as f32, lookup_y as f32),
                                                tile_width,
                                                tile_height,
                                            );

                                            let start = Vec2::new(
                                                center.x(),
                                                center.y() - tile_height,
                                            );

                                            let end = Vec2::new(
                                                center.x() + tile_width,
                                                center.y(),
                                            );

                                            (start.x(), end.x(), start.y(), end.y())
                                        }
                                        tiled::Orientation::Isometric => {
                                            let center = Map::project_iso(
                                                Vec2::new(lookup_x as f32, lookup_y as f32),
                                                tile_width,
                                                tile_height,
                                            );

                                            let start = Vec2::new(
                                                center.x() - tile_width / 2.0,
                                                center.y() - tile_height,
                                            );

                                            let end = Vec2::new(
                                                center.x() + tile_width / 2.0,
                                                center.y(),
                                            );

                                            (start.x(), end.x(), start.y(), end.y())
                                        }
                                        _ => {
                                            panic!("Unsupported orientation {:?}", map.orientation)
                                        }
                                    };

                                    // Calculate UV:
                                    let mut start_u: f32 = sprite_sheet_x / texture_width;
                                    let mut end_u: f32 =
                                        (sprite_sheet_x + tile_width) / texture_width;
                                    let mut start_v: f32 = sprite_sheet_y / texture_height;
                                    let mut end_v: f32 =
                                        (sprite_sheet_y + tile_height) / texture_height;

                                    if map_tile.flip_h {
                                        let temp_startu = start_u;
                                        start_u = end_u;
                                        end_u = temp_startu;
                                    }
                                    if map_tile.flip_v {
                                        let temp_startv = start_v;
                                        start_v = end_v;
                                        end_v = temp_startv;
                                    }

                                    Tile {
                                        tile_id: map_tile.gid,
                                        pos: Vec2::new(tile_x as f32, tile_y as f32),
                                        vertex: Vec4::new(start_x, start_y, end_x, end_y),
                                        uv: Vec4::new(start_u, start_v, end_u, end_v),
                                    }
                                } else {
                                    // Empty tile
                                    Tile {
                                        tile_id: 0,
                                        pos: Vec2::new(tile_x as f32, tile_y as f32),
                                        vertex: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                        uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                    }
                                };

                                tiles_y.push(chunk_tile);
                            }
                            tiles.push(tiles_y);
                        }

                        let chunk = Chunk {
                            position: Vec2::new(chunk_x as f32, chunk_y as f32),
                            tiles,
                        };
                        chunks_y.push(chunk);
                    }
                    chunks.push(chunks_y);
                }

                let tileset_layer = TilesetLayer {
                    tile_size: Vec2::new(tile_width, tile_height),
                    chunks,
                    tileset_guid: tileset.first_gid,
                };
                tileset_layers.push(tileset_layer);
            }

            let layer = Layer { tileset_layers };
            layers.push(layer);
        }

        let mut meshes = Vec::new();
        for (layer_id, layer) in layers.iter().enumerate() {
            for tileset_layer in layer.tileset_layers.iter() {
                for x in 0..tileset_layer.chunks.len() {
                    let chunk_x = &tileset_layer.chunks[x];
                    for y in 0..chunk_x.len() {
                        let chunk = &chunk_x[y];

                        let mut positions: Vec<[f32; 3]> = Vec::new();
                        let mut uvs: Vec<[f32; 2]> = Vec::new();
                        let mut indices: Vec<u32> = Vec::new();

                        let mut i = 0;
                        for tile in chunk.tiles.iter().flat_map(|tiles_y| tiles_y.iter()) {
                            if tile.tile_id < tileset_layer.tileset_guid {
                                continue;
                            }

                            // X, Y
                            positions.push([tile.vertex.x(), tile.vertex.y(), 0.0]);
                            uvs.push([tile.uv.x(), tile.uv.w()]);

                            // X, Y + 1
                            positions.push([tile.vertex.x(), tile.vertex.w(), 0.0]);
                            uvs.push([tile.uv.x(), tile.uv.y()]);

                            // X + 1, Y + 1
                            positions.push([tile.vertex.z(), tile.vertex.w(), 0.0]);
                            uvs.push([tile.uv.z(), tile.uv.y()]);

                            // X + 1, Y
                            positions.push([tile.vertex.z(), tile.vertex.y(), 0.0]);
                            uvs.push([tile.uv.z(), tile.uv.w()]);

                            indices.extend_from_slice(&[i + 0, i + 2, i + 1, i + 0, i + 3, i + 2]);

                            i += 4;
                        }

                        if positions.len() > 0 {
                            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                            mesh.set_attribute(
                                "Vertex_Position",
                                VertexAttributeValues::Float3(positions),
                            );
                            mesh.set_attribute("Vertex_Uv", VertexAttributeValues::Float2(uvs));
                            mesh.set_indices(Some(Indices::U32(indices)));
                            meshes.push((layer_id as u32, tileset_layer.tileset_guid, mesh));
                        }
                    }
                }
            }
        }

        let map = Map {
            map,
            meshes,
            layers,
            tile_size,
            image_folder: asset_path.parent().unwrap().into(),
        };

        Ok(map)
    }
}

#[derive(Default)]
pub struct TiledMapCenter(pub bool);

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapComponents {
    pub map_asset: Handle<Map>,
    pub materials: HashMap<u32, Handle<ColorMaterial>>,
    pub origin: Transform,
    pub center: TiledMapCenter,
}

impl Default for TiledMapComponents {
    fn default() -> Self {
        Self {
            map_asset: Handle::default(),
            materials: HashMap::default(),
            center: TiledMapCenter::default(),
            origin: Transform::default(),
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
    pub global_transform: GlobalTransform,
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
            global_transform: Default::default(),
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
        &TiledMapCenter,
        &Handle<Map>,
        &mut HashMap<u32, Handle<ColorMaterial>>,
        &Transform,
    )>,
) {
    let mut changed_maps = HashSet::<Handle<Map>>::new();
    for event in state.map_event_reader.iter(&map_events) {
        match event {
            AssetEvent::Created { handle } => {
                changed_maps.insert(handle.clone());
            }
            AssetEvent::Modified { handle } => {
                changed_maps.insert(handle.clone());
            }
            AssetEvent::Removed { handle } => {
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.remove(handle);
            }
        }
    }

    let mut new_meshes = HashMap::<&Handle<Map>, Vec<(u32, u32, Handle<Mesh>)>>::default();
    for changed_map in changed_maps.iter() {
        let map = maps.get_mut(changed_map).unwrap();

        for (_, _, _, mut materials_map, _) in query.iter_mut() {
            for tileset in &map.map.tilesets {
                if !materials_map.contains_key(&tileset.first_gid) {
                    let texture_path = map
                        .image_folder
                        .join(tileset.images.first().unwrap().source.as_str());
                    let texture_handle = asset_server.load(texture_path);
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

    for (_, center, map_handle, materials_map, origin) in query.iter_mut() {
        if new_meshes.contains_key(map_handle) {
            let map = maps.get(map_handle).unwrap();

            let tile_map_transform = if center.0 {
                map.center(origin.clone())
            } else {
                origin.clone()
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
                            transform: tile_map_transform.clone(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}
