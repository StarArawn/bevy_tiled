use bevy::{
    prelude::*,
    render::{
        pipeline::{
            DynamicBinding, PipelineSpecialization, RenderPipeline,
        },
        render_graph::base::MainPass,
    },
};

use glam::{Vec2, Vec4};
use std::collections::{HashMap, HashSet};
use crate::TILE_MAP_PIPELINE_HANDLE;

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
    pub meshes: Vec<Mesh>,
    pub layers: Vec<Layer>,
    pub tile_size: Vec2,
}

/// A bundle of tiled map entities.
#[derive(Bundle)]
pub struct TiledMapComponents {
    pub map_asset: Handle<Map>,
    pub main_pass: MainPass,
    pub material: Handle<ColorMaterial>,
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
            material: Handle::default(),
            render_pipeline: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                TILE_MAP_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0,
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

#[derive(Default)]
pub struct MapResourceProviderState {
    map_event_reader: EventReader<AssetEvent<Map>>,
}

pub fn process_loaded_tile_maps(
    mut commands: Commands,
    mut state: Local<MapResourceProviderState>,
    map_events: Res<Events<AssetEvent<Map>>>,
    mut maps: ResMut<Assets<Map>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(Entity, &Handle<Map>)>,
) {
    let mut changed_maps = HashSet::<Handle<Map>>::new();
    for event in state.map_event_reader.iter(&map_events) {
        match event {
            AssetEvent::Created { handle } => {
                changed_maps.insert(*handle);
            }
            AssetEvent::Modified { handle } => {
                changed_maps.insert(*handle);
            }
            AssetEvent::Removed { handle } => {
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_maps.remove(handle);
            }
        }
    }

    let mut new_meshes = HashMap::<&Handle<Map>, Vec<Handle<Mesh>>>::new();
    for changed_map in changed_maps.iter() {
        let map = maps.get_mut(changed_map).unwrap();
        for mesh in map.meshes.drain(0..map.meshes.len()) {
            let handle = meshes.add(mesh);

            if new_meshes.contains_key(changed_map) {
                let mesh_list = new_meshes.get_mut(changed_map).unwrap();
                mesh_list.push(handle);
            } else {
                let mut mesh_list = Vec::new();
                mesh_list.push(handle);
                new_meshes.insert(changed_map, mesh_list);
            }
        }
    }

    for (e, map_handle) in &mut query.iter() {
        if new_meshes.contains_key(map_handle) {
            let mesh_list = new_meshes.get_mut(map_handle).unwrap();
            for mesh in mesh_list.drain(0..mesh_list.len()) {
                // Transfer meshes to entity..
                commands.insert_one(e, mesh);
            }
        }
    }
}
