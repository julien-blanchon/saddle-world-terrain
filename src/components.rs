use std::sync::Arc;

use bevy::prelude::*;

use crate::{TerrainChunkKey, config::TerrainConfig, source::TerrainSource};

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainRoot;

#[derive(Component)]
pub struct TerrainSourceHandle(pub Arc<dyn TerrainSource>);

impl TerrainSourceHandle {
    pub fn new(source: impl TerrainSource) -> Self {
        Self(Arc::new(source))
    }

    pub fn from_arc(source: Arc<dyn TerrainSource>) -> Self {
        Self(source)
    }
}

#[derive(Bundle)]
pub struct TerrainBundle {
    name: Name,
    root: TerrainRoot,
    source: TerrainSourceHandle,
    config: TerrainConfig,
    stats: TerrainRootStats,
    spatial: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    inherited_visibility: InheritedVisibility,
    view_visibility: ViewVisibility,
}

impl TerrainBundle {
    pub fn new(source: impl TerrainSource, config: TerrainConfig) -> Self {
        Self::with_name("Terrain Root", TerrainSourceHandle::new(source), config)
    }

    pub fn from_arc(source: Arc<dyn TerrainSource>, config: TerrainConfig) -> Self {
        Self::with_name(
            "Terrain Root",
            TerrainSourceHandle::from_arc(source),
            config,
        )
    }

    pub fn with_name(
        name: impl Into<String>,
        source: TerrainSourceHandle,
        config: TerrainConfig,
    ) -> Self {
        Self {
            name: Name::new(name.into()),
            root: TerrainRoot,
            source,
            config,
            stats: TerrainRootStats::default(),
            spatial: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::VISIBLE,
            view_visibility: ViewVisibility::default(),
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainChunk {
    pub terrain: Entity,
    pub key: TerrainChunkKey,
}

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Debug)]
pub enum TerrainChunkState {
    Queued,
    Building,
    Ready,
    Failed,
}

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainChunkBounds {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainRootStats {
    pub active_visual_chunks: u32,
    pub active_collider_chunks: u32,
    pub pending_chunks: u32,
    pub ready_chunks: u32,
    pub max_visible_lod: u8,
    pub focus_count: u32,
    pub cache_hits: u64,
}

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainFocus {
    pub terrain: Option<Entity>,
    pub visual_radius_bias: f32,
    pub collider_radius_bias: f32,
}

#[derive(Reflect, Clone, Copy, Debug, Default)]
pub struct TerrainFocusPoint {
    pub terrain: Option<Entity>,
    pub position: Vec3,
    pub visual_radius_bias: f32,
    pub collider_radius_bias: f32,
}

#[derive(Resource, Reflect, Clone, Debug, Default)]
#[reflect(Resource, Clone, Debug)]
pub struct TerrainFocusPoints(pub Vec<TerrainFocusPoint>);

#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainProbe {
    pub terrain: Option<Entity>,
    pub world_offset: Vec3,
}

#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component, Clone, Debug)]
pub struct TerrainProbeSample {
    pub height: f32,
    pub world_position: Vec3,
    pub normal: Vec3,
    pub slope_degrees: f32,
    pub dominant_layer: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct TerrainColliderPatch {
    pub origin: Vec2,
    pub extent: Vec2,
    pub dimensions: UVec2,
    pub heights: Arc<[f32]>,
}

#[derive(Component, Clone, Debug, Deref)]
pub struct TerrainColliderData(pub Arc<TerrainColliderPatch>);

#[derive(Message, Clone, Copy, Debug)]
pub struct TerrainChunkReady {
    pub terrain: Entity,
    pub chunk: Entity,
    pub key: TerrainChunkKey,
    pub from_cache: bool,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct TerrainChunkRemoved {
    pub terrain: Entity,
    pub chunk: Entity,
    pub key: TerrainChunkKey,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct TerrainColliderReady {
    pub terrain: Entity,
    pub chunk: Entity,
    pub key: TerrainChunkKey,
}
