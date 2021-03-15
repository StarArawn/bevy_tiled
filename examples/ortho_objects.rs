use bevy::prelude::*;
use bevy_tiled_prototype::{DebugConfig, Object, TiledMapCenter};

// this example demonstrates debugging objects

const SCALE: f32 = 2.0;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system(toggle_debug.system())
        .add_startup_system(setup.system())
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(bevy_tiled_prototype::TiledMapComponents {
            map_asset: asset_server.load("ortho-map.tmx"),
            center: TiledMapCenter(true),
            origin: Transform::from_scale(Vec3::new(SCALE, SCALE, 1.0)),
            debug_config: DebugConfig {
                enabled: true,
                material: None,

            },
            ..Default::default()
        })
        .spawn(Camera2dBundle::default());
}

fn toggle_debug(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Visible, With<Object>>,
) {
    for mut visible in query.iter_mut() {
        if keyboard_input.just_released(KeyCode::Space) {
            visible.is_visible = !visible.is_visible;
        }
    }
}
