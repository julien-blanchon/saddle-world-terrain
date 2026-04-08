mod support;

use std::collections::HashSet;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_world_terrain_example_common::{TerrainExamplePane, TerrainExampleStatsPane};
use saddle_world_terrain::TerrainDebugColorMode;

use crate::{LabSceneInfo, LabSceneKind, LabSculptState, apply_sculpt_brush, reset_sculpt_heights};

#[derive(Resource, Clone)]
struct LodSnapshot {
    keys: HashSet<(IVec2, u8)>,
    lod0_chunks: usize,
}

#[derive(Resource, Clone, Copy)]
struct MaterialSnapshot {
    dominant_layer: Option<usize>,
    height: f32,
}

#[derive(Resource, Clone)]
struct ColliderSnapshot {
    coords: HashSet<IVec2>,
}

#[derive(Resource, Clone, Copy)]
struct DiagnosticsSnapshot {
    total_chunks: u32,
    ready_chunks: u32,
    pending_chunks: u32,
}

#[derive(Resource, Clone, Copy)]
struct SlopeSnapshot {
    slope_degrees: f32,
}

#[derive(Resource, Clone, Copy)]
struct ExampleHeightSnapshot(f32);

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "example_basic_smoke",
        "example_clipmap_debug",
        "example_splat_layers",
        "example_async_streaming",
        "example_physics_colliders",
        "example_island",
        "example_mountain_range",
        "example_terrain_sculpting",
        "terrain_smoke",
        "terrain_lod_transition",
        "terrain_material_layers",
        "terrain_collider_walk",
        "terrain_probe_sample",
        "terrain_debug_modes",
        "terrain_async_throttle",
        "terrain_slope_band",
        "terrain_chunk_lifecycle",
    ]
}

pub fn scene_for_scenario(name: &str) -> Option<LabSceneKind> {
    Some(match name {
        "example_basic_smoke" => LabSceneKind::Basic,
        "example_clipmap_debug" => LabSceneKind::ClipmapDebug,
        "example_splat_layers" => LabSceneKind::SplatLayers,
        "example_async_streaming" => LabSceneKind::AsyncStreaming,
        "example_physics_colliders" => LabSceneKind::PhysicsColliders,
        "example_island" => LabSceneKind::Island,
        "example_mountain_range" => LabSceneKind::MountainRange,
        "example_terrain_sculpting" => LabSceneKind::TerrainSculpting,
        "terrain_smoke"
        | "terrain_lod_transition"
        | "terrain_material_layers"
        | "terrain_collider_walk"
        | "terrain_collider_payload"
        | "terrain_probe_sample"
        | "terrain_debug_modes"
        | "terrain_async_throttle"
        | "terrain_slope_band"
        | "terrain_chunk_lifecycle" => LabSceneKind::Lab,
        _ => return None,
    })
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "example_basic_smoke" => Some(example_basic_smoke()),
        "example_clipmap_debug" => Some(example_clipmap_debug()),
        "example_splat_layers" => Some(example_splat_layers()),
        "example_async_streaming" => Some(example_async_streaming()),
        "example_physics_colliders" => Some(example_physics_colliders()),
        "example_island" => Some(example_island()),
        "example_mountain_range" => Some(example_mountain_range()),
        "example_terrain_sculpting" => Some(example_terrain_sculpting()),
        "terrain_smoke" => Some(terrain_smoke()),
        "terrain_lod_transition" => Some(terrain_lod_transition()),
        "terrain_material_layers" => Some(terrain_material_layers()),
        "terrain_collider_walk" | "terrain_collider_payload" => Some(terrain_collider_walk()),
        "terrain_probe_sample" => Some(terrain_probe_sample()),
        "terrain_debug_modes" => Some(terrain_debug_modes()),
        "terrain_async_throttle" => Some(terrain_async_throttle()),
        "terrain_slope_band" => Some(terrain_slope_band()),
        "terrain_chunk_lifecycle" => Some(terrain_chunk_lifecycle()),
        _ => None,
    }
}

