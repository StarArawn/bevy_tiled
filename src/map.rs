use anyhow::Result;
use bevy::{prelude::*, render::mesh::Indices, render::{
        draw::Visible,
        mesh::VertexAttributeValues,
        pipeline::PrimitiveTopology,
        pipeline::RenderPipeline,
        render_graph::base::MainPass,
    }, utils::{HashMap, HashSet}};
use bevy_reflect::TypeUuid;

use crate::{loader::TiledMapLoader, TileMapChunk, TILE_MAP_PIPELINE_HANDLE};
use glam::Vec2;
use std::{io::BufReader, path::Path};

pub use tiled::ObjectShape;

#[derive(Debug)]
pub struct Tile {
    pub tile_id: u32,
    pub pos: Vec2,
    pub vertex: Vec4,
    pub uv: Vec4,
    pub flip_d: bool,
    pub flip_h: bool,
    pub flip_v: bool,
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
    pub groups: Vec<ObjectGroup>,
    pub tile_size: Vec2,
    pub image_folder: std::path::PathBuf,
}

impl Map {
    pub fn project_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = tile_width * pos.x;
        let y = tile_height * pos.y;
        Vec2::new(x, -y)
    }
    pub fn unproject_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = pos.x / tile_width;
        let y = -(pos.y) / tile_height;
        Vec2::new(x, y)
    }
    pub fn project_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let x = (pos.x - pos.y) * tile_width / 2.0;
        let y = (pos.x + pos.y) * tile_height / 2.0;
        Vec2::new(x, -y)
    }
    pub fn unproject_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
        let half_width = tile_width / 2.0;
        let half_height = tile_height / 2.0;
        let x = ((pos.x / half_width) + (-(pos.y) / half_height)) / 2.0;
        let y = ((-(pos.y) / half_height) - (pos.x / half_width)) / 2.0;
        Vec2::new(x.round(), y.round())
    }
    pub fn center(&self, origin: Transform) -> Transform {
        let tile_size = Vec2::new(self.map.tile_width as f32, self.map.tile_height as f32);
        let map_center = Vec2::new(self.map.width as f32 / 2.0, self.map.height as f32 / 2.0);
        match self.map.orientation {
            tiled::Orientation::Orthogonal => {
                let center = Map::project_ortho(map_center, tile_size.x, tile_size.y);
                Transform::from_matrix(
                    origin.compute_matrix() * Mat4::from_translation(-center.extend(0.0)),
                )
            }
            tiled::Orientation::Isometric => {
                let center = Map::project_iso(map_center, tile_size.x, tile_size.y);
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
        let mut groups = Vec::new();

        // this only works if gids are uniques across all maps used - todo move into ObjectGroup?
        let mut tile_gids: HashMap<u32, u32> = Default::default();

        for tileset in &map.tilesets {
            for i in tileset.first_gid..(tileset.first_gid + tileset.tilecount.unwrap_or(1)) {
                tile_gids.insert(i, tileset.first_gid);
            }
        }

        let mut object_gids: HashSet<u32> = Default::default();
        for object_group in map.object_groups.iter() {
            // recursively creates objects in the groups:
            let tiled_o_g = ObjectGroup::new_with_tile_ids(object_group, &tile_gids);
            // keep track of which objects will need to have tiles loaded
            tiled_o_g.objects.iter().for_each(|o| {
                tile_gids.get(&o.gid).map(|first_gid| {
                    object_gids.insert(*first_gid);
                });
            });
            groups.push(tiled_o_g);
        }

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
                                                center.x - tile_width / 2.0,
                                                center.y - tile_height / 2.0,
                                            );

                                            let end = Vec2::new(
                                                center.x + tile_width / 2.0,
                                                center.y + tile_height / 2.0,
                                            );

                                            (start.x, end.x, start.y, end.y)
                                        }
                                        tiled::Orientation::Isometric => {
                                            let center = Map::project_iso(
                                                Vec2::new(lookup_x as f32, lookup_y as f32),
                                                tile_width,
                                                tile_height,
                                            );

                                            let start = Vec2::new(
                                                center.x - tile_width / 2.0,
                                                center.y - tile_height / 2.0,
                                            );

                                            let end = Vec2::new(
                                                center.x + tile_width / 2.0,
                                                center.y + tile_height / 2.0,
                                            );

                                            (start.x, end.x, start.y, end.y)
                                        }
                                        _ => {
                                            panic!("Unsupported orientation {:?}", map.orientation)
                                        }
                                    };

                                    // Calculate UV:
                                    let start_u: f32 = sprite_sheet_x / texture_width;
                                    let end_u: f32 =
                                        (sprite_sheet_x + tile_width) / texture_width;
                                    let start_v: f32 = sprite_sheet_y / texture_height;
                                    let end_v: f32 =
                                        (sprite_sheet_y + tile_height) / texture_height;

                                    Tile {
                                        tile_id: map_tile.gid,
                                        pos: Vec2::new(tile_x as f32, tile_y as f32),
                                        vertex: Vec4::new(start_x, start_y, end_x, end_y),
                                        uv: Vec4::new(start_u, start_v, end_u, end_v),
                                        flip_d: map_tile.flip_d,
                                        flip_h: map_tile.flip_h,
                                        flip_v: map_tile.flip_v,
                                    }
                                } else {
                                    // Empty tile
                                    Tile {
                                        tile_id: 0,
                                        pos: Vec2::new(tile_x as f32, tile_y as f32),
                                        vertex: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                        uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
                                        flip_d: false,
                                        flip_h: false,
                                        flip_v: false,
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
                            positions.push([tile.vertex.x, tile.vertex.y, 0.0]);
                            // X, Y + 1
                            positions.push([tile.vertex.x, tile.vertex.w, 0.0]);
                            // X + 1, Y + 1
                            positions.push([tile.vertex.z, tile.vertex.w, 0.0]);
                            // X + 1, Y
                            positions.push([tile.vertex.z, tile.vertex.y, 0.0]);

                            let mut next_uvs = [
                                // X, Y
                                [tile.uv.x, tile.uv.w],
                                // X, Y + 1
                                [tile.uv.x, tile.uv.y],
                                // X + 1, Y + 1
                                [tile.uv.z, tile.uv.y],
                                // X + 1, Y
                                [tile.uv.z, tile.uv.w]
                            ];
                            if tile.flip_d {
                                next_uvs.swap(0, 2);
                            }
                            if tile.flip_h {
                                next_uvs.reverse();
                            }
                            if tile.flip_v {
                                next_uvs.reverse();
                                next_uvs.swap(0, 2);
                                next_uvs.swap(1, 3);
                            }

                            next_uvs.iter().for_each(|uv| uvs.push(*uv));

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
            groups,
            tile_size,
            image_folder: asset_path.parent().unwrap().into(),
        };

        Ok(map)
    }
}

