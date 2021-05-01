use bevy::prelude::*;
use bevy_tiled_prototype::prelude::*;

// this example demonstrates moving the map mesh entities using
// the MapRoot marker on a passed-in parent element

const SCALE: f32 = 0.25;

#[derive(Debug, Default)]
struct MovementData {
    transform: Transform,
}

struct MapRoot;

fn main() {
    App::build()
        .insert_resource(MovementData::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_system(process_input.system())
        .add_system(move_parent_entity.system())
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // let's pass in a parent to append map tiles to
    let parent = commands
        .spawn_bundle((
            Transform {
                ..Default::default()
            },
            GlobalTransform {
                ..Default::default()
            },
        ))
        .insert(MapRoot)
        .id();

    commands.entity(parent).with_children(|child_builder| {
        child_builder.spawn_bundle(TiledMapBundle {
            map_asset: asset_server.load("ortho-map.tmx"),
        ..Default::default()
        });
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn process_input(
    mut movement_data: ResMut<MovementData>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left) {
        direction -= Vec3::new(SCALE, 0.0, 0.0);
    }

    if keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right) {
        direction += Vec3::new(SCALE, 0.0, 0.0);
    }

    if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
        direction += Vec3::new(0.0, SCALE, 0.0);
    }

    if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
        direction -= Vec3::new(0.0, SCALE, 0.0);
    }

    movement_data.transform.translation += time.delta_seconds() * direction * 1000.;
    movement_data.transform.scale = Vec3::splat(SCALE);
}

fn move_parent_entity(
    movement_data: Res<MovementData>,
    mut query: Query<(&MapRoot, &mut Transform)>,
) {
    for (_, mut transform) in query.iter_mut() {
        transform.clone_from(&movement_data.transform);
    }
}
