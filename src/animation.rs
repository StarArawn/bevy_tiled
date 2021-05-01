use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::Tile;

/// Information specific to the current frame in the animation. 
#[derive(Clone)]
pub struct Frame {
    /// Tile id.
    pub tile_id: u32,
    /// Duration until next frame.
    pub duration: f64,
}

/// Information about the tiles animation state.
pub struct Animation {
    /// Frame info.
    pub frames: Vec<Frame>,
    /// The current frame.
    pub current_frame: usize,
    pub last_update: f64,
}

pub fn update(
    time: Res<Time>,
    mut query: Query<(&mut Tile, &mut Animation)>,
) {
    let current_time = time.seconds_since_startup();
    for (mut tile, mut animation) in query.iter_mut() {
        let frame = animation.frames[animation.current_frame].clone();
        if (current_time - animation.last_update) > frame.duration {
            animation.current_frame += 1;
            tile.texture_index = frame.tile_id;
            if animation.current_frame > animation.frames.len() - 1 {
                animation.current_frame = 0;
            }
            animation.last_update = current_time;
        }
    }
}
