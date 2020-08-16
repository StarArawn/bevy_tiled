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
) {
    commands.spawn(bevy_tiled::map::TiledMapComponents {
        map_asset: asset_server.load("assets/map.tmx").unwrap(),
        ..Default::default()
    });
}