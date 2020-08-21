use bevy::core::Byteable;
use bevy::render::renderer::{RenderResource, RenderResources};

#[repr(C)]
#[derive(Default, RenderResources, RenderResource)]
#[render_resources(from_self)]
pub struct TileMapChunk {
    pub layer_id: f32,
}

// SAFE: sprite is repr(C) and only consists of byteables
unsafe impl Byteable for TileMapChunk {}
