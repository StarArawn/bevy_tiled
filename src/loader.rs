use crate::{
    map::{Chunk, Map},
    Layer, Tile, TilesetLayer,
};
use anyhow::Result;
use bevy::{
    asset::AssetLoader,
    prelude::Mesh,
    render::{mesh::VertexAttribute, pipeline::PrimitiveTopology},
};
use glam::{Vec2, Vec4};
use std::{io::BufReader, path::Path};

#[derive(Default)]
pub struct TiledMapLoader;

impl TiledMapLoader {
    fn remove_tile_flags(tile: u32) -> u32 {
        let tile = tile & !ALL_FLIP_FLAGS;
        tile
    }
}

const FLIPPED_HORIZONTALLY_FLAG: u32 = 0x80000000;
const FLIPPED_VERTICALLY_FLAG: u32 = 0x40000000;
const FLIPPED_DIAGONALLY_FLAG: u32 = 0x20000000;
const ALL_FLIP_FLAGS: u32 =
    FLIPPED_HORIZONTALLY_FLAG | FLIPPED_VERTICALLY_FLAG | FLIPPED_DIAGONALLY_FLAG;

impl AssetLoader<Map> for TiledMapLoader {
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<Map> {
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
                                    // let map_tile = match &layer.tiles {
                                    //     tiled::LayerData::Finite(tiles) => {
                                    //         &tiles[lookup_y][lookup_x]
                                    //     },
                                    //     _ => panic!("Infinte maps not supported"),
                                    // };

                                    let map_tile = layer.tiles[lookup_y][lookup_x];

                                    let tile = map_tile.gid;
                                    if tile < tileset.first_gid
                                        || tile >= tileset.first_gid + tileset.tilecount.unwrap()
                                    {
                                        continue;
                                    }

                                    let tile = (Self::remove_tile_flags(tile) as f32)
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
                                                center.x() - tile_width / 2.0,
                                                center.y() - tile_height / 2.0,
                                            );

                                            let end = Vec2::new(
                                                center.x() + tile_width / 2.0,
                                                center.y() + tile_height / 2.0,
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
                                                center.y() - tile_height / 2.0,
                                            );

                                            let end = Vec2::new(
                                                center.x() + tile_width / 2.0,
                                                center.y() + tile_height / 2.0,
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

                        let mut positions = Vec::new();
                        let mut normals = Vec::new();
                        let mut uvs = Vec::new();
                        let mut indices = Vec::new();

                        let mut i = 0;
                        for tile in chunk.tiles.iter().flat_map(|tiles_y| tiles_y.iter()) {
                            if tile.tile_id < tileset_layer.tileset_guid {
                                continue;
                            }

                            // X, Y
                            positions.push([tile.vertex.x(), tile.vertex.y(), 0.0]);
                            normals.push([0.0, 0.0, 1.0]);
                            uvs.push([tile.uv.x(), tile.uv.w()]);

                            // X, Y + 1
                            positions.push([tile.vertex.x(), tile.vertex.w(), 0.0]);
                            normals.push([0.0, 0.0, 1.0]);
                            uvs.push([tile.uv.x(), tile.uv.y()]);

                            // X + 1, Y + 1
                            positions.push([tile.vertex.z(), tile.vertex.w(), 0.0]);
                            normals.push([0.0, 0.0, 1.0]);
                            uvs.push([tile.uv.z(), tile.uv.y()]);

                            // X + 1, Y
                            positions.push([tile.vertex.z(), tile.vertex.y(), 0.0]);
                            normals.push([0.0, 0.0, 1.0]);
                            uvs.push([tile.uv.z(), tile.uv.w()]);

                            let mut new_indices = vec![i + 0, i + 2, i + 1, i + 0, i + 3, i + 2];
                            indices.append(&mut new_indices);

                            i += 4;
                        }

                        if positions.len() > 0 {
                            let mesh = Mesh {
                                primitive_topology: PrimitiveTopology::TriangleList,
                                attributes: vec![
                                    VertexAttribute::position(positions),
                                    VertexAttribute::normal(normals),
                                    VertexAttribute::uv(uvs),
                                ],
                                indices: Some(indices),
                            };
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
            image_folder: asset_path.parent().unwrap().to_str().unwrap().to_string(),
        };

        Ok(map)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tmx"];
        EXTENSIONS
    }
}
