use bevy::mesh::{Indices, VertexAttributeValues};

use super::*;

#[test]
fn terrain_config_uses_requested_scalars() {
    let config = terrain_config(84.0, -12.5);

    assert_eq!(config.height_scale, 84.0);
    assert_eq!(config.height_offset, -12.5);
    assert_eq!(config.size, TERRAIN_SIZE);
    assert_eq!(config.material.layers.len(), 5);
}

#[test]
fn sample_world_height_clamps_out_of_bounds_queries() {
    let dataset = build_dataset();
    let config = terrain_config(96.0, -10.0);

    let clamped_min = sample_world_height(&dataset, &config, Vec2::new(-80.0, -40.0));
    let origin = sample_world_height(&dataset, &config, Vec2::ZERO);
    assert!((clamped_min - origin).abs() < 0.001);

    let clamped_max = sample_world_height(&dataset, &config, TERRAIN_SIZE + Vec2::splat(80.0));
    let max = sample_world_height(&dataset, &config, TERRAIN_SIZE);
    assert!((clamped_max - max).abs() < 0.001);
}

#[test]
fn build_cover_mesh_matches_patch_resolution() {
    let dataset = build_dataset();
    let config = terrain_config(96.0, -10.0);
    let mesh = build_cover_mesh(&dataset, &config);

    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("positions should exist");
    let VertexAttributeValues::Float32x3(positions) = positions else {
        panic!("positions should be Float32x3");
    };
    assert_eq!(
        positions.len(),
        (PATCH_RESOLUTION.x * PATCH_RESOLUTION.y) as usize
    );

    let Some(Indices::U32(indices)) = mesh.indices() else {
        panic!("mesh should use u32 indices");
    };
    assert_eq!(
        indices.len(),
        ((PATCH_RESOLUTION.x - 1) * (PATCH_RESOLUTION.y - 1) * 6) as usize
    );
}
