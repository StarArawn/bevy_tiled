use bevy::{prelude::*, render::camera::Camera};

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
            map_asset: asset_server.load("assets/ortho-map.tmx").unwrap(),
            center: true,
            ..Default::default()
        })
        .spawn(Camera2dComponents::default());
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Camera, &mut Translation)>,
) {
    for (_, mut translation) in &mut query.iter() {
        let mut direction = Vec3::zero();
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

        translation.0 += time.delta_seconds * direction * 1000.0;
    }
}
