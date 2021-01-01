use bevy::{
    prelude::*,
    render::{
        pipeline::{
            BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
            CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, PipelineDescriptor,
            RasterizationStateDescriptor, StencilStateDescriptor, StencilStateFaceDescriptor,
        },
        render_graph::{base, RenderGraph, RenderResourcesNode},
        shader::{ShaderStage, ShaderStages},
        texture::TextureFormat,
    },
    reflect::TypeUuid,
};

use crate::TileMapChunk;

pub const TILE_MAP_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 4129645945969645246);

pub fn build_tile_map_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                #[cfg(feature = "web")]
                include_str!("tile_map_webgl2.vert"),
                #[cfg(not(feature = "web"))]
                include_str!("tile_map.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                #[cfg(feature = "web")]
                include_str!("tile_map_webgl2.frag"),
                #[cfg(not(feature = "web"))]
                include_str!("tile_map.frag"),
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
