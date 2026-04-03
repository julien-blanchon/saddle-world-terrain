use bevy::{math::Mat3, prelude::*};

use crate::{
    config::TerrainConfig,
    material::{TerrainLayerBlend, evaluate_layer_blend},
    source::TerrainSource,
};

#[derive(Clone, Debug, Default)]
pub struct TerrainSample {
    pub local_position: Vec3,
    pub world_position: Vec3,
    pub height: f32,
    pub local_height: f32,
    pub normal: Vec3,
    pub slope_degrees: f32,
    pub layers: TerrainLayerBlend,
}

pub fn sample_terrain(
    world_position: Vec3,
    terrain_transform: &GlobalTransform,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Option<TerrainSample> {
    let local = terrain_transform
        .affine()
        .inverse()
        .transform_point3(world_position);
    let local_xz = Vec2::new(local.x, local.z);
    let uv = config.local_to_uv(local_xz)?;
    if source.sample_hole(uv) >= 0.5 {
        return None;
    }

    let normalized_height = source.sample_height(uv);
    let local_height = config.height_offset + normalized_height * config.height_scale;
    let local_position = Vec3::new(local_xz.x, local_height, local_xz.y);
    let world_position = terrain_transform.transform_point(local_position);
    let normal_local = sample_normal_local(local_xz, config, source);
    let slope_degrees = normal_local.angle_between(Vec3::Y).to_degrees();
    let explicit_weights = sample_explicit_weights(uv, source);
    let layers = evaluate_layer_blend(
        &config.material,
        normalized_height,
        slope_degrees,
        &explicit_weights,
    );

    let normal_matrix = Mat3::from(terrain_transform.affine().matrix3)
        .inverse()
        .transpose();
    let normal_world = (normal_matrix * normal_local).normalize_or_zero();

    Some(TerrainSample {
        local_position,
        world_position,
        height: world_position.y,
        local_height,
        normal: normal_world,
        slope_degrees,
        layers,
    })
}

pub fn sample_height(
    world_position: Vec3,
    terrain_transform: &GlobalTransform,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Option<f32> {
    sample_terrain(world_position, terrain_transform, config, source).map(|sample| sample.height)
}

pub fn sample_normal(
    world_position: Vec3,
    terrain_transform: &GlobalTransform,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Option<Vec3> {
    sample_terrain(world_position, terrain_transform, config, source).map(|sample| sample.normal)
}

pub fn sample_layer_weights(
    world_position: Vec3,
    terrain_transform: &GlobalTransform,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Option<TerrainLayerBlend> {
    sample_terrain(world_position, terrain_transform, config, source).map(|sample| sample.layers)
}

pub(crate) fn sample_height_local(
    local_xz: Vec2,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Option<f32> {
    let uv = config.local_to_uv(local_xz)?;
    if source.sample_hole(uv) >= 0.5 {
        return None;
    }
    Some(config.height_offset + source.sample_height(uv) * config.height_scale)
}

pub(crate) fn sample_normal_local(
    local_xz: Vec2,
    config: &TerrainConfig,
    source: &dyn TerrainSource,
) -> Vec3 {
    let dx = config.normal_sample_distance.max(0.001);
    let dz = dx;
    let left = sample_height_local(local_xz - Vec2::X * dx, config, source)
        .or_else(|| sample_height_local(local_xz, config, source))
        .unwrap_or(config.height_offset);
    let right = sample_height_local(local_xz + Vec2::X * dx, config, source)
        .or_else(|| sample_height_local(local_xz, config, source))
        .unwrap_or(config.height_offset);
    let down = sample_height_local(local_xz - Vec2::Y * dz, config, source)
        .or_else(|| sample_height_local(local_xz, config, source))
        .unwrap_or(config.height_offset);
    let up = sample_height_local(local_xz + Vec2::Y * dz, config, source)
        .or_else(|| sample_height_local(local_xz, config, source))
        .unwrap_or(config.height_offset);

    Vec3::new(left - right, 2.0 * dx.max(dz), down - up).normalize_or_zero()
}

pub(crate) fn sample_explicit_weights(uv: Vec2, source: &dyn TerrainSource) -> Vec<f32> {
    let channel_count = source.explicit_weight_channel_count();
    let mut weights = Vec::with_capacity(channel_count);
    for channel in 0..channel_count {
        weights.push(source.sample_explicit_weight(channel, uv));
    }
    weights
}

#[cfg(test)]
#[path = "sampling_tests.rs"]
mod tests;
