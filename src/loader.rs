use std::path::{Path, PathBuf};

use crate::tiled_map::TiledMap;
use anyhow::Result;
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, LoadedAsset},
    utils::BoxedFuture,
};
pub struct TiledMapLoader {
    asset_folder: PathBuf,
}

impl TiledMapLoader {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        TiledMapLoader {
            asset_folder: path.as_ref().to_path_buf(),
        }
    }
}

impl AssetLoader for TiledMapLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let path = load_context.path();
            let mut map = TiledMap::try_from_bytes(self.asset_folder.as_path(), path, bytes.into())?;
            let dependencies = map
                .asset_dependencies
                .drain(..)
                .map(|image_path| {
                    // add tileset to dependencies
                    AssetPath::new(image_path, None)
                })
                .collect();
            let loaded_asset = LoadedAsset::new(map);
            load_context.set_default_asset(loaded_asset.with_dependencies(dependencies));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tmx"];
        EXTENSIONS
    }
}
