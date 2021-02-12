use crate::map::Map;
use anyhow::Result;
use bevy::{asset::{AssetLoader, AssetPath, LoadContext, LoadedAsset}, utils::BoxedFuture};

#[derive(Default)]
pub struct TiledMapLoader;

impl TiledMapLoader {
    pub fn remove_tile_flags(tile: u32) -> u32 {
        let tile = tile & !ALL_FLIP_FLAGS;
        tile
    }
}

const FLIPPED_HORIZONTALLY_FLAG: u32 = 0x80000000;
const FLIPPED_VERTICALLY_FLAG: u32 = 0x40000000;
const FLIPPED_DIAGONALLY_FLAG: u32 = 0x20000000;
const ALL_FLIP_FLAGS: u32 =
    FLIPPED_HORIZONTALLY_FLAG | FLIPPED_VERTICALLY_FLAG | FLIPPED_DIAGONALLY_FLAG;

impl AssetLoader for TiledMapLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let path = load_context.path();
            let mut map = Map::try_from_bytes(path, bytes.into())?;
            let dependencies = map.asset_dependencies.drain(..)
                .map(|image_path| {
                    // add tileset to dependencies
                    AssetPath::new(image_path, None)
                }).collect();
            let loaded_asset = LoadedAsset::new(map);
            load_context.set_default_asset(
                loaded_asset.with_dependencies(dependencies)
            );
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tmx"];
        EXTENSIONS
    }
}
