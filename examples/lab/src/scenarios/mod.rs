mod support;

use std::collections::HashSet;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_world_terrain::TerrainDebugColorMode;

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

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "terrain_smoke",
        "terrain_lod_transition",
        "terrain_material_layers",
        "terrain_collider_walk",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "terrain_smoke" => Some(terrain_smoke()),
        "terrain_lod_transition" => Some(terrain_lod_transition()),
        "terrain_material_layers" => Some(terrain_material_layers()),
        "terrain_collider_walk" | "terrain_collider_payload" => Some(terrain_collider_walk()),
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
