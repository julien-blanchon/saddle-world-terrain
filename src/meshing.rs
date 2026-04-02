use std::sync::Arc;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::{
    TerrainColliderPatch, TerrainDebugColorMode, TerrainLayerBlend,
    chunking::{TerrainChunkKey, chunk_extent_local, chunk_origin_local},
    config::TerrainConfig,
    sampling::{sample_explicit_weights, sample_height_local, sample_normal_local},
    source::TerrainSource,
};

#[derive(Clone, Debug)]
pub struct TerrainBuildArtifact {
    pub mesh: Mesh,
    pub bounds: crate::TerrainChunkBounds,
    pub collider_patch: Option<Arc<TerrainColliderPatch>>,
}

pub fn build_chunk_artifact(
    source: &dyn TerrainSource,
    config: &TerrainConfig,
    key: TerrainChunkKey,
    color_mode: TerrainDebugColorMode,
) -> Result<TerrainBuildArtifact, String> {
    let resolution = resolution_for_lod(config, key.lod);
    let sample_origin = chunk_origin_local(key.coord, config);
    let extent = chunk_extent_local(key.coord, config);
    if extent.x <= f32::EPSILON || extent.y <= f32::EPSILON {
        return Err("chunk extent is empty".into());
    }

    let columns = resolution + 1;
    let rows = resolution + 1;

    let mut positions = Vec::<[f32; 3]>::with_capacity((columns * rows) as usize);
    let mut normals = Vec::<[f32; 3]>::with_capacity((columns * rows) as usize);
    let mut uvs = Vec::<[f32; 2]>::with_capacity((columns * rows) as usize);
    let mut colors = Vec::<[f32; 4]>::with_capacity((columns * rows) as usize);
    let mut indices = Vec::<u32>::new();

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for row in 0..rows {
        for column in 0..columns {
            let tx = column as f32 / resolution as f32;
            let tz = row as f32 / resolution as f32;
            let chunk_local_xz = extent * Vec2::new(tx, tz);
            let sample_local_xz = sample_origin + chunk_local_xz;
            let uv = config.local_to_uv(sample_local_xz).unwrap_or(Vec2::ZERO);
            let normalized_height = source.sample_height(uv);
            let height = config.height_offset + normalized_height * config.height_scale;
            let local = Vec3::new(chunk_local_xz.x, height, chunk_local_xz.y);
            let normal = sample_normal_local(sample_local_xz, config, source);
            let slope = normal.angle_between(Vec3::Y).to_degrees();
            let explicit_weights = sample_explicit_weights(uv, source);
            let layer_blend = crate::material::evaluate_layer_blend(
                &config.material,
                normalized_height,
                slope,
                &explicit_weights,
            );

            positions.push(local.to_array());
            normals.push(normal.to_array());
            uvs.push(uv.to_array());
            colors.push(
                sample_color(&layer_blend, normalized_height, slope, key.lod, color_mode)
                    .to_linear()
                    .to_f32_array(),
            );
            min = min.min(local);
            max = max.max(local);
        }
    }

    for row in 0..resolution {
        for column in 0..resolution {
            let i0 = row * columns + column;
            let i1 = i0 + 1;
            let i2 = i0 + columns;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    append_skirts(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut colors,
        &mut indices,
        columns,
        rows,
        config.skirt_depth.max(0.0),
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    Ok(TerrainBuildArtifact {
        mesh,
        bounds: crate::TerrainChunkBounds { min, max },
        collider_patch: if config.collider.enabled {
            Some(Arc::new(build_collider_patch(source, config, key)))
        } else {
            None
        },
    })
}

pub fn resolution_for_lod(config: &TerrainConfig, lod: u8) -> u32 {
    let mut resolution = config.vertex_resolution.max(2);
    for _ in 0..lod {
        resolution = (resolution / 2).max(config.lod.minimum_vertex_resolution.max(2));
    }
    resolution
}

fn sample_color(
    blend: &TerrainLayerBlend,
    height_normalized: f32,
    slope_degrees: f32,
    lod: u8,
    color_mode: TerrainDebugColorMode,
) -> Color {
    match color_mode {
        TerrainDebugColorMode::Natural => blend.color,
        TerrainDebugColorMode::ByLod => match lod {
            0 => Color::srgb(0.28, 0.78, 0.36),
            1 => Color::srgb(0.28, 0.62, 0.88),
            2 => Color::srgb(0.96, 0.76, 0.22),
            3 => Color::srgb(0.92, 0.46, 0.20),
            _ => Color::srgb(0.68, 0.44, 0.86),
        },
        TerrainDebugColorMode::ByChunkState => blend.color,
        TerrainDebugColorMode::ByLayerDominance => blend.dominant_color,
        TerrainDebugColorMode::BySlopeBand => {
            let altitude = height_normalized.clamp(0.0, 1.0);
            let slope = (slope_degrees / 90.0).clamp(0.0, 1.0);
            Color::srgb(altitude, 1.0 - slope, slope)
        }
    }
}

fn append_skirts(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    columns: u32,
    rows: u32,
    skirt_depth: f32,
) {
    if skirt_depth <= f32::EPSILON {
        return;
    }

    let mut add_edge = |edge: Vec<u32>| {
        let skirt_start = positions.len() as u32;
        for &index in &edge {
            let mut position = Vec3::from_array(positions[index as usize]);
            position.y -= skirt_depth;
            positions.push(position.to_array());
            normals.push(normals[index as usize]);
            uvs.push(uvs[index as usize]);
            colors.push(colors[index as usize]);
        }

        for i in 0..edge.len().saturating_sub(1) {
            let top_a = edge[i];
            let top_b = edge[i + 1];
            let bottom_a = skirt_start + i as u32;
            let bottom_b = skirt_start + i as u32 + 1;
            indices.extend_from_slice(&[top_a, bottom_a, top_b, top_b, bottom_a, bottom_b]);
        }
    };

    let top = (0..columns).collect::<Vec<_>>();
    let bottom = (0..columns)
        .map(|column| (rows - 1) * columns + column)
        .collect::<Vec<_>>();
    let left = (0..rows).map(|row| row * columns).collect::<Vec<_>>();
    let right = (0..rows)
        .map(|row| row * columns + (columns - 1))
        .collect::<Vec<_>>();

    add_edge(top);
    add_edge(bottom);
    add_edge(left);
    add_edge(right);
}

fn build_collider_patch(
    source: &dyn TerrainSource,
    config: &TerrainConfig,
    key: TerrainChunkKey,
) -> TerrainColliderPatch {
    let resolution =
        (resolution_for_lod(config, key.lod) / config.collider.resolution_divisor.max(1)).max(4);
    let sample_origin = chunk_origin_local(key.coord, config);
    let extent = chunk_extent_local(key.coord, config);
    let columns = resolution + 1;
    let rows = resolution + 1;
    let mut heights = Vec::with_capacity((columns * rows) as usize);

    for row in 0..rows {
        for column in 0..columns {
            let tx = column as f32 / resolution as f32;
            let tz = row as f32 / resolution as f32;
            let local = sample_origin + extent * Vec2::new(tx, tz);
            heights
                .push(sample_height_local(local, config, source).unwrap_or(config.height_offset));
        }
    }

    TerrainColliderPatch {
        origin: Vec2::ZERO,
        extent,
        dimensions: UVec2::new(columns, rows),
        heights: Arc::from(heights),
    }
}

#[cfg(test)]
#[path = "meshing_tests.rs"]
mod tests;
