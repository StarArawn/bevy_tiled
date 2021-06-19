use bevy::{ecs::system::EntityCommands, prelude::*, utils::HashMap};

use crate::{DebugConfig, Map, loader::TiledMapLoader};

#[derive(Debug)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub objects: Vec<Object>,
}

impl ObjectGroup {
    pub fn new_with_tile_ids(
        inner: &tiled::ObjectGroup,
        tileset_by_gid: &HashMap<u32, tiled::Tileset>,
    ) -> ObjectGroup {
        // println!("grp {}", inner.name.to_string());
        ObjectGroup {
            name: inner.name.to_string(),
            opacity: inner.opacity,
            visible: inner.visible,
            objects: inner
                .objects
                .iter()
                .map(|obj| Object::new_with_tile_ids(obj, tileset_by_gid))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub shape: tiled::ObjectShape,
    pub props: tiled::Properties,
    pub position: Vec2,
    pub size: Vec2,
    pub name: String,
    pub obj_type: String,
    pub visible: bool,
    pub gid: u32,                 // sprite ID from tiled::Object
    pub tileset_gid: Option<u32>, // AKA first_gid
    pub sprite_index: Option<u32>,
    pub tile_scale: Option<Vec2>,
}

impl Object {
    pub fn new(original_object: &tiled::Object) -> Object {
        // println!("obj {} {}", original_object.name, original_object.visible.to_string());
        Object {
            shape: original_object.shape.clone(),
            props: original_object.properties.clone(),
            gid: TiledMapLoader::remove_tile_flags(original_object.gid), // zero for most non-tile objects
            visible: original_object.visible,
            tileset_gid: None,
            sprite_index: None,
            tile_scale: None,
            position: Vec2::new(original_object.x, original_object.y),
            size: Vec2::new(original_object.width, original_object.height),
            name: original_object.name.clone(),
            obj_type: original_object.obj_type.clone(),
        }
    }

    pub fn is_shape(&self) -> bool {
        self.tileset_gid.is_none()
    }

    pub fn new_with_tile_ids(
        original_object: &tiled::Object,
        tileset_by_gid: &HashMap<u32, tiled::Tileset>,
    ) -> Object {
        // println!("obj {}", original_object.gid.to_string());
        let mut o = Object::new(original_object);
        o.set_tile_ids(tileset_by_gid);
        o
    }
    pub fn set_tile_ids(&mut self, tileset_by_gid: &HashMap<u32, tiled::Tileset>) {
        let tileset = tileset_by_gid.get(&self.gid);
        self.tileset_gid = tileset.map(|ts| ts.first_gid.clone());
        self.sprite_index = self.tileset_gid.map(|first_gid| &self.gid - first_gid);

        // compute scale from tileset size to new height/widtho
        if let &(Some(ts), Some(dims)) = &(tileset, self.dimensions()) {
            self.tile_scale = Some(Vec2::new(
                dims.x / ts.tile_width as f32,
                dims.y / ts.tile_height as f32,
            ));
        }

    }

    pub fn transform_from_map(
        &self,
        map: &tiled::Map,
        map_transform: &Transform,
        tile_scale: Option<Vec3>,
    ) -> Transform {
        // tile scale being None means this is not a tile object

        // clone entire map transform
        let mut transform = map_transform.clone();

        //// this was made obsolete by Kurble's branch changes
        // let map_tile_width = map.tile_width as f32;
        // let map_tile_height = map.tile_height as f32;
        //// offset transform position by 1/2 map tile
        // transform.translation -= map_transform.scale * Vec3::new(map_tile_width, -map_tile_height, 0.0) / 2.0;

        let map_orientation: tiled::Orientation = map.orientation;
        // replacing map Z with something far in front for objects -- should probably be configurable
        // transform.translation.z = 1000.0;
        let z_relative_to_map = 15.0; // used for a range of 5-25 above tile Z coordinate for items (max 20k map)
        match self.shape {
            tiled::ObjectShape::Rect { width, height } => {
                match map_orientation {
                    tiled::Orientation::Orthogonal => {
                        let mut center_offset = Vec2::new(self.position.x, -self.position.y);
                        match tile_scale {
                            None => {
                                // shape object x/y represent top left corner
                                center_offset += Vec2::new(width, -height) / 2.0;
                            }
                            Some(tile_scale) => {
                                // tile object x/y represents bottom left corner
                                center_offset += Vec2::new(width, height) / 2.0;
                                // tile object scale based on map scale and passed-in scale from image dimensions
                                transform.scale = tile_scale * transform.scale;
                            }
                        }
                        // apply map scale to object position, if this is a tile
                        center_offset *= map_transform.scale.truncate();
                        // offset transform by object position
                        transform.translation +=
                            center_offset.extend(z_relative_to_map - center_offset.y / 2000.0);
                        // ^ HACK only support up to 20k pixels maps, TODO: configure in API
                    }
                    // tiled::Orientation::Isometric => {

                    // }
                    _ => panic!("Sorry, {:?} objects aren't supported -- please hide this object layer for now.", map_orientation),
                }
            }
            tiled::ObjectShape::Ellipse {
                width: _,
                height: _,
            } => {}
            tiled::ObjectShape::Polyline { points: _ } => {}
            tiled::ObjectShape::Polygon { points: _ } => {}
            tiled::ObjectShape::Point(_, _) => {}
        }
        transform
    }

    pub fn spawn<'a, 'b>(
        &self,
        commands: &'b mut Commands<'a>,
        texture_atlas: Option<&Handle<TextureAtlas>>,
        map: &tiled::Map,
        map_handle: Handle<Map>,
        tile_map_transform: &Transform,
        debug_config: &DebugConfig,
    ) -> EntityCommands<'a, 'b> {

        // object dimensions
        let dimensions = self
            .dimensions()
            .expect("Don't know how to handle object without dimensions");

        let mut new_entity_commands = if let Some(texture_atlas) = texture_atlas {
            let sprite_index = self.sprite_index.expect("missing sprite index");
            let tileset_gid = self.tileset_gid.expect("missing tileset");

            // fetch tile for this object if it exists
            let object_tileset = map
                .tilesets
                .iter()
                .find(|ts| ts.first_gid == tileset_gid);
            let object_tile_size = object_tileset
                .map(|ts| Vec2::new(ts.tile_width as f32, ts.tile_height as f32));
            let object_tile = object_tileset.and_then(|ts| ts
                .tiles.iter().find(|&tile| tile.id + ts.first_gid == self.gid)
            );

            // use object dimensions and tile size to determine extra scale to apply for tile objects
            let tile_scale = if let Some(size) = object_tile_size {
                Some((dimensions / size).extend(1.0))
            } else {
                None
            };
            let mut entity_commands = commands.spawn_bundle(SpriteSheetBundle {
                transform: self.transform_from_map(&map, tile_map_transform, tile_scale),
                texture_atlas: texture_atlas.clone(),
                sprite: TextureAtlasSprite {
                    index: sprite_index,
                    ..Default::default()
                },
                visible: Visible {
                    is_visible: self.visible,
                    is_transparent: true,
                    ..Default::default()
                },
                ..Default::default()
            });
            // spawn embedded objects as children
            let tile_size = object_tile_size.expect("child object needs parent to have a size");
            object_tile.map(|tile| {
                entity_commands.with_children(|builder| {
                    //builder.spawn
                    for obj_grp in &tile.objectgroup {
                        for obj in &obj_grp.objects {
                            let marker_object = Object::new(obj);

                            let mut embedded_object_transform = Transform::from_scale(Vec3::splat(1.0));
                            embedded_object_transform.translation = Vec3::new(
                                obj.x -(tile_size.x - obj.width) / 2.0, //(self.size.y / tile_scale.unwrap_or(Vec3::ONE).y - obj.height) / 2.0 - obj.y,
                                -obj.y + (tile_size.y - obj.height) / 2.0, //(self.size.y / tile_scale.unwrap_or(Vec3::ONE).y - obj.height) / 2.0 - obj.y,
                                0.0001
                            );
                            //println!("{:?},{:?}", tile_scale.unwrap(), embedded_object_transform.translation );
                            let size  = marker_object.dimensions().expect("embedded object needs dimension");

                            builder.spawn_bundle(
                                SpriteBundle {
                                    material: debug_config
                                        .material
                                        .clone()
                                        .unwrap_or_else(|| Handle::<ColorMaterial>::default()),
                                    sprite: Sprite::new(size),
                                    transform:  embedded_object_transform,
                                    visible: Visible {
                                        is_visible: debug_config.enabled,
                                        is_transparent: true,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }
                            ).insert(marker_object);
                        }
                    }
                });
            });
            entity_commands
        } else {
            // commands.spawn((self.map_transform(&map.map, &tile_map_transform, None), GlobalTransform::default()))
            let transform = self.transform_from_map(&map, &tile_map_transform, None);
            commands
                // Debug box.
                .spawn_bundle(SpriteBundle {
                    material: debug_config
                        .material
                        .clone()
                        .unwrap_or_else(|| Handle::<ColorMaterial>::default()),
                    sprite: Sprite::new(dimensions),
                    transform,
                    visible: Visible {
                        is_visible: debug_config.enabled,
                        is_transparent: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
        };

        new_entity_commands.insert_bundle((map_handle, self.clone()));
        new_entity_commands
    }

    pub fn dimensions(&self) -> Option<Vec2> {
        match self.shape {
            tiled::ObjectShape::Rect { width, height }
            | tiled::ObjectShape::Ellipse { width, height } => Some(Vec2::new(width, height)),
            tiled::ObjectShape::Polyline { points: _ }
            | tiled::ObjectShape::Polygon { points: _ }
            | tiled::ObjectShape::Point(_, _) => Some(Vec2::splat(1.0)),
        }
    }
}
