use bevy::{prelude::*, render::texture::FilterMode};

// demo of https://github.com/StarArawn/bevy_tiled/issues/47#issuecomment-817126515
//  Would be cleaner to put this in a separate AppState, transitioning out after textures loaded
pub fn set_texture_filters_to_nearest(
    mut texture_events: EventReader<AssetEvent<Texture>>,
    mut textures: ResMut<Assets<Texture>>,
) {
    // quick and dirty, run this for all textures every time a map is created/modified
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(mut texture) = textures.get_mut(handle){
                    texture.sampler.min_filter = FilterMode::Nearest;
                }
            }
            _ => ()
        }
    }
}
