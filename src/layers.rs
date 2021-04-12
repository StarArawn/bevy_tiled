use crate::{LayerChunk, TileChunk, utils::project_ortho, utils::project_iso, };
use bevy::prelude::*;

#[derive(Debug)]
pub struct TilesetLayer {
    pub tile_size: Vec2,
    pub chunks: Vec<Vec<LayerChunk>>,
    pub tileset_guid: u32,
}
impl TilesetLayer {
    pub fn new(map: &tiled::Map, layer: &tiled::Layer, tileset: &tiled::Tileset) -> TilesetLayer {
        let target_chunk_x = 32;
        let target_chunk_y = 32;

        let chunk_size_x = (map.width as f32 / target_chunk_x as f32).ceil().max(1.0) as usize;
        let chunk_size_y = (map.height as f32 / target_chunk_y as f32).ceil().max(1.0) as usize;

        let tile_width = tileset.tile_width as f32;
        let tile_height = tileset.tile_height as f32;
        let tile_space = tileset.spacing as f32;

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
                        let chunk_pos = Vec2::new(lookup_x as f32, lookup_y as f32);

                        tiles_y.push(
                            if lookup_x < map.width as usize && lookup_y < map.height as usize {
                                let map_tile = match &layer.tiles {
                                    tiled::LayerData::Finite(tiles) => &tiles[lookup_y][lookup_x],
                                    _ => panic!("Infinte maps not supported"),
                                };
                                // tile not in this set
                                if map_tile.gid < tileset.first_gid
                                    || map_tile.gid
                                        >= tileset.first_gid + tileset.tilecount.unwrap()
                                {
                                    continue;
                                }
                                // Calculate positions
                                let vertex = match map.orientation {
                                    tiled::Orientation::Orthogonal => {
                                        let center =
                                            project_ortho(chunk_pos, tile_width, tile_height);

                                        let start = Vec2::new(
                                            center.x,
                                            center.y - tile_height - tile_space,
                                        );

                                        let end =
                                            Vec2::new(center.x + tile_width + tile_space, center.y);

                                        Vec4::new(start.x, start.y, end.x, end.y)
                                    }
                                    tiled::Orientation::Isometric => {
                                        let center =
                                            project_iso(chunk_pos, tile_width, tile_height);

                                        let start = Vec2::new(
                                            center.x - tile_width / 2.0,
                                            center.y - tile_height,
                                        );

                                        let end = Vec2::new(center.x + tile_width / 2.0, center.y);

                                        Vec4::new(start.x, start.y, end.x, end.y)
                                    }
                                    _ => {
                                        panic!("Unsupported orientation {:?}", map.orientation)
                                    }
                                };
                                // Get chunk tile.
                                TileChunk::from_layer_and_tileset(
                                    map_tile, tileset, chunk_pos, vertex,
                                )
                            } else {
                                // Empty tile
                                TileChunk {
                                    tile_id: 0,
                                    pos: chunk_pos,
                                    vertex: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                    uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                    flip_d: false,
                                    flip_h: false,
                                    flip_v: false,
                                }
                            },
                        ); // end tiles_y.push(chunk_tile);
                    }
                    tiles.push(tiles_y);
                }

                let chunk = LayerChunk {
                    position: Vec2::new(chunk_x as f32, chunk_y as f32),
                    tiles,
                };
                chunks_y.push(chunk);
            }
            chunks.push(chunks_y);
        }

        TilesetLayer {
            tile_size: Vec2::new(tile_width, tile_height),
            chunks,
            tileset_guid: tileset.first_gid,
        }
    }
}
#[derive(Debug)]
pub struct MapLayer {
    pub tileset_layers: Vec<TilesetLayer>,
}