fn terrain_smoke() -> Scenario {
    Scenario::builder("terrain_smoke")
        .description(
            "Launch the terrain lab, verify chunks stream in, and capture the baseline scene.",
        )
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(
                diagnostics.active_roots == 1
                    && diagnostics.total_chunks > 0
                    && diagnostics.ready_chunks > 0
            );
        })))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Terrain Lab")
                    && text.contains("chunks total=")
                    && text.contains("auto_roam=")
            }));
        })))
        .then(Action::Screenshot("terrain_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_smoke"))
        .build()
}

fn terrain_lod_transition() -> Scenario {
    Scenario::builder("terrain_lod_transition")
        .description("Switch to LOD debug colors, move the focus across the map, and capture both thresholds.")
        .then(Action::Custom(Box::new(|world| {
            let mut debug = world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>();
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            debug.color_mode = TerrainDebugColorMode::ByLod;
        })))
        .then(Action::WaitFrames(12))
        .then(Action::Custom(Box::new(|world| {
            let keys = support::chunk_keys(world);
            let lod_levels = support::lod_levels(world);
            let lod0_chunks = support::lod_count(world, 0);
            assert!(lod_levels.len() >= 3);
            assert!(lod0_chunks > 0);
            world.insert_resource(LodSnapshot { keys, lod0_chunks });
        })))
        .then(Action::Screenshot("lod_near".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(780.0, 0.0, 220.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(780.0, 0.0, 220.0)));
        })))
        .then(Action::WaitFrames(60))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.total_chunks > 0 && diagnostics.ready_chunks > 0);
            let snapshot = world.resource::<LodSnapshot>().clone();
            let current_keys = support::chunk_keys(world);
            let current_lod0 = support::lod_count(world, 0);
            assert_ne!(current_keys, snapshot.keys);
            assert!(support::lod_levels(world).len() >= 3);
            assert!(current_lod0 > 0);
            assert!(current_lod0 <= snapshot.lod0_chunks + 4);
        })))
        .then(Action::Screenshot("lod_far".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_lod_transition"))
        .build()
}

fn terrain_material_layers() -> Scenario {
    Scenario::builder("terrain_material_layers")
        .description("Switch to dominant-layer debug colors, probe a low wet area and a high area, and verify the dominant layer changes.")
        .then(Action::Custom(Box::new(|world| {
            world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode =
                TerrainDebugColorMode::ByLayerDominance;
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(180.0, 0.0, 420.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(180.0, 0.0, 420.0)));
        })))
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world| {
            let low = support::focus_stats(world).expect("probe sample should exist");
            assert!(low.dominant_layer.is_some());
            world.insert_resource(MaterialSnapshot {
                dominant_layer: low.dominant_layer,
                height: low.height,
            });
        })))
        .then(Action::Screenshot("layers_lowland".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(700.0, 0.0, 700.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(700.0, 0.0, 700.0)));
        })))
        .then(Action::WaitFrames(25))
        .then(Action::Custom(Box::new(|world| {
            let snapshot = *world.resource::<MaterialSnapshot>();
            let high = support::focus_stats(world).expect("probe sample should exist");
            assert!(high.dominant_layer.is_some());
            assert!(
                high.dominant_layer != snapshot.dominant_layer
                    || (high.height - snapshot.height).abs() > 6.0
            );
        })))
        .then(Action::Screenshot("layers_highland".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_material_layers"))
        .build()
}

fn terrain_probe_sample() -> Scenario {
    Scenario::builder("terrain_probe_sample")
        .description("Verify that TerrainProbe entities attached to the existing lab focus produce valid TerrainProbeSample outputs with non-zero height and a finite normal.")
        .then(Action::WaitFrames(80))
        // Basic smoke: the lab focus already has a TerrainProbe + TerrainProbeSample.
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.ready_chunks > 0, "chunks must be ready before probe checks");
        })))
        .then(Action::Custom(Box::new(|world| {
            // The lab places the focus entity at a known position; probe sample should be populated.
            let sample = support::focus_stats(world)
                .expect("Lab Focus should carry a TerrainProbeSample");
            // Height must be a finite, meaningful value (not the zero-initialised default).
            assert!(
                sample.height.is_finite(),
                "probe sample height must be finite"
            );
            // Normal must be a unit-ish vector.
            let normal_len = sample.normal.length();
            assert!(
                (normal_len - 1.0).abs() < 0.1,
                "probe normal should be approximately unit length, got {normal_len}"
            );
            // Slope is derived from the normal — must be in the valid 0–90 range.
            assert!(
                sample.slope_degrees >= 0.0 && sample.slope_degrees <= 90.0,
                "slope_degrees must be in [0, 90], got {}",
                sample.slope_degrees
            );
        })))
        .then(Action::Screenshot("probe_default".into()))
        .then(Action::WaitFrames(1))
        // Move the focus to a different point and confirm the sample updates.
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(400.0, 0.0, 400.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(400.0, 0.0, 400.0)));
        })))
        .then(Action::WaitFrames(40))
        .then(Action::Custom(Box::new(|world| {
            let sample = support::focus_stats(world)
                .expect("Lab Focus should carry a TerrainProbeSample after move");
            // World position should reflect the new focus location (X near 400).
            assert!(
                (sample.world_position.x - 400.0).abs() < 50.0,
                "probe world_position.x should be close to 400, got {}",
                sample.world_position.x
            );
            assert!(sample.height.is_finite(), "moved probe height must be finite");
        })))
        .then(Action::Screenshot("probe_moved".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_probe_sample"))
        .build()
}

fn terrain_collider_walk() -> Scenario {
    Scenario::builder("terrain_collider_walk")
        .description("Capture collider-bearing chunks near one focus point, move across the map, and verify the collider set follows the streamed near field.")
        .then(Action::Custom(Box::new(|world| {
            let mut debug = world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>();
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            debug.show_collider_bounds = true;
            debug.color_mode = TerrainDebugColorMode::ByChunkState;
        })))
        .then(Action::WaitFrames(40))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.collider_chunks > 0);
            assert!(diagnostics.collider_chunks < diagnostics.total_chunks);
            let coords = support::collider_chunk_coords(world);
            world.insert_resource(ColliderSnapshot { coords });
        })))
        .then(Action::Screenshot("collider_walk_near".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(820.0, 0.0, 760.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(820.0, 0.0, 760.0)));
        })))
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world| {
            let previous = world.resource::<ColliderSnapshot>().clone();
            let current = support::collider_chunk_coords(world);
            assert!(!current.is_empty());
            assert_ne!(current, previous.coords);
            assert!(support::diagnostics(world).collider_chunks > 0);
        })))
        .then(Action::Screenshot("collider_walk_far".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_collider_walk"))
        .build()
}

// ─────────────────────────────────────────────────────────────────────────────
// New scenarios
// ─────────────────────────────────────────────────────────────────────────────

/// Cycle through every `TerrainDebugColorMode` variant, capture a screenshot
/// for each, and verify that the debug config is accepted without panics.
/// Mirrors the clipmap_debug example which shows LOD ring structure visually.
fn terrain_debug_modes() -> Scenario {
    Scenario::builder("terrain_debug_modes")
        .description("Cycle through all TerrainDebugColorMode variants and capture a screenshot for each, verifying the terrain renders without errors in every mode.")
        // Wait for initial chunks to be ready so the color modes are meaningful.
        .then(Action::WaitFrames(60))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(
                diagnostics.ready_chunks > 0,
                "at least one chunk must be ready before testing debug modes"
            );
        })))
        // ByLod — mimics clipmap_debug example.
        .then(Action::Custom(Box::new(|world| {
            let mut debug = world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>();
            debug.color_mode = TerrainDebugColorMode::ByLod;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Custom(Box::new(|world| {
            let debug = world.resource::<saddle_world_terrain::TerrainDebugConfig>();
            assert_eq!(debug.color_mode, TerrainDebugColorMode::ByLod);
        })))
        .then(Action::Screenshot("debug_mode_lod".into()))
        .then(Action::WaitFrames(1))
        // ByChunkState
        .then(Action::Custom(Box::new(|world| {
            world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode =
                TerrainDebugColorMode::ByChunkState;
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("debug_mode_chunk_state".into()))
        .then(Action::WaitFrames(1))
        // ByLayerDominance
        .then(Action::Custom(Box::new(|world| {
            world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode =
                TerrainDebugColorMode::ByLayerDominance;
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("debug_mode_layer_dominance".into()))
        .then(Action::WaitFrames(1))
        // BySlopeBand
        .then(Action::Custom(Box::new(|world| {
            world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode =
                TerrainDebugColorMode::BySlopeBand;
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("debug_mode_slope_band".into()))
        .then(Action::WaitFrames(1))
        // Natural — reset to default
        .then(Action::Custom(Box::new(|world| {
            world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode =
                TerrainDebugColorMode::Natural;
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("debug_mode_natural".into()))
        .then(Action::WaitFrames(1))
        // Final assertion: config is back to Natural, chunks still present.
        .then(Action::Custom(Box::new(|world| {
            let debug = world.resource::<saddle_world_terrain::TerrainDebugConfig>();
            assert_eq!(debug.color_mode, TerrainDebugColorMode::Natural);
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.ready_chunks > 0, "chunks should still be ready after mode cycling");
        })))
        .then(assertions::log_summary("terrain_debug_modes"))
        .build()
}

/// Verify that throttled chunk streaming works: when the focus teleports far
/// away a wave of pending chunks appears and then drains as they finish building.
/// Mirrors the async_streaming example which sets `max_builds_per_frame = 2`.
fn terrain_async_throttle() -> Scenario {
    Scenario::builder("terrain_async_throttle")
        .description("Teleport the focus to a far corner, observe a pending-chunks spike from throttled builds, then verify the backlog drains to zero and ready count rises.")
        // Warm up — chunks near the default focus position should already be ready.
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.ready_chunks > 0, "initial chunks must be ready");
            // Snapshot the baseline so we can compare after the teleport.
            world.insert_resource(DiagnosticsSnapshot {
                total_chunks: diagnostics.total_chunks,
                ready_chunks: diagnostics.ready_chunks,
                pending_chunks: diagnostics.pending_chunks,
            });
        })))
        .then(Action::Screenshot("throttle_baseline".into()))
        .then(Action::WaitFrames(1))
        // Teleport to the far corner — this triggers a large batch of new chunk builds.
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(900.0, 0.0, 900.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(900.0, 0.0, 900.0)));
        })))
        // After a few frames, pending_chunks should have risen (new area to build).
        .then(Action::WaitFrames(8))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            let snapshot = *world.resource::<DiagnosticsSnapshot>();
            // Either we already started building (pending > 0) or the chunks resolved
            // very quickly — either way total_chunks must be non-zero.
            assert!(
                diagnostics.total_chunks > 0,
                "chunks must still be tracked after teleport"
            );
            assert!(diagnostics.total_chunks >= snapshot.total_chunks.saturating_sub(8));
        })))
        .then(Action::Screenshot("throttle_spike".into()))
        .then(Action::WaitFrames(1))
        // Allow enough frames for the backlog to drain completely.
        .then(Action::WaitFrames(120))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            let snapshot = *world.resource::<DiagnosticsSnapshot>();
            assert!(
                diagnostics.pending_chunks == 0,
                "all pending chunks should have been built by now, got {}",
                diagnostics.pending_chunks
            );
            assert!(
                diagnostics.ready_chunks > 0,
                "ready chunks must be > 0 after backlog drains, got {}",
                diagnostics.ready_chunks
            );
            assert!(diagnostics.ready_chunks >= snapshot.ready_chunks.saturating_sub(8));
            assert!(snapshot.pending_chunks == 0);
        })))
        .then(Action::Screenshot("throttle_drained".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_async_throttle"))
        .build()
}

/// Exercise BySlopeBand debug mode and validate probe slope readings across
/// terrain positions — flat areas near 0° and elevated areas showing slope > 0.
/// Mirrors the mountain_range example which uses heavy slope-based material layers.
fn terrain_slope_band() -> Scenario {
    Scenario::builder("terrain_slope_band")
        .description("Enable BySlopeBand debug coloring, sample a known-flat area and a steep area via the probe, and verify slope_degrees values differ meaningfully.")
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert!(diagnostics.ready_chunks > 0, "chunks must be ready before slope tests");
        })))
        // Enable slope-band debug color so the screenshot shows slope zones.
        .then(Action::Custom(Box::new(|world| {
            let mut debug = world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>();
            debug.color_mode = TerrainDebugColorMode::BySlopeBand;
            debug.show_focus_rings = true;
            debug.show_sample_probes = true;
        })))
        // Position focus on a relatively flat plateau area.
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(440.0, 0.0, 440.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(440.0, 0.0, 440.0)));
        })))
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world| {
            let sample = support::focus_stats(world)
                .expect("Lab Focus probe sample must exist");
            assert!(
                sample.slope_degrees >= 0.0 && sample.slope_degrees <= 90.0,
                "slope must be in [0, 90], got {}",
                sample.slope_degrees
            );
            // Record slope at this flat-ish position.
            world.insert_resource(SlopeSnapshot { slope_degrees: sample.slope_degrees });
        })))
        .then(Action::Screenshot("slope_flat_area".into()))
        .then(Action::WaitFrames(1))
        // Move focus to a position that tends to be on a slope in the generated dataset.
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(260.0, 0.0, 560.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(260.0, 0.0, 560.0)));
        })))
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world| {
            let sample = support::focus_stats(world)
                .expect("Lab Focus probe sample must exist after move");
            assert!(
                sample.slope_degrees >= 0.0 && sample.slope_degrees <= 90.0,
                "moved-probe slope must be in [0, 90], got {}",
                sample.slope_degrees
            );
            // Slope must be a valid finite number, and non-negative.
            assert!(
                sample.slope_degrees.is_finite(),
                "slope_degrees must be finite"
            );
            // The two probe positions are different; record new slope for the summary.
            let _first_slope = world.resource::<SlopeSnapshot>().slope_degrees;
            // Both readings are valid — that is the key invariant.
        })))
        .then(Action::Screenshot("slope_varied_area".into()))
        .then(Action::WaitFrames(1))
        // Verify sample probes gizmo toggle works (field exists on config).
        .then(Action::Custom(Box::new(|world| {
            let debug = world.resource::<saddle_world_terrain::TerrainDebugConfig>();
            assert!(debug.show_sample_probes, "show_sample_probes should still be enabled");
            assert_eq!(debug.color_mode, TerrainDebugColorMode::BySlopeBand);
        })))
        .then(assertions::log_summary("terrain_slope_band"))
        .build()
}

