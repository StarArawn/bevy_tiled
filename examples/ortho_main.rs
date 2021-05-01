use bevy::{app::CoreStage::PreUpdate, prelude::*, render::camera::Camera};
use bevy_tiled_prototype::prelude::*;

fn main() {
    env_logger::Builder::from_default_env()
    .filter_level(log::LevelFilter::Info)
    .init();

    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_tiled_prototype::TiledMapPlugin)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        // Needs to run before rendering to set texture atlas filter for new tiles- would be better to use states
        .add_system_to_stage(PreUpdate, set_texture_filters_to_nearest.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
    
    commands.spawn_bundle(TiledMapBundle {
        map_asset: asset_server.load("ortho-map.tmx"),
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

// demo of https://github.com/StarArawn/bevy_tiled/issues/47#issuecomment-817126515
//  Would be cleaner to put this in a separate AppState, transitioning out after textures loaded
fn set_texture_filters_to_nearest(
    mut map_ready_events: EventReader<MapReadyEvent>,
    mut textures: ResMut<Assets<Texture>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    // quick and dirty, run this for all textures every time a map is created/modified
    if map_ready_events.iter().count() > 0 {
        for (_, atlas) in texture_atlases.iter() {
            if let Some(texture) = textures.get_mut(atlas.texture.clone()) {
                texture.sampler.min_filter = bevy::render::texture::FilterMode::Nearest;
            }
        }
    }
}
