use super::*;
use crate::TerrainLodConfig;
use bevy::prelude::default;

#[test]
fn hysteresis_keeps_previous_lod_inside_transition_band() {
    let lod = TerrainLodConfig {
        near_distance: 32.0,
        distance_multiplier: 2.0,
        hysteresis: 6.0,
        ..default()
    };

    let previous = 0;
    let threshold = lod_distance_for_level(previous, &lod);

    assert_eq!(
        select_lod_with_hysteresis(threshold + 2.0, Some(previous), &lod),
        previous
    );
    assert_eq!(
        select_lod_with_hysteresis(threshold + 8.0, Some(previous), &lod),
        1
    );
}
