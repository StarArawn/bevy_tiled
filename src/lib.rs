use bevy::{asset::AssetServerSettings, prelude::*};

mod utils;
pub use utils::*;

mod loader;
mod map;
pub use map::*;
mod layers;
pub use layers::*;
mod objects;
pub use objects::*;

mod view;
pub use view::*;
/// Adds support for GLTF file loading to Apps
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

        app.add_asset::<map::Map>()
            .add_asset_loader(loader::TiledMapLoader::new(asset_folder))
            .add_event::<ObjectReadyEvent>()
            .add_event::<MapReadyEvent>()
            .add_system(process_loaded_tile_maps.system());

        let world = app.world_mut();
        add_tile_map_graph(world);
    }
}