#[derive(Default)]
pub struct TiledMapCenter(pub bool);

#[derive(Debug)]
pub struct ObjectGroup {
    pub name: String,
    opacity: f32,
    pub visible: bool,
    pub objects: Vec<Object>,
}


impl ObjectGroup {
    pub fn new_with_tile_ids(inner: &tiled::ObjectGroup, tile_gids: &HashMap<u32, u32>) -> ObjectGroup {
        // println!("grp {}", inner.name.to_string());
        ObjectGroup {
            name: inner.name.to_string(),
            opacity: inner.opacity,
            visible: inner.visible,
            objects: inner.objects.iter().map(|obj| Object::new_with_tile_ids(obj, tile_gids)).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub shape: tiled::ObjectShape,
    pub position: Vec2,
    pub name: String,
    gid: u32, // sprite ID from tiled::Object
    tileset_gid: Option<u32>, // AKA first_gid
    sprite_index: Option<u32>,
}

impl Object {
    pub fn new(original_object: &tiled::Object) -> Object {
        // println!("obj {}", original_object.gid.to_string());
        Object {
            shape: original_object.shape.clone(),
            gid: original_object.gid, // zero for most non-tile objects
            tileset_gid: None,
            sprite_index: None,
            position: Vec2::new(original_object.x, original_object.y),
            name: original_object.name.clone()
        }
    }
    pub fn new_with_tile_ids(original_object: &tiled::Object, tile_gids: &HashMap<u32, u32>) -> Object {
        // println!("obj {}", original_object.gid.to_string());
        let mut o = Object::new(original_object);
        o.set_tile_ids(tile_gids);
        o
    }
    pub fn set_tile_ids(&mut self, tile_gids: &HashMap<u32, u32>) {
        self.tileset_gid = tile_gids.get(&self.gid).cloned();
        self.sprite_index = self.tileset_gid.map(|first_gid| &self.gid - first_gid );
    }

    pub fn transform_from_map(&self, map: &tiled::Map, tile_transform: &Transform, extra_scale: Option<Vec3>) -> Transform {
        // clone entire map transform
        let mut transform = tile_transform.clone();

        let map_tile_width = map.tile_width as f32;
        let map_tile_height = map.tile_height as f32;
        // offset transform position by 1/2 map tile
        transform.translation -= tile_transform.scale * Vec3::new(map_tile_width, -map_tile_height, 0.0) / 2.0;

        let map_orientation: tiled::Orientation = map.orientation;
        // replacing map Z with something far in front for objects -- should probably be configurable
        // transform.translation.z = 1000.0;
        let z_relative_to_map = 15.0; // used for a range of 5-25 above tile Z coordinate for items (max 20k map)
        match self.shape {
            tiled::ObjectShape::Rect { width, height } => {
                match map_orientation {
                    tiled::Orientation::Orthogonal => {
                        // object scale based on map scale, sometimes modified by passed-in scale from tile dimensions
                        transform.scale = extra_scale.unwrap_or(Vec3::new(1.0, 1.0, 1.0)) * transform.scale;
                        // apply map scale to object position
                        let mut center = Vec2::new(self.position.x + width / 2.0, -self.position.y + height / 2.0);
                        center *= tile_transform.scale.truncate();
                        // offset transform by object position
                        transform.translation += center.extend(z_relative_to_map - center.y / 2000.0 ); // only support up to 20k pixels maps
                        
                    }
                    // tiled::Orientation::Isometric => {
                    // }
                    _ => panic!("Unsupported orientation for object {:?}", map_orientation),
                }
            }
            tiled::ObjectShape::Ellipse { width: _ , height: _ } => {}
            tiled::ObjectShape::Polyline { points: _ } => {}
            tiled::ObjectShape::Polygon { points: _ } => {}
            tiled::ObjectShape::Point(_, _) => {}
        }
        transform

    }

    pub fn spawn<'a>(&self,
        commands: &'a mut Commands,
        texture_atlas: Option<&Handle<TextureAtlas>>,
        map: &tiled::Map,
        tile_map_transform: &Transform,
        debug_material: Handle<ColorMaterial>,
    ) -> &'a mut Commands {
        if let Some(texture_atlas) = texture_atlas {
            let sprite_index = self.sprite_index.expect("missing sprite index");
            let tileset_gid = self.tileset_gid.expect("missing tileset");
            
            // fetch tile for this object if it exists
            let object_tile_size = map.tilesets.iter().find(|ts| {
                ts.first_gid == tileset_gid
            }).map(|ts| Vec2::new(ts.tile_width as f32, ts.tile_height as f32));
            // object dimensions
            let dims = self.dimensions();
            // use object dimensions to determine extra scale (tile objects might have been resized)
            let extra_scale = if let (Some(dims), Some(size)) = (dims, object_tile_size) {
                Some((dims / size).extend(1.0))
            } else {
                None
            };

            commands.spawn(SpriteSheetBundle {
                    transform: self.transform_from_map(&map, tile_map_transform, extra_scale),
                    texture_atlas: texture_atlas.clone(),
                    sprite: TextureAtlasSprite {
                        index: sprite_index,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(self.clone())
        } else {
            println!("Spawning debug {:?}", self);
            // commands.spawn((self.map_transform(&map.map, &tile_map_transform, None), GlobalTransform::default()))
            let dimensions = self.dimensions().expect("Don't know how to handle object without dimensions");
            println!("dim {:?}", dimensions);
            let transform = self.transform_from_map(&map, &tile_map_transform, None);
            println!("tform {:?}", transform);
            commands
                // Debug box.
                .spawn(SpriteBundle {
                    material: debug_material,
                    sprite: Sprite::new(dimensions),
                    transform,
                    visible: Visible {
                        is_transparent: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(self.clone())
        }
    }

    pub fn dimensions(&self) -> Option<Vec2> {
        match self.shape {
            tiled::ObjectShape::Rect { width , height } |
            tiled::ObjectShape::Ellipse { width , height } => Some(Vec2::new(width, height)),
            tiled::ObjectShape::Polyline { points: _ } |
            tiled::ObjectShape::Polygon { points: _ } |
            tiled::ObjectShape::Point(_, _) => None,
        }
    }
}


/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapComponents {
    pub map_asset: Handle<Map>,
    pub materials: HashMap<u32, Handle<ColorMaterial>>,
    pub atlases: HashMap<u32, Handle<TextureAtlas>>,
    pub origin: Transform,
    pub center: TiledMapCenter,
}

impl Default for TiledMapComponents {
    fn default() -> Self {
        Self {
            map_asset: Handle::default(),
            materials: HashMap::default(),
            atlases: HashMap::default(),
            center: TiledMapCenter::default(),
            origin: Transform::default(),
        }
    }
}

#[derive(Default)]
pub struct MapResourceProviderState {
    pub map_event_reader: EventReader<AssetEvent<Map>>,
    pub created_layer_entities: HashMap<u32, Vec<Entity>>,
    pub created_object_entities: HashMap<u32, Vec<Entity>>,
}

#[derive(Bundle)]
pub struct ChunkComponents {
    pub chunk: TileMapChunk,
    pub main_pass: MainPass,
    pub material: Handle<ColorMaterial>,
    pub render_pipeline: RenderPipelines,
    pub visible: Visible,
    pub draw: Draw,
    pub mesh: Handle<Mesh>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ChunkComponents {
    fn default() -> Self {
        Self {
            chunk: TileMapChunk::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            draw: Default::default(),
            main_pass: MainPass,
            mesh: Handle::default(),
            material: Handle::default(),
            render_pipeline: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                TILE_MAP_PIPELINE_HANDLE.typed()
            )]),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

pub fn process_loaded_tile_maps(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut state: Local<MapResourceProviderState>,
    map_events: Res<Events<AssetEvent<Map>>>,
    mut ready_events: ResMut<Events<ObjectReadyEvent>>,
    mut maps: ResMut<Assets<Map>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<(
        Entity,
        &TiledMapCenter,
        &Handle<Map>,
        &mut HashMap<u32, Handle<ColorMaterial>>,
        &mut HashMap<u32, Handle<TextureAtlas>>,
        &Transform,
    )>,
) {
    let mut changed_maps = HashSet::<Handle<Map>>::default();
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

        for (_, _, _, mut materials_map, mut texture_atlas_map, _) in query.iter_mut() {
            for tileset in &map.map.tilesets {
                if !materials_map.contains_key(&tileset.first_gid) {
                    let texture_path = map
                        .image_folder
                        .join(tileset.images.first().unwrap().source.as_str());
                    let texture_handle = asset_server.load(texture_path);
                    materials_map.insert(tileset.first_gid, materials.add(texture_handle.clone().into()));

                    // only generate texture_atlas for tilesets used in objects
                    let object_gids: Vec<_> = map.groups.iter().flat_map(|og| og.objects.iter().map(|o| o.tileset_gid)).collect();
                    if object_gids.contains(&Some(tileset.first_gid)) {
                        // For simplicity use textureAtlasSprite for object layers
                        // these insertions should be limited to sprites referenced by objects
                        let tile_width = tileset.tile_width as f32;
                        let tile_height = tileset.tile_height as f32;
                        let image = tileset.images.first().unwrap();
                        let texture_width = image.width as f32;
                        let texture_height = image.height as f32;
                        let columns = (texture_width / tile_width).floor() as usize;
                        let rows = (texture_height / tile_height).floor() as usize;

                        let has_new = (0..(columns*rows) as u32).fold(false, |total, next | total || !texture_atlas_map.contains_key(&(tileset.first_gid + next)));
                        if has_new {
                            let atlas = TextureAtlas::from_grid(
                                texture_handle.clone(),
                                Vec2::new(tile_width, tile_height),
                                columns,
                                rows
                            );
                            let atlas_handle = texture_atlases.add(atlas);
                            for i in 0..(columns * rows) as u32 {
                                if texture_atlas_map.contains_key(&(tileset.first_gid + i)) { continue; }
                                // println!("insert: {}", tileset.first_gid + i);
                                texture_atlas_map.insert(tileset.first_gid + i, atlas_handle.clone());
                            }
                        }
                    }
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

    for (_, center, map_handle, materials_map, texture_atlas_map, origin) in query.iter_mut() {
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

                    state.created_layer_entities.get(&tileset_layer.tileset_guid).map(|entities| {
                        // println!("Despawning previously-created mesh for this chunk");
                        for entity in entities.iter() {
                            commands.despawn(*entity);
                        }
                    });
                    for (_, tileset_guid, mesh) in chunk_mesh_list.iter() {
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
                        }).current_entity().map(|new_entity| {
                            // println!("added created_entry after spawn");
                            state.created_layer_entities.entry(*tileset_guid)
                                .or_insert_with(|| Vec::new()).push(new_entity);
                        });
                    }
                }
            }

            for object_group in map.groups.iter() {
                for object in object_group.objects.iter() {
                    state.created_object_entities.get(&object.gid).map(|entities| {
                        // println!("Despawning previously-created object sprite");
                        for entity in entities.iter() {
                            commands.despawn(*entity);
                        }
                    });
                }
                if !object_group.visible {
                    continue;
                }
                // TODO: use object_group.name, opacity, colour (properties)
                for object in object_group.objects.iter() {
                    // println!("in object_group {}, object {}, grp: {}", object_group.name, &object.id, object.gid);
                    let atlas_handle = object.tileset_gid.and_then(|tileset_gid|
                        texture_atlas_map.get(&tileset_gid)
                    );

                    let debug_material = materials.add(Color::rgba(0.4, 0.4, 0.9, 0.5).into());
                    object.spawn(
                            commands,
                            atlas_handle,
                            &map.map,
                            &tile_map_transform,
                            debug_material,
                        )
                        .current_entity().map(|entity| {
                            // when done spawning, fire event
                            let evt = ObjectReadyEvent {
                                map_handle: map_handle.clone(),
                                entity: entity.clone()
                            };
                            ready_events.send(evt);

                            state.created_object_entities.entry(object.gid)
                                .or_insert_with(|| Vec::new()).push(entity);
                        });
                }
            }
        }
    }
}

pub struct ObjectReadyEvent {
    pub map_handle: Handle<Map>,
    pub entity: Entity
}