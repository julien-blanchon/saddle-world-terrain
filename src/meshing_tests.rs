use super::*;
use crate::{TerrainConfig, TerrainDataset, TerrainDebugColorMode, TerrainHoleMask};
use bevy::math::Vec2;

fn dataset() -> TerrainDataset {
    TerrainDataset::from_fn(UVec2::new(16, 16), |_coord, uv| (uv.x + uv.y) * 0.5).unwrap()
}

#[test]
fn adjacent_chunks_share_world_border_positions() {
    let source = dataset();
    let config = TerrainConfig {
        size: Vec2::new(64.0, 32.0),
        chunk_size: Vec2::new(32.0, 32.0),
        vertex_resolution: 8,
        skirt_depth: 0.0,
        ..default()
    };

    let left = build_chunk_artifact(
        &source,
        &config,
        TerrainChunkKey {
            coord: IVec2::new(0, 0),
            lod: 0,
        },
        TerrainDebugColorMode::Natural,
    )
    .unwrap();
    let right = build_chunk_artifact(
        &source,
        &config,
        TerrainChunkKey {
            coord: IVec2::new(1, 0),
            lod: 0,
        },
        TerrainDebugColorMode::Natural,
    )
    .unwrap();

    let left_positions = left
        .mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let right_positions = right
        .mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let resolution = resolution_for_lod(&config, 0) as usize;
    let right_origin = chunk_origin_local(IVec2::new(1, 0), &config);

    for row in 0..=resolution {
        let left_vertex = left_positions[row * (resolution + 1) + resolution];
        let right_vertex = right_positions[row * (resolution + 1)];
        let left_world = Vec3::new(left_vertex[0], left_vertex[1], left_vertex[2]);
        let right_world = Vec3::new(
            right_vertex[0] + right_origin.x,
            right_vertex[1],
            right_vertex[2] + right_origin.y,
        );

        assert!((left_world.x - right_world.x).abs() < 0.0001);
        assert!((left_world.y - right_world.y).abs() < 0.0001);
        assert!((left_world.z - right_world.z).abs() < 0.0001);
    }
}

#[test]
fn skirts_add_extra_vertices() {
    let source = dataset();
    let config = TerrainConfig {
        size: Vec2::new(32.0, 32.0),
        chunk_size: Vec2::new(32.0, 32.0),
        vertex_resolution: 8,
        skirt_depth: 3.0,
        ..default()
    };

    let artifact = build_chunk_artifact(
        &source,
        &config,
        TerrainChunkKey {
            coord: IVec2::ZERO,
            lod: 0,
        },
        TerrainDebugColorMode::Natural,
    )
    .unwrap();

    let resolution = resolution_for_lod(&config, 0) as usize;
    let base_vertices = (resolution + 1) * (resolution + 1);
    let positions = artifact
        .mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    assert!(positions.len() > base_vertices);
}

#[test]
fn collider_patch_matches_requested_resolution() {
    let source = dataset();
    let config = TerrainConfig {
        vertex_resolution: 32,
        collider: crate::TerrainColliderConfig {
            enabled: true,
            resolution_divisor: 4,
        },
        ..default()
    };

    let artifact = build_chunk_artifact(
        &source,
        &config,
        TerrainChunkKey {
            coord: IVec2::ZERO,
            lod: 0,
        },
        TerrainDebugColorMode::Natural,
    )
    .unwrap();

    let collider = artifact.collider_patch.unwrap();
    assert_eq!(collider.dimensions, UVec2::new(9, 9));
    assert_eq!(collider.origin, Vec2::ZERO);
}

#[test]
fn hole_mask_removes_surface_triangles_and_marks_collider_samples() {
    let source = TerrainDataset::from_heights(UVec2::new(2, 2), vec![0.5; 4])
        .unwrap()
        .with_hole_mask(TerrainHoleMask::from_values(UVec2::new(2, 2), vec![1.0; 4]).unwrap());
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        chunk_size: Vec2::new(8.0, 8.0),
        vertex_resolution: 2,
        skirt_depth: 0.0,
        collider: crate::TerrainColliderConfig {
            enabled: true,
            ..default()
        },
        ..default()
    };

    let artifact = build_chunk_artifact(
        &source,
        &config,
        TerrainChunkKey {
            coord: IVec2::ZERO,
            lod: 0,
        },
        TerrainDebugColorMode::Natural,
    )
    .unwrap();

    let index_count = artifact
        .mesh
        .indices()
        .map(|indices| match indices {
            Indices::U16(values) => values.len(),
            Indices::U32(values) => values.len(),
        })
        .unwrap_or_default();
    let collider = artifact.collider_patch.unwrap();

    assert_eq!(index_count, 0);
    assert!(collider.holes.iter().all(|value| *value == 1));
}
