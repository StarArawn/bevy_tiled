use bevy::{prelude::*, render::camera::Camera};
use bevy_tiled_prototype::TiledMapCenter;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(bevy_tiled_prototype::TiledMapComponents {
            map_asset: asset_server.load("ortho-map.tmx"),
            center: TiledMapCenter(true),
            origin: Transform::from_scale(Vec3::new(4.0, 4.0, 1.0)),
            ..Default::default()
        })
        .spawn(Camera2dBundle::default());
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    for (_, mut transform) in query.iter_mut() {
        let mut direction = Vec3::zero();
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