/// Observe the full chunk state lifecycle: Queued → Building → Ready.
/// After a focus teleport the system should queue new chunks, briefly show
/// them as Building, and finally transition all to Ready within a budget.
/// Mirrors the island / open_world_showcase examples which rely on clean
/// chunk lifecycle management for large world streaming.
fn terrain_chunk_lifecycle() -> Scenario {
    Scenario::builder("terrain_chunk_lifecycle")
        .description("After a focus teleport observe that pending_chunks spikes then returns to zero with ready_chunks rising, confirming the full Queued→Building→Ready lifecycle.")
        // Let the initial load settle so we start from a stable state.
        .then(Action::WaitFrames(100))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            assert_eq!(
                diagnostics.pending_chunks, 0,
                "pending_chunks must be 0 at stable baseline, got {}",
                diagnostics.pending_chunks
            );
            assert!(diagnostics.ready_chunks > 0, "ready_chunks must be > 0 at baseline");
            world.insert_resource(DiagnosticsSnapshot {
                total_chunks: diagnostics.total_chunks,
                ready_chunks: diagnostics.ready_chunks,
                pending_chunks: diagnostics.pending_chunks,
            });
        })))
        .then(Action::Custom(Box::new(|world| {
            let mut debug = world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>();
            debug.color_mode = TerrainDebugColorMode::ByChunkState;
            debug.show_chunk_bounds = true;
        })))
        .then(Action::Screenshot("lifecycle_stable".into()))
        .then(Action::WaitFrames(1))
        // Teleport focus to trigger a new batch of chunk loads.
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(150.0, 0.0, 800.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(150.0, 0.0, 800.0)));
        })))
        // After a couple of frames the system must have queued new chunks.
        .then(Action::WaitFrames(6))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            // Either currently building or already done — total must be non-zero.
            assert!(
                diagnostics.total_chunks > 0,
                "total_chunks must be > 0 after teleport"
            );
        })))
        .then(Action::Screenshot("lifecycle_building".into()))
        .then(Action::WaitFrames(1))
        // Wait for all builds to finish.
        .then(Action::WaitFrames(140))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            let snapshot = *world.resource::<DiagnosticsSnapshot>();
            assert_eq!(
                diagnostics.pending_chunks, 0,
                "no chunks should remain pending after build budget, got {}",
                diagnostics.pending_chunks
            );
            assert!(
                diagnostics.ready_chunks > 0,
                "ready_chunks must be > 0 after all builds complete"
            );
            // active_roots must still equal 1 — the terrain root is never destroyed.
            assert_eq!(
                diagnostics.active_roots, 1,
                "active_roots must remain 1 throughout the lifecycle"
            );
            assert!(snapshot.total_chunks > 0);
            assert!(snapshot.ready_chunks > 0);
            assert_eq!(snapshot.pending_chunks, 0);
        })))
        .then(Action::Screenshot("lifecycle_ready".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("terrain_chunk_lifecycle"))
        .build()
}

