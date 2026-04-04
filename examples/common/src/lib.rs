use bevy::prelude::*;
use saddle_world_terrain::{
    TerrainBlendRange, TerrainConfig, TerrainDataset, TerrainLayer, TerrainMaterialProfile,
};

/// Generate a procedural heightmap dataset with weight-painted regions.
pub fn generated_dataset(dimensions: UVec2) -> TerrainDataset {
    let base = TerrainDataset::from_fn(dimensions, |_coord, uv| {
        let ridge = ((uv.x * std::f32::consts::TAU * 1.7).sin() * 0.35
            + (uv.y * std::f32::consts::TAU * 2.3).cos() * 0.25)
            * 0.5
            + 0.5;
        let crater = 1.0 - ((uv - Vec2::new(0.72, 0.28)).length() * 1.8).clamp(0.0, 1.0);
        let shelf = ((uv.x * 3.5).fract() * 0.6 + (uv.y * 2.0).fract() * 0.4).sin() * 0.1 + 0.5;
        (ridge * 0.55 + crater * 0.25 + shelf * 0.20).clamp(0.0, 1.0)
    })
    .unwrap();

    let weights = (0..dimensions.y)
        .flat_map(|y| {
            (0..dimensions.x).map(move |x| {
                let uv = Vec2::new(
                    x as f32 / (dimensions.x - 1).max(1) as f32,
                    y as f32 / (dimensions.y - 1).max(1) as f32,
                );
                let river = (1.0 - ((uv.x - 0.18).abs() * 8.0)).clamp(0.0, 1.0);
                let meadow = (1.0 - ((uv - Vec2::new(0.55, 0.58)).length() * 2.4)).clamp(0.0, 1.0);
                let road = (1.0 - ((uv.y - 0.44).abs() * 10.0)).clamp(0.0, 1.0);
                [river, meadow, road, 0.0]
            })
        })
        .collect();

    base.with_weight_map(
        saddle_world_terrain::TerrainWeightMap::from_rgba(dimensions, weights).unwrap(),
    )
}

/// A richly configured terrain for examples: 640x640 world units, 5 material
/// layers (water, meadow, dirt, rock, snow) with height and slope blending.
pub fn default_config() -> TerrainConfig {
    TerrainConfig {
        size: Vec2::new(640.0, 640.0),
        chunk_size: Vec2::new(40.0, 40.0),
        vertex_resolution: 64,
        height_scale: 82.0,
        height_offset: -6.0,
        skirt_depth: 8.0,
        streaming: saddle_world_terrain::TerrainStreamingConfig {
            visual_radius: 180.0,
            collider_radius: 90.0,
            max_builds_per_frame: 8,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("water", Color::srgb(0.16, 0.34, 0.62)).with_weight_channel(0),
                TerrainLayer::tinted("meadow", Color::srgb(0.28, 0.56, 0.26))
                    .with_weight_channel(1)
                    .with_height_range(TerrainBlendRange::new(0.05, 0.65)),
                TerrainLayer::tinted("dirt", Color::srgb(0.47, 0.34, 0.24))
                    .with_weight_channel(2)
                    .with_height_range(TerrainBlendRange::new(0.10, 0.85)),
                TerrainLayer::tinted("rock", Color::srgb(0.56, 0.56, 0.58)).with_slope_range(
                    TerrainBlendRange {
                        start: 24.0,
                        end: 70.0,
                        falloff: 0.2,
                    },
                ),
                TerrainLayer::tinted("snow", Color::srgb(0.93, 0.95, 0.98)).with_height_range(
                    TerrainBlendRange {
                        start: 0.72,
                        end: 1.0,
                        falloff: 0.1,
                    },
                ),
            ],
            ..default()
        },
        ..default()
    }
}
