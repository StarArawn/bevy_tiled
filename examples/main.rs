use bevy::prelude::*;

fn main() {
    App::build()
    .add_default_plugins()
    .add_plugin(bevy_tiled::TiledMapPlugin)
    .add_startup_system(setup.system())
    .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("assets/buch-outdoor.png").unwrap();
    commands
        .spawn(bevy_tiled::TiledMapComponents {
            map_asset: asset_server.load("assets/map.tmx").unwrap(),
            material: materials.add(texture_handle.into()),
            ..Default::default()
        })
        .spawn(Camera2dComponents::default());
}