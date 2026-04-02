use super::*;
use crate::{
    TerrainBundle, TerrainConfig, TerrainDataset, TerrainFocus, TerrainPlugin, TerrainProbe,
    TerrainProbeSample,
};
use bevy::gizmos::GizmoPlugin;
use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
use std::{thread, time::Duration};

fn dataset() -> TerrainDataset {
    TerrainDataset::from_fn(UVec2::new(32, 32), |_coord, uv| (uv.x * uv.y).sqrt()).unwrap()
}

fn test_app() -> App {
    AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::default().build());

    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(GizmoPlugin);
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.add_plugins(TerrainPlugin::default());
    app
}

#[test]
fn chunk_spawns_and_despawns_with_focus_motion() {
    let mut app = test_app();
    let config = TerrainConfig {
        size: Vec2::new(128.0, 128.0),
        chunk_size: Vec2::new(32.0, 32.0),
        streaming: crate::TerrainStreamingConfig {
            visual_radius: 40.0,
            ..default()
        },
        ..default()
    };
    let terrain = app
        .world_mut()
        .spawn(TerrainBundle::new(dataset(), config))
        .id();
    let focus = app
        .world_mut()
        .spawn((
            TerrainFocus {
                terrain: Some(terrain),
                ..default()
            },
            Transform::from_xyz(16.0, 0.0, 16.0),
            GlobalTransform::default(),
        ))
        .id();

    app.update();
    let initial = {
        let world = app.world_mut();
        let mut query = world.query::<&TerrainChunk>();
        query.iter(world).count()
    };
    assert!(initial > 0);

    app.world_mut()
        .entity_mut(focus)
        .insert(Transform::from_xyz(112.0, 0.0, 112.0));
    app.world_mut()
        .entity_mut(focus)
        .insert(GlobalTransform::from(Transform::from_xyz(
            112.0, 0.0, 112.0,
        )));
    app.update();

    let moved = {
        let world = app.world_mut();
        let mut query = world.query::<&TerrainChunk>();
        query.iter(world).count()
    };
    assert!(moved > 0);
}

#[test]
fn stale_async_results_are_rejected_and_requeued() {
    let mut app = test_app();
    let config = TerrainConfig {
        size: Vec2::new(64.0, 64.0),
        chunk_size: Vec2::new(32.0, 32.0),
        streaming: crate::TerrainStreamingConfig {
            max_builds_per_frame: 1,
            ..default()
        },
        ..default()
    };
    let terrain = app
        .world_mut()
        .spawn(TerrainBundle::new(dataset(), config))
        .id();
    app.world_mut().spawn((
        TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
        Transform::from_xyz(16.0, 0.0, 16.0),
        GlobalTransform::default(),
    ));

    app.update();
    let chunk_entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &TerrainChunkRuntime, &TerrainBuildTask)>();
        query
            .iter(world)
            .next()
            .map(|(entity, _, _)| entity)
            .expect("expected a queued build task")
    };

    {
        let mut entity = app.world_mut().entity_mut(chunk_entity);
        let mut runtime = entity.get_mut::<TerrainChunkRuntime>().unwrap();
        runtime.build_generation = runtime.build_generation.wrapping_add(1);
    }

    for _ in 0..8 {
        thread::sleep(Duration::from_millis(2));
        app.update();
    }

    let state = app
        .world_mut()
        .get::<TerrainChunkState>(chunk_entity)
        .copied()
        .unwrap();
    assert!(matches!(
        state,
        TerrainChunkState::Queued | TerrainChunkState::Building | TerrainChunkState::Ready
    ));
}

#[test]
fn probe_sample_is_removed_when_probe_leaves_the_terrain() {
    let mut app = test_app();
    let config = TerrainConfig {
        size: Vec2::new(64.0, 64.0),
        ..default()
    };
    let terrain = app
        .world_mut()
        .spawn(TerrainBundle::new(dataset(), config))
        .id();
    let probe = app
        .world_mut()
        .spawn((
            TerrainProbe {
                terrain: Some(terrain),
                ..default()
            },
            Transform::from_xyz(16.0, 0.0, 16.0),
            GlobalTransform::default(),
        ))
        .id();

    app.update();
    assert!(app.world().get::<TerrainProbeSample>(probe).is_some());

    app.world_mut()
        .entity_mut(probe)
        .insert(Transform::from_xyz(200.0, 0.0, 200.0));
    app.world_mut()
        .entity_mut(probe)
        .insert(GlobalTransform::from(Transform::from_xyz(
            200.0, 0.0, 200.0,
        )));

    app.update();
    assert!(app.world().get::<TerrainProbeSample>(probe).is_none());
}
