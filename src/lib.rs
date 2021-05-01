use bevy::{asset::AssetServerSettings, prelude::*};
use bevy_ecs_tilemap::prelude::*;
use tiled_map::{MapReadyEvent, process_loaded_tile_maps};

mod layers;
mod loader;
mod tiled_map;
mod animation;

#[derive(Default)]
pub struct TiledMapPlugin;

impl Plugin for TiledMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let asset_folder = app
            .world()
            .get_resource::<AssetServerSettings>()
            .unwrap()
            .asset_folder
            .clone();

        app
            .add_plugin(TileMapPlugin)
            .add_asset::<tiled_map::TiledMap>()
            .add_asset_loader(loader::TiledMapLoader::new(asset_folder))
            .add_event::<MapReadyEvent>()
            .add_system(process_loaded_tile_maps.system())
            .add_system(animation::update.system());
    }
}

pub mod prelude {
    pub use crate::TiledMapPlugin;
    pub use crate::tiled_map::{TiledMapBundle, MapReadyEvent};
    pub use crate::animation::{Animation, Frame};
}