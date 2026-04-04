use bevy::prelude::*;
use grass::{GrassConfig, GrassPatch, GrassWindBridge};
use saddle_bevy_e2e::{
    E2EPlugin, E2ESet, action::Action, actions::assertions, init_scenario, scenario::Scenario,
};
use saddle_world_foliage::FoliageLayer;
use saddle_world_sky::SkyConfig;
use saddle_world_terrain::{TerrainConfig, TerrainSystems};
use saddle_world_wind::WindConfig;

use crate::{OpenWorldPane, OpenWorldScene, ShowcaseOverlay};

pub struct OpenWorldShowcaseE2EPlugin;

impl Plugin for OpenWorldShowcaseE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(TerrainSystems::MaintainFocus));

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                init_scenario(app, scenario);
            } else {
                error!(
                    "[open_world_showcase:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["open_world_showcase_smoke", "open_world_showcase_tune"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "open_world_showcase_smoke" => Some(build_smoke()),
        "open_world_showcase_tune" => Some(build_tune()),
        _ => None,
    }
}

fn wait_until(
    label: impl Into<String>,
    condition: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> Action {
    Action::WaitUntil {
        label: label.into(),
        condition: Box::new(condition),
        max_frames: 240,
    }
}

fn approx_eq(left: f32, right: f32, epsilon: f32) -> bool {
    (left - right).abs() <= epsilon
}

fn build_smoke() -> Scenario {
    Scenario::builder("open_world_showcase_smoke")
        .description(
            "Launch the open-world showcase, wait for terrain, foliage, and grass diagnostics to settle, then capture the baseline valley.",
        )
        .then(wait_until("open world bootstrapped", |world| {
            world.get_resource::<OpenWorldScene>().is_some()
        }))
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (terrain_entity, grass_patch_entity, foliage_layer_entity) = {
                let scene = world.resource::<OpenWorldScene>();
                (
                    scene.terrain_entity,
                    scene.grass_patch_entity,
                    scene.foliage_layer_entity,
                )
            };
            let mut overlays = world.query_filtered::<&Text, With<ShowcaseOverlay>>();
            let overlay = overlays
                .single(world)
                .expect("open world overlay should exist");
            assert!(overlay.0.contains("Open-world integration showcase"));
            assert!(overlay.0.contains("terrain + procgen noise + sky + wind + foliage + grass"));
            assert!(world.get_entity(terrain_entity).is_ok());
            assert!(world.get::<GrassPatch>(grass_patch_entity).is_some());
            assert!(world.get::<FoliageLayer>(foliage_layer_entity).is_some());
            assert!(world.resource::<GrassWindBridge>().enabled);
        })))
        .then(Action::Screenshot("open_world_showcase_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("open_world_showcase_smoke summary"))
        .build()
}

fn build_tune() -> Scenario {
    Scenario::builder("open_world_showcase_tune")
        .description(
            "Retune terrain, sky, wind, and vegetation through the pane resource and verify the synced runtime settings update before capture.",
        )
        .then(wait_until("open world bootstrapped", |world| {
            world.get_resource::<OpenWorldScene>().is_some()
        }))
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut pane = world.resource_mut::<OpenWorldPane>();
            pane.terrain_height_scale = 108.0;
            pane.terrain_height_offset = -6.0;
            pane.time_of_day_hours = 17.2;
            pane.cloud_coverage = 0.62;
            pane.wind_speed = 10.8;
            pane.wind_intensity = 1.28;
            pane.grass_density = 32.0;
            pane.canopy_density = 0.060;
            pane.grass_sway_scale = 1.90;
        })))
        .then(Action::WaitFrames(5))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let pane = world.resource::<OpenWorldPane>();
            let scene = world.resource::<OpenWorldScene>();
            let terrain = world
                .get::<TerrainConfig>(scene.terrain_entity)
                .expect("terrain config should exist");
            let grass = world
                .get::<GrassConfig>(scene.grass_patch_entity)
                .expect("grass config should exist");
            let layer = world
                .get::<FoliageLayer>(scene.foliage_layer_entity)
                .expect("foliage layer should exist");
            let sky = world.resource::<SkyConfig>();
            let wind = world.resource::<WindConfig>();
            let bridge = world.resource::<GrassWindBridge>();

            assert!(
                approx_eq(pane.terrain_height_scale, 108.0, 0.01),
                "pane height_scale was {}",
                pane.terrain_height_scale
            );
            assert!(
                approx_eq(pane.terrain_height_offset, -6.0, 0.01),
                "pane height_offset was {}",
                pane.terrain_height_offset
            );
            assert!(
                approx_eq(terrain.height_scale, 108.0, 0.01),
                "terrain height_scale was {}",
                terrain.height_scale
            );
            assert!(
                approx_eq(terrain.height_offset, -6.0, 0.01),
                "terrain height_offset was {}",
                terrain.height_offset
            );
            assert!(
                approx_eq(grass.density_per_square_unit, 32.0, 0.01),
                "grass density was {}",
                grass.density_per_square_unit
            );
            assert!(
                approx_eq(layer.density_per_square_unit, 0.060, 0.001),
                "foliage density was {}",
                layer.density_per_square_unit
            );
            assert!(
                approx_eq(sky.time_of_day.hours, 17.2, 0.01),
                "sky hours were {}",
                sky.time_of_day.hours
            );
            assert!(
                approx_eq(wind.speed, 10.8, 0.01),
                "wind speed was {}",
                wind.speed
            );
            assert!(
                approx_eq(wind.intensity, 1.28, 0.01),
                "wind intensity was {}",
                wind.intensity
            );
            assert!(
                approx_eq(bridge.sway_strength_scale, 1.90, 0.01),
                "grass sway scale was {}",
                bridge.sway_strength_scale
            );
        })))
        .then(Action::Screenshot("open_world_showcase_tuned".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("open_world_showcase_tune summary"))
        .build()
}
