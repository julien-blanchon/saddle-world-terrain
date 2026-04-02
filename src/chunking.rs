use bevy::{math::Vec2, prelude::*, reflect::Reflect};

use crate::config::TerrainConfig;

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerrainChunkAddress(pub IVec2);

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerrainChunkKey {
    pub coord: IVec2,
    pub lod: u8,
}

pub fn terrain_chunk_for_local(local_xz: Vec2, config: &TerrainConfig) -> Option<IVec2> {
    if local_xz.x < 0.0
        || local_xz.y < 0.0
        || local_xz.x > config.size.x
        || local_xz.y > config.size.y
    {
        return None;
    }

    let max_coord = config.chunk_dimensions().as_ivec2() - IVec2::ONE;
    Some(IVec2::new(
        ((local_xz.x / config.chunk_size.x.max(f32::EPSILON)).floor() as i32).min(max_coord.x),
        ((local_xz.y / config.chunk_size.y.max(f32::EPSILON)).floor() as i32).min(max_coord.y),
    ))
}

pub fn chunk_origin_local(coord: IVec2, config: &TerrainConfig) -> Vec2 {
    Vec2::new(
        coord.x as f32 * config.chunk_size.x,
        coord.y as f32 * config.chunk_size.y,
    )
}

pub fn chunk_extent_local(coord: IVec2, config: &TerrainConfig) -> Vec2 {
    let origin = chunk_origin_local(coord, config);
    Vec2::new(
        config.chunk_size.x.min((config.size.x - origin.x).max(0.0)),
        config.chunk_size.y.min((config.size.y - origin.y).max(0.0)),
    )
}

pub fn chunk_center_local(coord: IVec2, config: &TerrainConfig) -> Vec2 {
    let origin = chunk_origin_local(coord, config);
    origin + chunk_extent_local(coord, config) * 0.5
}

pub fn chunk_is_valid(coord: IVec2, config: &TerrainConfig) -> bool {
    let dims = config.chunk_dimensions().as_ivec2();
    coord.x >= 0 && coord.y >= 0 && coord.x < dims.x && coord.y < dims.y
}

pub fn chunk_coords_in_radius(
    focus_local: Vec2,
    radius: f32,
    config: &TerrainConfig,
) -> Vec<IVec2> {
    let chunk_size = config.chunk_size.max(Vec2::splat(0.001));
    let min = ((focus_local - Vec2::splat(radius)) / chunk_size)
        .floor()
        .as_ivec2()
        - IVec2::ONE;
    let max = ((focus_local + Vec2::splat(radius)) / chunk_size)
        .ceil()
        .as_ivec2()
        + IVec2::ONE;

    let mut coords = Vec::new();
    for y in min.y..=max.y {
        for x in min.x..=max.x {
            let coord = IVec2::new(x, y);
            if !chunk_is_valid(coord, config) {
                continue;
            }
            let center = chunk_center_local(coord, config);
            let half = chunk_extent_local(coord, config) * 0.5;
            let dx = (focus_local.x - center.x).abs() - half.x;
            let dy = (focus_local.y - center.y).abs() - half.y;
            let outside = Vec2::new(dx.max(0.0), dy.max(0.0));
            if outside.length() <= radius {
                coords.push(coord);
            }
        }
    }

    coords
}

#[cfg(test)]
#[path = "chunking_tests.rs"]
mod tests;
