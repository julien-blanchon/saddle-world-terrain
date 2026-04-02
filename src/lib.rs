mod chunking;
mod components;
mod config;
mod debug;
mod lod;
mod material;
mod meshing;
mod sampling;
mod source;
mod systems;

pub use chunking::{TerrainChunkAddress, TerrainChunkKey, terrain_chunk_for_local};
pub use components::{
    TerrainBundle, TerrainChunk, TerrainChunkBounds, TerrainChunkReady, TerrainChunkRemoved,
    TerrainChunkState, TerrainColliderData, TerrainColliderPatch, TerrainColliderReady,
    TerrainFocus, TerrainFocusPoint, TerrainFocusPoints, TerrainProbe, TerrainProbeSample,
    TerrainRoot, TerrainRootStats, TerrainSourceHandle,
};
pub use config::{
    TerrainCacheConfig, TerrainColliderConfig, TerrainConfig, TerrainLodConfig,
    TerrainStreamingConfig,
};
pub use debug::{TerrainDebugColorMode, TerrainDebugConfig, TerrainDiagnostics};
pub use material::{TerrainBlendRange, TerrainLayer, TerrainLayerBlend, TerrainMaterialProfile};
pub use sampling::{
    TerrainSample, sample_height, sample_layer_weights, sample_normal, sample_terrain,
};
pub use source::{TerrainDataset, TerrainSource, TerrainWeightMap};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TerrainSystems {
    MaintainFocus,
    StreamChunks,
    BuildMeshes,
    BuildColliders,
    UpdateMaterials,
    Debug,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct TerrainPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl TerrainPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for TerrainPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_gizmo_group::<bevy::gizmos::config::DefaultGizmoConfigGroup>()
            .init_resource::<debug::TerrainDebugConfig>()
            .init_resource::<debug::TerrainDiagnostics>()
            .init_resource::<components::TerrainFocusPoints>()
            .init_resource::<systems::TerrainRuntimeState>()
            .init_resource::<systems::TerrainChunkCache>()
            .add_message::<components::TerrainChunkReady>()
            .add_message::<components::TerrainChunkRemoved>()
            .add_message::<components::TerrainColliderReady>()
            .register_type::<components::TerrainChunk>()
            .register_type::<components::TerrainChunkBounds>()
            .register_type::<components::TerrainChunkState>()
            .register_type::<components::TerrainFocus>()
            .register_type::<components::TerrainFocusPoint>()
            .register_type::<components::TerrainFocusPoints>()
            .register_type::<components::TerrainProbe>()
            .register_type::<components::TerrainProbeSample>()
            .register_type::<components::TerrainRoot>()
            .register_type::<components::TerrainRootStats>()
            .register_type::<config::TerrainCacheConfig>()
            .register_type::<config::TerrainColliderConfig>()
            .register_type::<config::TerrainConfig>()
            .register_type::<config::TerrainLodConfig>()
            .register_type::<config::TerrainStreamingConfig>()
            .register_type::<debug::TerrainDebugColorMode>()
            .register_type::<debug::TerrainDebugConfig>()
            .register_type::<debug::TerrainDiagnostics>()
            .register_type::<material::TerrainBlendRange>()
            .register_type::<material::TerrainLayer>()
            .register_type::<material::TerrainMaterialProfile>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    TerrainSystems::MaintainFocus,
                    TerrainSystems::UpdateMaterials,
                    TerrainSystems::StreamChunks,
                    TerrainSystems::BuildMeshes,
                    TerrainSystems::BuildColliders,
                    TerrainSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                systems::advance_runtime_frame
                    .in_set(TerrainSystems::MaintainFocus)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::sync_root_materials,
                    systems::update_probe_samples,
                    systems::update_diagnostics,
                )
                    .chain()
                    .in_set(TerrainSystems::UpdateMaterials)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::refresh_chunk_targets
                    .in_set(TerrainSystems::StreamChunks)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::queue_chunk_builds,
                    systems::poll_chunk_builds,
                    systems::prune_cache,
                )
                    .chain()
                    .in_set(TerrainSystems::BuildMeshes)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::sync_chunk_colliders
                    .in_set(TerrainSystems::BuildColliders)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                debug::draw_debug_gizmos
                    .in_set(TerrainSystems::Debug)
                    .run_if(systems::runtime_is_active),
            );
    }
}
