use super::*;
use crate::{
    TerrainBlendRange, TerrainConfig, TerrainDataset, TerrainHoleMask, TerrainLayer,
    TerrainMaterialProfile,
};
use bevy::color::Color;

fn flat_dataset() -> TerrainDataset {
    TerrainDataset::from_fn(UVec2::new(8, 8), |_coord, uv| uv.x * 0.5 + uv.y * 0.25).unwrap()
}

#[test]
fn sample_height_uses_config_scale_and_offset() {
    let source = flat_dataset();
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        height_scale: 20.0,
        height_offset: 3.0,
        ..default()
    };
    let transform = GlobalTransform::default();

    let height = sample_height(Vec3::new(4.0, 0.0, 4.0), &transform, &config, &source).unwrap();
    assert!(height > 8.0);
    assert!(height < 20.0);
}

#[test]
fn sample_normal_reports_non_flat_surface() {
    let source = flat_dataset();
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        ..default()
    };
    let transform = GlobalTransform::default();

    let normal = sample_normal(Vec3::new(4.0, 0.0, 4.0), &transform, &config, &source).unwrap();
    assert!(normal != Vec3::Y);
    assert!(normal.x.abs() > 0.01 || normal.z.abs() > 0.01);
}

#[test]
fn layer_weights_follow_height_rules() {
    let source = flat_dataset();
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("low", Color::srgb(0.2, 0.4, 0.2))
                    .with_height_range(TerrainBlendRange::new(0.0, 0.4))
                    .with_strength(0.2),
                TerrainLayer::tinted("high", Color::srgb(0.8, 0.8, 0.9))
                    .with_height_range(TerrainBlendRange::new(0.4, 1.0)),
            ],
            ..default()
        },
        ..default()
    };
    let transform = GlobalTransform::default();

    let low = sample_layer_weights(Vec3::new(0.2, 0.0, 0.2), &transform, &config, &source).unwrap();
    let high =
        sample_layer_weights(Vec3::new(7.8, 0.0, 7.8), &transform, &config, &source).unwrap();
    assert_eq!(low.weights.len(), 2);
    assert_eq!(high.weights.len(), 2);

    assert_eq!(low.dominant_layer, Some(0));
    assert_eq!(high.dominant_layer, Some(1));
    assert!(high.weights[1] > 0.6);
}

#[test]
fn sample_height_respects_root_translation() {
    let source = flat_dataset();
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        ..default()
    };
    let identity = GlobalTransform::default();
    let translated = GlobalTransform::from(Transform::from_xyz(0.0, 15.0, 0.0));

    let base = sample_height(Vec3::new(4.0, 0.0, 4.0), &identity, &config, &source).unwrap();
    let moved = sample_height(Vec3::new(4.0, 0.0, 4.0), &translated, &config, &source).unwrap();

    assert!((moved - (base + 15.0)).abs() < 0.0001);
}

#[test]
fn sampling_returns_none_inside_holes() {
    let source = TerrainDataset::from_heights(UVec2::new(2, 2), vec![0.5; 4])
        .unwrap()
        .with_hole_mask(TerrainHoleMask::from_values(UVec2::new(2, 2), vec![1.0; 4]).unwrap());
    let config = TerrainConfig {
        size: Vec2::new(8.0, 8.0),
        ..default()
    };
    let transform = GlobalTransform::default();

    assert!(sample_height(Vec3::new(4.0, 0.0, 4.0), &transform, &config, &source).is_none());
    assert!(sample_normal(Vec3::new(4.0, 0.0, 4.0), &transform, &config, &source).is_none());
    assert!(sample_terrain(Vec3::new(4.0, 0.0, 4.0), &transform, &config, &source).is_none());
}
