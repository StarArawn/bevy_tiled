use bevy::prelude::*;

mod loader;
pub mod map;
pub use map::*;

/// Adds support for GLTF file loading to Apps
#[derive(Default)]
pub struct TiledMapPlugin;

impl Plugin for TiledMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_asset::<map::Map>()
            .add_asset_loader::<map::Map, loader::TiledMapLoader>();
    }
}
