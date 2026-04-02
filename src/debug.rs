use bevy::{color::palettes::css, prelude::*, reflect::Reflect};

use crate::{
    TerrainColliderData, TerrainChunkState, TerrainFocus, TerrainFocusPoints, TerrainProbe,
    TerrainProbeSample, TerrainRoot, TerrainRootStats, components::TerrainChunkBounds,
    config::TerrainConfig,
};

#[derive(Default, Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainDebugColorMode {
    #[default]
    Natural,
    ByLod,
    ByChunkState,
    ByLayerDominance,
    BySlopeBand,
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource, Clone, Debug)]
pub struct TerrainDebugConfig {
    pub show_chunk_bounds: bool,
    pub show_focus_rings: bool,
    pub show_collider_bounds: bool,
    pub show_sample_probes: bool,
    pub color_mode: TerrainDebugColorMode,
}

impl Default for TerrainDebugConfig {
    fn default() -> Self {
        Self {
            show_chunk_bounds: false,
            show_focus_rings: false,
            show_collider_bounds: false,
            show_sample_probes: false,
            color_mode: TerrainDebugColorMode::Natural,
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug, Default)]
#[reflect(Resource, Clone, Debug)]
pub struct TerrainDiagnostics {
    pub active_roots: u32,
    pub total_chunks: u32,
    pub pending_chunks: u32,
    pub ready_chunks: u32,
    pub collider_chunks: u32,
    pub cache_entries: u32,
    pub focus_points: u32,
}

pub(crate) fn draw_debug_gizmos(
    debug: Res<TerrainDebugConfig>,
    focus_points: Res<TerrainFocusPoints>,
    roots: Query<(Entity, &TerrainConfig, &GlobalTransform, &TerrainRootStats), With<TerrainRoot>>,
    chunks: Query<
        (
            &crate::TerrainChunk,
            &TerrainChunkBounds,
            &GlobalTransform,
            &TerrainChunkState,
            Option<&TerrainColliderData>,
        ),
        With<crate::TerrainChunk>,
    >,
    focuses: Query<(&GlobalTransform, &TerrainFocus)>,
    probes: Query<(&GlobalTransform, &TerrainProbe, &TerrainProbeSample)>,
    mut gizmos: Option<Gizmos>,
) {
    let Some(gizmos) = gizmos.as_mut() else {
        return;
    };
    if debug.show_chunk_bounds || debug.show_collider_bounds {
        for (chunk, bounds, chunk_transform, state, collider) in &chunks {
            let center = (bounds.min + bounds.max) * 0.5;
            let size = bounds.max - bounds.min;

            if debug.show_chunk_bounds {
                let color = match debug.color_mode {
                    TerrainDebugColorMode::ByChunkState => chunk_state_color(*state),
                    _ => lod_color(chunk.key.lod),
                };
                gizmos.cube(oriented_bounds(chunk_transform, center, size), color);
            }

            if debug.show_collider_bounds && collider.is_some() {
                gizmos.cube(
                    oriented_bounds(chunk_transform, center + Vec3::Y * 0.35, size + Vec3::splat(0.2)),
                    Color::from(css::SPRING_GREEN),
                );
            }
        }
    }

    if debug.show_focus_rings {
        for (root_entity, config, root_transform, stats) in &roots {
            let ring_center = root_transform.affine().transform_point3(Vec3::new(
                config.size.x * 0.5,
                config.height_offset,
                config.size.y * 0.5,
            ));
            if stats.focus_count == 0 {
                gizmos.circle(
                    Isometry3d::new(
                        ring_center,
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ),
                    config.streaming.visual_radius,
                    Color::from(css::DARK_GRAY),
                );
            }

            for point in focus_points
                .0
                .iter()
                .filter(|point| point.terrain.is_none() || point.terrain == Some(root_entity))
            {
                draw_focus_gizmos(
                    gizmos,
                    point.position,
                    config.streaming.visual_radius + point.visual_radius_bias.max(0.0),
                    config.streaming.collider_radius + point.collider_radius_bias.max(0.0),
                );
            }

            for (focus_transform, focus) in &focuses {
                if focus.terrain.is_some() && focus.terrain != Some(root_entity) {
                    continue;
                }
                draw_focus_gizmos(
                    gizmos,
                    focus_transform.translation(),
                    config.streaming.visual_radius + focus.visual_radius_bias.max(0.0),
                    config.streaming.collider_radius + focus.collider_radius_bias.max(0.0),
                );
            }
        }
    }

    if debug.show_sample_probes {
        for (transform, _probe, sample) in &probes {
            let probe_position = transform.translation();
            let hit = sample.world_position;
            gizmos.line(probe_position, hit, Color::from(css::YELLOW));
            gizmos.arrow(hit, hit + sample.normal * 6.0, Color::from(css::ORANGE_RED));
            gizmos.sphere(hit, 0.35, Color::from(css::WHITE));
        }
    }
}

fn oriented_bounds(
    chunk_transform: &GlobalTransform,
    local_center: Vec3,
    local_size: Vec3,
) -> Transform {
    let mut transform = chunk_transform.compute_transform();
    transform.translation = chunk_transform.transform_point(local_center);
    transform.scale *= local_size;
    transform
}

fn lod_color(lod: u8) -> Color {
    match lod {
        0 => Color::from(css::LIMEGREEN),
        1 => Color::from(css::SKY_BLUE),
        2 => Color::from(css::GOLD),
        3 => Color::from(css::ORANGE_RED),
        _ => Color::from(css::VIOLET),
    }
}

fn chunk_state_color(state: TerrainChunkState) -> Color {
    match state {
        TerrainChunkState::Queued => Color::from(css::YELLOW),
        TerrainChunkState::Building => Color::from(css::ORANGE),
        TerrainChunkState::Ready => Color::from(css::LIMEGREEN),
        TerrainChunkState::Failed => Color::from(css::CRIMSON),
    }
}

fn draw_focus_gizmos(gizmos: &mut Gizmos, position: Vec3, visual_radius: f32, collider_radius: f32) {
    gizmos.circle(
        Isometry3d::new(position, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        visual_radius,
        Color::from(css::AQUA),
    );
    gizmos.circle(
        Isometry3d::new(position, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        collider_radius,
        Color::from(css::SPRING_GREEN),
    );
    gizmos.sphere(position + Vec3::Y * 0.2, 0.35, Color::from(css::WHITE));
}
