use crate::config::TerrainLodConfig;

pub fn select_lod(distance: f32, lod: &TerrainLodConfig) -> u8 {
    let mut threshold = lod.near_distance.max(0.001);
    for level in 0..lod.lod_count {
        if distance <= threshold {
            return level;
        }
        threshold *= lod.distance_multiplier.max(1.01);
    }
    lod.lod_count.saturating_sub(1)
}

pub fn select_lod_with_hysteresis(
    distance: f32,
    previous: Option<u8>,
    lod: &TerrainLodConfig,
) -> u8 {
    let candidate = select_lod(distance, lod);
    let Some(previous) = previous else {
        return candidate;
    };
    if previous == candidate {
        return previous;
    }

    let previous_distance = lod_distance_for_level(previous, lod);
    let hysteresis = lod.hysteresis.max(0.0);
    if (candidate > previous && distance < previous_distance + hysteresis)
        || (candidate < previous && distance > previous_distance - hysteresis)
    {
        previous
    } else {
        candidate
    }
}

pub fn lod_distance_for_level(level: u8, lod: &TerrainLodConfig) -> f32 {
    lod.near_distance * lod.distance_multiplier.powi(level as i32)
}

#[cfg(test)]
#[path = "lod_tests.rs"]
mod tests;
