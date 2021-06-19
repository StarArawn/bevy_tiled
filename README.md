# bevy_tiled

Welcome to bevy_tiled!

This is a plugin for rendering tiled maps. Specfically, maps from the "Tiled" editor which can be found here:

https://www.mapeditor.org/

Feel free to use this code as a reference for your own custom tile mapping solution as well.

## Bevy Versions

The `main` branch of this repository targets Bevy 0.5. When using bevy_tiled, please make sure your version of Bevy matches the version referenced by this library. There are versions for 0.4 and 0.3 as well.

If you were using framp's fork for Bevy 0.4, it is currently merged in and mirrored by the [v0.1.0-bevy-0.4](https://github.com/StarArawn/bevy_tiled/releases/tag/v0.1.0-bevy-0.4) release. For a few more bugfixes, you can point to the [v0.1.1-bevy-0.4](https://github.com/StarArawn/bevy_tiled/releases/tag/v0.1.1-bevy-0.4) tag. Object support in Bevy 0.4 is usable, and is an [open PR](https://github.com/StarArawn/bevy_tiled/pull/41), targetting the `bevy-0.4` branch and published as the [v0.2.1-rc1-bevy-0.4](https://github.com/StarArawn/bevy_tiled/releases/tag/v0.2.1-rc1-bevy-0.4) tag.

For those of you relying on the old bevy_tiled, you will want to point your Cargo.toml to the `bevy-0.3` branch or the [v0.1.0-bevy-0.3](https://github.com/StarArawn/bevy_tiled/releases/tag/v0.1.0-bevy-0.3) tag. There is a small fix there for map positioning, and it is otherwise unchanged.

## Basic Setup

Follow the Rust [Getting Started](https://www.rust-lang.org/learn/get-started) and make sure you have `rustc` compiler and `cargo` build system available on your machine.

Clone this repo and try running some of the [examples](/examples):

```sh
# Runs the orthographic tile map example
cargo run --example ortho_main
```

```sh
# Runs the isometric tile map example
cargo run --example iso_main
```

In these examples, you should be able to use the wasd keys to pan across the maps. You can follow a similar pattern in your own Bevy project. For more information, follow the [Bevy Setup] guide.

# Features
## Toplevel Entity Support

For now, TiledMapBundle is just a configuration object. If you would like access to a toplevel entity that can be transformed, pass into the configuration:

    parent_option: Some(entity)

Then, both chunks and objects will be inserted as children to this entity, which will be tagged with MapRoot. This API is likely to change, but we have an [example](/examples/parent_entity.rs) for how it currently works.
## Object Group Support

Object Groups are now supported for orthographic maps. They will be skipped if not visible.
Individual objects that are invisible will be spawned with is_visible set to false. 
### Example

To see objects and debugging in action, run the `ortho_debug` example which will enable debug viewing of objects.
Use the spacebar to toggle objects. Rects within tiles are also shown.

```sh
# Runs the debug/objects example
cargo run --example ortho_debug
```

### Debug visibility

You may pass into the configuration object:

    debug_config: DebugConfig { enabled: true, material: None }

to show a color mesh for objects that have no tile sprite. `material: None` will use the default material.
This is only supported for rects at this time. Some other objects will show up as small squares until we improve support.

### Embedded Objects in Tiles

The Tiled editor allows you to specify collision regions within tiles. If your map has objects used as sprites, embedded rectangular objects will be included. These are separately spawned as object children to simplify positioning. 

Objects spawned from tile map layers is currently not supported.
## Events

There are two events that you can listen for when you spawn a map.

- ObjectReadyEvent fires when an object has been spawned.
- MapReadyEvent fires when all objects and layers have been spawned.

These both have:
    pub map_entity_option: Option<Entity>,
    pub map_handle: Handle<Map>,

and ObjectReadyEvent additionally includes `entity: Entity` for what the object was spawned as.

## Hot reload

Limited support for hot reload is provided. Old entities are removed based on the asset handles (for now).

    asset_server.watch_for_changes().expect("watch for changes failed");

Then when you save your map, it should update in the application.

## WASM and bevy_webgl2

Use `default-features=false, features=["web"]` in your project's `Cargo.toml`. Tiled maps using Zstd compression are not supported.

## Contributing

We are so happy to be seeing more PRs! Keep 'em coming!
### Top-requested features

  * better support for isometric maps
    * ~tile layer offsets~ Thanks Dint!
    * isometric projection for rectangular objects
  * support for collision objects in tiles
    * ~objectgroups in objects~ implemented recently for rects!
    * objectgroups within tiles
  * support for embedded images in Tmx files
  * support for flipping sprite objects, and their associated collision regions
  * support for animations
  * support for tint color and opacity

