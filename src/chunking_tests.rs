use super::*;
use crate::TerrainConfig;
use bevy::math::{IVec2, Vec2};

#[test]
fn terrain_chunk_for_local_clamps_exact_terrain_edge_to_last_chunk() {
    let config = TerrainConfig {
        size: Vec2::new(64.0, 64.0),
        chunk_size: Vec2::new(32.0, 32.0),
        ..default()
    };

    assert_eq!(
        terrain_chunk_for_local(Vec2::new(64.0, 64.0), &config),
        Some(IVec2::new(1, 1))
    );
}

#[test]
fn terrain_chunk_for_local_rejects_positions_outside_extent() {
    let config = TerrainConfig {
        size: Vec2::new(64.0, 64.0),
        chunk_size: Vec2::new(32.0, 32.0),
        ..default()
    };

    assert_eq!(terrain_chunk_for_local(Vec2::new(64.1, 16.0), &config), None);
    assert_eq!(terrain_chunk_for_local(Vec2::new(-0.1, 16.0), &config), None);
}
