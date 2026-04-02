use super::*;
use bevy::math::UVec2;

#[test]
fn datasets_reject_zero_dimensions() {
    assert!(TerrainDataset::from_heights(UVec2::ZERO, Vec::new()).is_err());
}

#[test]
fn weight_maps_reject_zero_dimensions() {
    assert!(TerrainWeightMap::from_rgba(UVec2::ZERO, Vec::new()).is_err());
}
