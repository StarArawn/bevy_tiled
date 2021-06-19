use bevy::prelude::*;
use bevy_tiled_prototype::{DebugConfig, Object, TiledMapCenter};

// this example demonstrates debugging objects. Hit spacebar to toggle them

const SCALE: f32 = 4.0;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system(toggle_debug.system())
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(bevy_tiled_prototype::TiledMapBundle {
        map_asset: asset_server.load("ortho-debug.tmx"),
        center: TiledMapCenter(true),
        origin: Transform::from_scale(Vec3::new(SCALE, SCALE, 1.0)),
        debug_config: DebugConfig {
            enabled: true,
            material: None,
        },
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn toggle_debug(keyboard_input: Res<Input<KeyCode>>, mut query: Query<&mut Visible, With<Object>>) {
    for mut visible in query.iter_mut() {
        if keyboard_input.just_released(KeyCode::Space) {
            visible.is_visible = !visible.is_visible;
        }
    }
}
