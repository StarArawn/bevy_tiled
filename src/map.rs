use crate::{ChunkBundle, TilesetLayer, TileMapChunk, MapLayer, objects::ObjectGroup, utils::project_iso, utils::project_ortho};
use anyhow::Result;
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::mesh::Indices,
    render::{mesh::VertexAttributeValues, pipeline::PrimitiveTopology},
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

// An asset for maps
#[derive(Debug, TypeUuid)]
#[uuid = "5f6fbac8-3f52-424e-a928-561667fea074"]
pub struct Map {
    pub map: tiled::Map,
    pub meshes: Vec<(u32, u32, Mesh)>,
    pub layers: Vec<MapLayer>,
    pub groups: Vec<ObjectGroup>,
    pub tile_size: Vec2,
    pub image_folder: std::path::PathBuf,
    pub asset_dependencies: Vec<PathBuf>,
}

impl Map {
    pub fn center(&self, origin: Transform) -> Transform {
        let tile_size = Vec2::new(self.map.tile_width as f32, self.map.tile_height as f32);
        let map_center = Vec2::new(self.map.width as f32 / 2.0, self.map.height as f32 / 2.0);
        match self.map.orientation {
            tiled::Orientation::Orthogonal => {
                let center = project_ortho(map_center, tile_size.x, tile_size.y);
                Transform::from_matrix(
                    origin.compute_matrix() * Mat4::from_translation(-center.extend(0.0)),
                )
            }
            tiled::Orientation::Isometric => {
                let center = project_iso(map_center, tile_size.x, tile_size.y);
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

        let tile_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);
        let image_folder: PathBuf = asset_path.parent().unwrap().into();
        let mut asset_dependencies = Vec::new();

        for layer in map.layers.iter() {
            if !layer.visible {
                continue;
            }
            let mut tileset_layers = Vec::new();

            for tileset in map.tilesets.iter() {
                let tile_path = image_folder.join(tileset.images.first().unwrap().source.as_str());
                asset_dependencies.push(tile_path);

                tileset_layers.push(TilesetLayer::new(&map, &layer, &tileset));
            }

            let layer = MapLayer { tileset_layers };
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
                                [tile.uv.z, tile.uv.w],
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
            image_folder,
            asset_dependencies,
        };

        Ok(map)
    }
}

#[derive(Default)]
pub struct TiledMapCenter(pub bool);

pub struct MapRoot; // used so consuming application can query for parent

pub struct DebugConfig {
    pub enabled: bool,
    pub material: Option<Handle<ColorMaterial>>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            material: Default::default(),
        }
    }
}

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapBundle {
    pub map_asset: Handle<Map>,
    pub parent_option: Option<Entity>,
    pub materials: HashMap<u32, Handle<ColorMaterial>>,
    pub atlases: HashMap<u32, Handle<TextureAtlas>>,
    pub origin: Transform,
    pub center: TiledMapCenter,
    pub debug_config: DebugConfig,
    pub created_entities: CreatedMapEntities,
}

