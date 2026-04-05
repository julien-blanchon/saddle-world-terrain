use bevy::{math::Vec2, prelude::*, reflect::Reflect};

use crate::material::TerrainMaterialProfile;

#[derive(Reflect, Clone, Debug)]
pub struct TerrainStreamingConfig {
    pub visual_radius: f32,
    pub collider_radius: f32,
    pub max_builds_per_frame: usize,
}

impl Default for TerrainStreamingConfig {
    fn default() -> Self {
        Self {
            visual_radius: 320.0,
            collider_radius: 128.0,
            max_builds_per_frame: 6,
        }
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainLodConfig {
    pub lod_count: u8,
    pub near_distance: f32,
    pub distance_multiplier: f32,
    pub hysteresis: f32,
    pub minimum_vertex_resolution: u32,
}

impl Default for TerrainLodConfig {
    fn default() -> Self {
        Self {
            lod_count: 5,
            near_distance: 48.0,
            distance_multiplier: 2.0,
            hysteresis: 8.0,
            minimum_vertex_resolution: 8,
        }
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainColliderConfig {
    pub enabled: bool,
    pub resolution_divisor: u32,
}

impl Default for TerrainColliderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            resolution_divisor: 2,
        }
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainCacheConfig {
    pub max_entries: usize,
}

impl Default for TerrainCacheConfig {
    fn default() -> Self {
        Self { max_entries: 256 }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct TerrainConfig {
    pub size: Vec2,
    pub chunk_size: Vec2,
    pub vertex_resolution: u32,
    pub height_scale: f32,
    pub height_offset: f32,
    pub skirt_depth: f32,
    pub normal_sample_distance: f32,
    pub lod: TerrainLodConfig,
    pub streaming: TerrainStreamingConfig,
    pub collider: TerrainColliderConfig,
    pub cache: TerrainCacheConfig,
    pub material: TerrainMaterialProfile,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(512.0, 512.0),
            chunk_size: Vec2::new(32.0, 32.0),
            vertex_resolution: 64,
            height_scale: 72.0,
            height_offset: 0.0,
            skirt_depth: 4.0,
            normal_sample_distance: 1.5,
            lod: TerrainLodConfig::default(),
            streaming: TerrainStreamingConfig::default(),
            collider: TerrainColliderConfig::default(),
            cache: TerrainCacheConfig::default(),
            material: TerrainMaterialProfile::default(),
        }
    }
}

impl TerrainConfig {
    pub fn terrain_bounds(&self) -> Vec2 {
        self.size.max(Vec2::splat(1.0))
    }

    pub fn chunk_dimensions(&self) -> UVec2 {
        UVec2::new(
            (self.size.x / self.chunk_size.x.max(f32::EPSILON)).ceil() as u32,
            (self.size.y / self.chunk_size.y.max(f32::EPSILON)).ceil() as u32,
        )
    }

    pub fn local_to_uv(&self, local_xz: Vec2) -> Option<Vec2> {
        if local_xz.x < 0.0
            || local_xz.y < 0.0
            || local_xz.x > self.size.x
            || local_xz.y > self.size.y
        {
            return None;
        }

        Some(Vec2::new(
            if self.size.x <= f32::EPSILON {
                0.0
            } else {
                local_xz.x / self.size.x
            },
            if self.size.y <= f32::EPSILON {
                0.0
            } else {
                local_xz.y / self.size.y
            },
        ))
    }

    pub fn uv_to_local(&self, uv: Vec2) -> Vec2 {
        Vec2::new(uv.x * self.size.x, uv.y * self.size.y)
    }

    /// Returns the total number of chunks that would cover this terrain.
    pub fn total_chunk_count(&self) -> u32 {
        let dims = self.chunk_dimensions();
        dims.x * dims.y
    }

    /// Estimates the vertex count for a single chunk at the given LOD level.
    pub fn chunk_vertex_count(&self, lod: u8) -> u32 {
        let resolution = crate::meshing::resolution_for_lod(self, lod);
        (resolution + 1) * (resolution + 1)
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn with_chunk_size(mut self, chunk_size: Vec2) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    pub fn with_vertex_resolution(mut self, resolution: u32) -> Self {
        self.vertex_resolution = resolution;
        self
    }

    pub fn with_height_scale(mut self, scale: f32) -> Self {
        self.height_scale = scale;
        self
    }

    pub fn with_height_offset(mut self, offset: f32) -> Self {
        self.height_offset = offset;
        self
    }

    pub fn with_lod(mut self, lod: TerrainLodConfig) -> Self {
        self.lod = lod;
        self
    }

    pub fn with_streaming(mut self, streaming: TerrainStreamingConfig) -> Self {
        self.streaming = streaming;
        self
    }

    pub fn with_collider(mut self, collider: TerrainColliderConfig) -> Self {
        self.collider = collider;
        self
    }

    pub fn with_material(mut self, material: TerrainMaterialProfile) -> Self {
        self.material = material;
        self
    }
}
