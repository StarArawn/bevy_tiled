use bevy::{prelude::*, render::camera::Camera};
use bevy_tiled_prototype::{TiledMapCenter};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(bevy_tiled_prototype::TiledMapBundle {
        map_asset: asset_server.load("images-map.tmx"),
        center: TiledMapCenter(true),
        origin: Transform::from_scale(Vec3::new(0.75, 0.75, 1.0)),
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    for (_, mut transform) in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        let scale = transform.scale.x;

        if keyboard_input.pressed(KeyCode::A) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::D) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::W) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::S) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Z) {
            let scale = scale + 0.1;
            transform.scale = Vec3::new(scale, scale, scale);
        }

        if keyboard_input.pressed(KeyCode::X) && scale > 1.1 {
            let scale = scale - 0.1;
            transform.scale = Vec3::new(scale, scale, scale);
        }

        transform.translation += time.delta_seconds() * direction * 1000.;
    }
}
