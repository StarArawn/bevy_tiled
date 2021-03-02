# bevy_tiled
A plugin for rendering tiled maps. Specfically maps from the tiled editor which can be found here:
https://www.mapeditor.org/

Feel free to use this code as a reference for your own custom tile mapping solution as well.

## Toplevel Entity Support

For now, TiledMapBundle is just a configuration object. If you would like access to a toplevel entity that can be transformed, pass into the configuration:

    parent_option: Some(entity)

Then, both chunks and objects will be inserted as children to this entity, which will be tagged with MapRoot. This API is likely to change.
## Object Layer Support

Object layers are now supported. They will be skipped if not visible. Individual objects that are invisible
will be spawned with is_visible set to false. You may pass into the configuration object:

    debug_config: DebugConfig { enabled: true, material: None }

to show a color mesh for objects that have no tile sprite. This is only supported for rects and points (small squares) at this time.

Objects within tiles are not currently supported.

## Events

There are two events that you can listen for when you spawn a map.

- ObjectReadyEvent fires when an object has been spawned.
- MapReadyEvent fires when all objects and layers have been spawned.

These both have:
    pub map_entity_option: Option<Entity>,
    pub map_handle: Handle<Map>,

and ObjectReadyEvent additionally includes the entity for the object itself

## Hot reload

Limited support for hot reload is provided. Old entities are removed based on the asset handles (for now).

    asset_server.watch_for_changes().expect("watch for changes failed");

Then when you save your map, it should update in the application.

## Top-needed features

  * support for iso maps
  * support for objects in tiles
  * support for embedded images