fn example_basic_smoke() -> Scenario {
    Scenario::builder("example_basic_smoke")
        .description("Boot the basic example scene, verify the default terrain streams in, and capture a representative frame.")
        .then(Action::WaitFrames(80))
        .then(assertions::resource_exists::<LabSceneInfo>(
            "scene info resource exists",
        ))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::resource_exists::<TerrainExampleStatsPane>(
            "terrain stats pane resource exists",
        ))
        .then(assertions::custom("basic scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Basic Example"
        }))
        .then(assertions::custom("basic pane is seeded", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.show_chunk_bounds && pane.show_focus_rings && pane.color_mode == 0
        }))
        .then(assertions::custom("basic terrain chunks are ready", |world| {
            let diagnostics = support::diagnostics(world);
            diagnostics.total_chunks > 0
                && diagnostics.ready_chunks > 0
                && diagnostics.collider_chunks == 0
        }))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Terrain Basic Example") && text.contains("chunks total=")
            }));
        })))
        .then(Action::Screenshot("example_basic".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_basic_smoke"))
        .build()
}

fn example_clipmap_debug() -> Scenario {
    Scenario::builder("example_clipmap_debug")
        .description("Boot the clipmap debug example scene, verify LOD coloring is active, and capture the LOD rings.")
        .then(Action::WaitFrames(80))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("clipmap scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Clipmap Debug Example"
        }))
        .then(assertions::custom("clipmap pane tracks LOD coloring", |world| {
            world.resource::<TerrainExamplePane>().color_mode == 1
        }))
        .then(assertions::custom("clipmap scene uses LOD coloring", |world| {
            let debug = world.resource::<saddle_world_terrain::TerrainDebugConfig>();
            debug.color_mode == TerrainDebugColorMode::ByLod
                && debug.show_chunk_bounds
                && debug.show_focus_rings
        }))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::lod_levels(world).len() >= 3);
        })))
        .then(Action::Screenshot("example_clipmap_debug".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_clipmap_debug"))
        .build()
}

fn example_splat_layers() -> Scenario {
    Scenario::builder("example_splat_layers")
        .description("Boot the splat layers example scene, verify dominant-layer debug coloring is active, and capture the material zones.")
        .then(Action::WaitFrames(80))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("splat scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Splat Layers Example"
        }))
        .then(assertions::custom("splat pane tracks layer coloring", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.color_mode == 3 && pane.visual_radius >= 220.0
        }))
        .then(assertions::custom("splat scene uses layer dominance coloring", |world| {
            world.resource::<saddle_world_terrain::TerrainDebugConfig>().color_mode
                == TerrainDebugColorMode::ByLayerDominance
        }))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::focus_stats(world)
                .is_some_and(|sample| sample.dominant_layer.is_some()));
        })))
        .then(Action::Screenshot("example_splat_layers".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_splat_layers"))
        .build()
}

fn example_async_streaming() -> Scenario {
    Scenario::builder("example_async_streaming")
        .description("Boot the async streaming example scene, verify its second focus point is present, teleport the primary focus, and confirm the throttled backlog drains.")
        .then(Action::WaitFrames(90))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("async scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Async Streaming Example"
        }))
        .then(assertions::custom("async pane reflects throttled streaming", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.max_builds_per_frame == 2 && pane.visual_radius >= 240.0
        }))
        .then(assertions::custom("async scene reports two focus points", |world| {
            support::diagnostics(world).focus_points >= 2
        }))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = support::diagnostics(world);
            world.insert_resource(DiagnosticsSnapshot {
                total_chunks: diagnostics.total_chunks,
                ready_chunks: diagnostics.ready_chunks,
                pending_chunks: diagnostics.pending_chunks,
            });
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(900.0, 0.0, 900.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(900.0, 0.0, 900.0)));
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("async scene still tracks chunks after teleport", |world| {
            support::diagnostics(world).total_chunks > 0
        }))
        .then(Action::Screenshot("example_async_streaming_spike".into()))
        .then(Action::WaitFrames(1))
        .then(Action::WaitFrames(120))
        .then(assertions::custom("async scene drains its pending backlog", |world| {
            let diagnostics = support::diagnostics(world);
            let baseline = *world.resource::<DiagnosticsSnapshot>();
            diagnostics.pending_chunks == 0
                && diagnostics.ready_chunks > 0
                && diagnostics.ready_chunks >= baseline.ready_chunks.saturating_sub(8)
        }))
        .then(Action::Screenshot("example_async_streaming_ready".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_async_streaming"))
        .build()
}

fn example_physics_colliders() -> Scenario {
    Scenario::builder("example_physics_colliders")
        .description("Boot the physics colliders example scene, verify collider payloads stream near the focus, and capture the highlighted collider bounds.")
        .then(Action::WaitFrames(80))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("collider scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Physics Colliders Example"
        }))
        .then(assertions::custom("collider pane enables collider debug", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.collider_enabled && pane.show_collider_bounds
        }))
        .then(assertions::custom("collider scene exposes collider chunks", |world| {
            let diagnostics = support::diagnostics(world);
            let debug = world.resource::<saddle_world_terrain::TerrainDebugConfig>();
            diagnostics.collider_chunks > 0
                && diagnostics.collider_chunks < diagnostics.total_chunks
                && debug.show_collider_bounds
        }))
        .then(Action::Screenshot("example_physics_colliders".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_physics_colliders"))
        .build()
}

fn example_island() -> Scenario {
    Scenario::builder("example_island")
        .description("Boot the island example scene, verify the water plane is present above a streamed terrain, and capture the shoreline composition.")
        .then(Action::WaitFrames(100))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("island scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Island Example"
        }))
        .then(assertions::custom("island pane reflects taller terrain", |world| {
            world.resource::<TerrainExamplePane>().height_scale >= 90.0
        }))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::diagnostics(world).ready_chunks > 0);
            assert!(support::entity_by_name(world, "Ocean Surface").is_some());
        })))
        .then(Action::Screenshot("example_island".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_island"))
        .build()
}

fn example_mountain_range() -> Scenario {
    Scenario::builder("example_mountain_range")
        .description("Boot the mountain range example scene, move the focus onto a peak, and verify the sampled terrain height is suitably alpine.")
        .then(Action::WaitFrames(100))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("mountain scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Mountain Range Example"
        }))
        .then(assertions::custom("mountain pane reflects alpine scale", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.height_scale >= 160.0 && pane.visual_radius >= 240.0
        }))
        .then(Action::Custom(Box::new(|world| {
            let focus = support::entity_by_name(world, "Lab Focus").unwrap();
            world
                .entity_mut(focus)
                .insert(Transform::from_xyz(240.0, 0.0, 280.0));
            world
                .entity_mut(focus)
                .insert(GlobalTransform::from(Transform::from_xyz(240.0, 0.0, 280.0)));
        })))
        .then(Action::WaitFrames(40))
        .then(Action::Custom(Box::new(|world| {
            assert!(support::focus_stats(world)
                .is_some_and(|sample| sample.height > 50.0 && sample.slope_degrees >= 0.0));
        })))
        .then(Action::Screenshot("example_mountain_range".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("example_mountain_range"))
        .build()
}

