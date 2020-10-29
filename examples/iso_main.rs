use bevy::{prelude::*, render::camera::Camera};
use bevy_tiled::TiledMapCenter;

fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(bevy_tiled::TiledMapPlugin)
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(bevy_tiled::TiledMapComponents {
            map_asset: asset_server.load("assets/iso-map.tmx").unwrap(),
            center: TiledMapCenter(true),
            origin: Transform::from_non_uniform_scale(Vec3::new(4.0, 4.0, 1.0)),
            ..Default::default()
        })
        .spawn(Camera2dComponents::default());
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    for (_, mut transform) in &mut query.iter() {
        let mut direction = Vec3::zero();
        let scale = transform.value().x_axis().x();
        
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
            transform.set_scale(scale + 0.1);
        }

        if keyboard_input.pressed(KeyCode::X)  && scale > 1.1 {
            transform.set_scale(scale - 0.1);
        }

        let translation = transform.translation();

        transform.set_translation(translation + time.delta_seconds * direction * 1000.0);
    }
}
