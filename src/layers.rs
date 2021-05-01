use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::prelude::{Animation, Frame};

#[derive(Debug)]
pub struct TilesetLayer;

impl TilesetLayer {
    pub fn new(
        parent_entity: Entity,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        material: Handle<ColorMaterial>,
        tiled_map: &tiled::Map,
        layer: &tiled::Layer,
        tileset: &tiled::Tileset,
    ) -> Entity {
        let tile_width = tileset.tile_width as f32;
        let tile_height = tileset.tile_height as f32;

        let _tile_space = tileset.spacing as f32; // TODO: re-add tile spacing.. :p

        let mut map = Map::new(
            Vec2::new((tiled_map.width as f32 / 64.0).ceil(), (tiled_map.height as f32 / 64.0).ceil()).into(), 
            Vec2::new(64.0, 64.0).into(),
            Vec2::new(tile_width, tile_height),
            Vec2::new(tileset.images[0].width as f32, tileset.images[0].height as f32), // TODO: support multiple tileset images?
            layer.layer_index,
        );
        map.mesher = match tiled_map.orientation {
            tiled::Orientation::Hexagonal => {
              Box::new(HexChunkMesher::new(HexType::ColumnEven))
            },
            tiled::Orientation::Isometric => {
                Box::new(IsoChunkMesher)
            },
            tiled::Orientation::Orthogonal => {
                Box::new(SquareChunkMesher)
            },
            _ => panic!("Unknown tile map orientation!")
        };

        // Create layer map rendering entity as child of the tiled map.
        let mut map_entity = None;
        commands.entity(parent_entity).with_children(|child_builder| {
            map_entity = Some(child_builder.spawn().id());
        });
        let map_entity = map_entity.unwrap();

        map.build(commands, meshes, material, map_entity, false);
        for x in 0..tiled_map.width as usize {
            for y in 0..tiled_map.height as usize {
                let map_tile = match &layer.tiles {
                    tiled::LayerData::Finite(tiles) => &tiles[y][x],
                    _ => panic!("Infinite maps not supported"),
                };

                if map_tile.gid < tileset.first_gid
                    || map_tile.gid
                        >= tileset.first_gid + tileset.tilecount.unwrap()
                {
                    continue;
                }

                let tile_id = map_tile.gid - tileset.first_gid;
                let mut tile_pos = MapVec2::new(
                    x as i32, //(x as f32 / tile_size_x_diff) as i32,
                    y as i32, //(y as f32 / tile_size_y_diff) as i32
                );
                if tiled_map.orientation == tiled::Orientation::Orthogonal {
                    tile_pos.y = tiled_map.height as i32 - tile_pos.y;
                }
                let tile_entity = map.add_tile(commands, tile_pos, Tile {
                    texture_index: tile_id,
                    flip_x: map_tile.flip_h || map_tile.flip_d,
                    flip_y: map_tile.flip_v || map_tile.flip_d,
                    ..Default::default()
                }).unwrap();

                if let Some(tile) = tileset.tiles.iter().find(|tile| tile.id == tile_id) {
                    if let Some(animations) = tile.animation.clone() {
                        let animation = Animation {
                            frames: animations.iter().map(|frame| Frame {
                                tile_id: frame.tile_id,
                                duration: (frame.duration as f64) / 1000.0,
                            }).collect(),
                            current_frame: 0,
                            last_update: 0.0,
                        };

                        commands.entity(tile_entity).insert(animation);
                    }
                }
                
            }
        }

        commands.entity(map_entity).insert_bundle(MapBundle {
            map,
            transform: Transform::from_xyz(
                layer.offset_x,
                -layer.offset_y,
                layer.layer_index as f32
            ),
            ..Default::default()
        });


        map_entity
    }
}
