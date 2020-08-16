use bevy::{
    render::{render_graph::base::MainPass, pipeline::{PipelineSpecialization, RenderPipeline, DynamicBinding}},
    prelude::*,
    sprite::{QUAD_HANDLE, SPRITE_PIPELINE_HANDLE}
};

use glam::{Vec2, Vec4};

#[derive(Debug)]
pub struct Tile {
    pub tile_id: u32,
    pub pos: Vec2,
    pub vertex: Vec4,
    pub uv: Vec4,
}

#[derive(Debug)]
pub struct Chunk {
    pub tiles: Vec<Vec<Tile>>,
}

#[derive(Debug)]
pub struct Layer {
    pub chunks: Vec<Vec<Chunk>>,
}

// An asset for maps
#[derive(Debug)]
pub struct Map {
    pub map: tiled::Map,
    pub layers: Vec<Layer>,
    pub tile_size: Vec2,
}

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapComponents {
    pub map_asset: Handle<Map>,
    pub mesh: Handle<Mesh>,
    pub main_pass: MainPass,
    pub render_pipeline: RenderPipelines,
    pub draw: Draw,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for TiledMapComponents {
    fn default() -> Self {
        Self {
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            map_asset: Handle::default(),
            mesh: QUAD_HANDLE,
            render_pipeline: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                SPRITE_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0,
                        },
                        // Sprite
                        DynamicBinding {
                            bind_group: 2,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}