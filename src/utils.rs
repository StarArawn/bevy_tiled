use bevy::math::Vec2;

pub fn project_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
    let x = tile_width * pos.x;
    let y = tile_height * pos.y;
    Vec2::new(x, -y)
}
pub fn unproject_ortho(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
    let x = pos.x / tile_width;
    let y = -(pos.y) / tile_height;
    Vec2::new(x, y)
}
pub fn project_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
    let x = (pos.x - pos.y) * tile_width / 2.0;
    let y = (pos.x + pos.y) * tile_height / 2.0;
    Vec2::new(x, -y)
}
pub fn unproject_iso(pos: Vec2, tile_width: f32, tile_height: f32) -> Vec2 {
    let half_width = tile_width / 2.0;
    let half_height = tile_height / 2.0;
    let x = ((pos.x / half_width) + (-(pos.y) / half_height)) / 2.0;
    let y = ((-(pos.y) / half_height) - (pos.x / half_width)) / 2.0;
    Vec2::new(x.round(), y.round())
}
