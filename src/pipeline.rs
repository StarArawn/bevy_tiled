use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        pipeline::{
            BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction,
            DepthBiasState, DepthStencilState, PipelineDescriptor, StencilFaceState, StencilState,
        },
        render_graph::{base, RenderGraph, RenderResourcesNode},
        shader::{ShaderStage, ShaderStages},
        texture::TextureFormat,
    },
};

use crate::TileMapChunk;

pub const TILE_MAP_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 4129645945969645246);

pub fn build_tile_map_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            clamp_depth: false,
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendState {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendState {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                if cfg!(feature = "web") {
                    include_str!("tile_map_webgl2.vert")
                } else {
                    include_str!("tile_map.vert")
                },
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                if cfg!(feature = "web") {
                    include_str!("tile_map_webgl2.frag")
                } else {
                    include_str!("tile_map.frag")
                },
            ))),
        })
    }
}

pub mod node {
    pub const TILE_MAP_CHUNK: &'static str = "tile_map_chunk";
}

pub trait TileMapRenderGraphBuilder {
    fn add_tile_map_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl TileMapRenderGraphBuilder for RenderGraph {
    fn add_tile_map_graph(&mut self, resources: &Resources) -> &mut Self {
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set_untracked(
            TILE_MAP_PIPELINE_HANDLE,
            build_tile_map_pipeline(&mut shaders),
        );
        self.add_system_node(
            node::TILE_MAP_CHUNK,
            RenderResourcesNode::<TileMapChunk>::new(true),
        );
        self.add_node_edge(node::TILE_MAP_CHUNK, base::node::MAIN_PASS)
            .unwrap();
        self
    }
}