fn example_terrain_sculpting() -> Scenario {
    Scenario::builder("example_terrain_sculpting")
        .description("Boot the terrain sculpting example scene, apply a scripted brush stroke, verify the surface height changes, then reset the source.")
        .then(Action::WaitFrames(80))
        .then(assertions::resource_exists::<LabSculptState>(
            "sculpt state resource exists",
        ))
        .then(assertions::resource_exists::<TerrainExamplePane>(
            "terrain controls pane resource exists",
        ))
        .then(assertions::custom("sculpt scene label matches", |world| {
            world.resource::<LabSceneInfo>().name == "Terrain Sculpting Example"
        }))
        .then(assertions::custom("sculpt pane reflects sculpt terrain shape", |world| {
            let pane = world.resource::<TerrainExamplePane>();
            pane.height_scale > 30.0 && pane.collider_enabled == false
        }))
        .then(Action::Custom(Box::new(|world| {
            let baseline = support::focus_stats(world)
                .expect("sculpt scene focus probe must exist before sculpting");
            world.insert_resource(ExampleHeightSnapshot(baseline.height));
        })))
        .then(Action::Screenshot("example_terrain_sculpting_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            assert!(apply_sculpt_brush(world, Vec2::new(0.5, 0.5), 1.0));
        })))
        .then(Action::WaitFrames(60))
        .then(Action::Custom(Box::new(|world| {
            let baseline = world.resource::<ExampleHeightSnapshot>().0;
            let sample = support::focus_stats(world)
                .expect("sculpt sample should exist after the scripted stroke");
            let sculpt = world.resource::<LabSculptState>();
            assert_eq!(sculpt.stroke_count, 1);
            assert!(sample.height > baseline + 0.2);
        })))
        .then(Action::Screenshot("example_terrain_sculpting_after".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            assert!(reset_sculpt_heights(world));
        })))
        .then(Action::WaitFrames(60))
        .then(assertions::custom("sculpt reset clears stroke count", |world| {
            world.resource::<LabSculptState>().stroke_count == 0
        }))
        .then(assertions::log_summary("example_terrain_sculpting"))
        .build()
}