impl Default for TiledMapBundle {
    fn default() -> Self {
        Self {
            map_asset: Handle::default(),
            parent_option: None,
            materials: HashMap::default(),
            atlases: HashMap::default(),
            center: TiledMapCenter::default(),
            origin: Transform::default(),
            debug_config: Default::default(),
            created_entities: Default::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct CreatedMapEntities {
    // maps layer id and tileset_gid to mesh entities
    created_layer_entities: HashMap<(usize, u32), Vec<Entity>>,
    // maps object guid to texture atlas sprite entity
    created_object_entities: HashMap<u32, Vec<Entity>>,
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut map_events: EventReader<AssetEvent<Map>>,
    mut ready_events: EventWriter<ObjectReadyEvent>,
    mut map_ready_events: EventWriter<MapReadyEvent>,
    mut maps: ResMut<Assets<Map>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<(
        Entity,
        &TiledMapCenter,
        &Handle<Map>,
        &Option<Entity>,
        &mut HashMap<u32, Handle<ColorMaterial>>,
        &mut HashMap<u32, Handle<TextureAtlas>>,
        &Transform,
        &mut DebugConfig,
        &mut CreatedMapEntities,
    )>,
) {
    let mut changed_maps = HashSet::<Handle<Map>>::default();
    for event in map_events.iter() {
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

        for (_, _, map_handle, _, mut materials_map, mut texture_atlas_map, _, _, _) in
            query.iter_mut()
        {
            // only deal with currently changed map
            if map_handle != changed_map {
                continue;
            }

            for tileset in &map.map.tilesets {
                if !materials_map.contains_key(&tileset.first_gid) {
                    let texture_path = map
                        .image_folder
                        .join(tileset.images.first().unwrap().source.as_str());
                    let texture_handle = asset_server.load(texture_path);
                    materials_map.insert(
                        tileset.first_gid,
                        materials.add(texture_handle.clone().into()),
                    );

                    // only generate texture_atlas for tilesets used in objects
                    let object_gids: Vec<_> = map
                        .groups
                        .iter()
                        .flat_map(|og| og.objects.iter().map(|o| o.tileset_gid))
                        .collect();
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

                        let has_new = (0..(columns * rows) as u32).fold(false, |total, next| {
                            total || !texture_atlas_map.contains_key(&(tileset.first_gid + next))
                        });
                        if has_new {
                            let atlas = TextureAtlas::from_grid(
                                texture_handle.clone(),
                                Vec2::new(tile_width, tile_height),
                                columns,
                                rows,
                            );
                            let atlas_handle = texture_atlases.add(atlas);
                            for i in 0..(columns * rows) as u32 {
                                if texture_atlas_map.contains_key(&(tileset.first_gid + i)) {
                                    continue;
                                }
                                // println!("insert: {}", tileset.first_gid + i);
                                texture_atlas_map
                                    .insert(tileset.first_gid + i, atlas_handle.clone());
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

    for (
        _,
        center,
        map_handle,
        optional_parent,
        materials_map,
        texture_atlas_map,
        origin,
        mut debug_config,
        mut created_entities,
    ) in query.iter_mut()
    {
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

                    // removing entities consumes the record of created entities
                    created_entities
                        .created_layer_entities
                        .remove(&(layer_id, tileset_layer.tileset_guid))
                        .map(|entities| {
                            // println!("Despawning previously-created mesh for this chunk");
                            for entity in entities.iter() {
                                // println!("calling despawn on {:?}", entity);
                                commands.entity(*entity).despawn();
                            }
                        });
                    let mut chunk_entities: Vec<Entity> = Default::default();

                    for (_, tileset_guid, mesh) in chunk_mesh_list.iter() {
                        // TODO: Sadly bevy doesn't support multiple meshes on a single entity with multiple materials.
                        // Change this once it does.

                        // Instead for now spawn a new entity per chunk.
                        let chunk_entity = commands
                            .spawn_bundle(ChunkBundle {
                                chunk: TileMapChunk {
                                    // TODO: Support more layers here..
                                    layer_id: layer_id as f32,
                                },
                                material: material_handle.clone(),
                                mesh: mesh.clone(),
                                map_parent: map_handle.clone(),
                                transform: tile_map_transform.clone(),
                                ..Default::default()
                            })
                            .id();

                        // println!("added created_entry after spawn");
                        created_entities
                            .created_layer_entities
                            .entry((layer_id, *tileset_guid))
                            .or_insert_with(|| Vec::new())
                            .push(chunk_entity);
                        chunk_entities.push(chunk_entity);
                    }
                    // if parent was passed in add children and mark it as MapRoot (temp until map bundle returns real entity)
                    if let Some(parent_entity) = optional_parent {
                        commands
                            .entity(parent_entity.clone())
                            .push_children(&chunk_entities)
                            .insert(MapRoot);
                    }
                }
            }

            if debug_config.enabled && debug_config.material.is_none() {
                debug_config.material =
                    Some(materials.add(ColorMaterial::from(Color::rgba(0.4, 0.4, 0.9, 0.5))));
            }
            for object_group in map.groups.iter() {
                for object in object_group.objects.iter() {
                    created_entities
                        .created_object_entities
                        .remove(&object.gid)
                        .map(|entities| {
                            // println!("Despawning previously-created object sprite");
                            for entity in entities.iter() {
                                // println!("calling despawn on {:?}", entity);
                                commands.entity(*entity).despawn();
                            }
                        });
                }
                if !object_group.visible {
                    continue;
                }

                let mut object_entities: Vec<Entity> = Default::default();

                // TODO: use object_group.name, opacity, colour (properties)
                for object in object_group.objects.iter() {
                    // println!("in object_group {}, object {:?}, grp: {}", object_group.name, &object.tileset_gid, object.gid);
                    let atlas_handle = object
                        .tileset_gid
                        .and_then(|tileset_gid| texture_atlas_map.get(&tileset_gid));

                    let entity = object
                        .spawn(
                            &mut commands,
                            atlas_handle,
                            &map.map,
                            map_handle.clone(),
                            &tile_map_transform,
                            &debug_config,
                        )
                        .id();
                    // when done spawning, fire event
                    let evt = ObjectReadyEvent {
                        entity: entity.clone(),
                        map_handle: map_handle.clone(),
                        map_entity_option: optional_parent.clone(),
                    };
                    ready_events.send(evt);

                    created_entities
                        .created_object_entities
                        .entry(object.gid)
                        .or_insert_with(|| Vec::new())
                        .push(entity);
                    object_entities.push(entity);
                }

                // if parent was passed in add children
                if let Some(parent_entity) = optional_parent {
                    commands
                        .entity(parent_entity.clone())
                        .push_children(&object_entities);
                }
            }
            let evt = MapReadyEvent {
                map_handle: map_handle.clone(),
                map_entity_option: optional_parent.clone(),
            };
            map_ready_events.send(evt);
        }
    }
}

// events fired when entity has been created

pub struct ObjectReadyEvent {
    pub entity: Entity,
    pub map_handle: Handle<Map>,
    pub map_entity_option: Option<Entity>,
}

pub struct MapReadyEvent {
    pub map_handle: Handle<Map>,
    pub map_entity_option: Option<Entity>,
}
