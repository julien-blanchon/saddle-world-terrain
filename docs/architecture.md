# Architecture

## Chosen LOD Strategy

The v1 runtime uses a **CDLOD-inspired distance-band selector** on top of **fixed-size streamed chunks**.

Why this choice:

- geometry clipmaps are excellent once the renderer is fully specialized around reusable ring meshes and toroidal updates
- the first shipped crate needs a stronger CPU-truth data model, sampling API, and debug surface than a renderer-first clipmap path
- fixed chunk sizes keep residency, sampling, caching, and backend-agnostic collider payloads straightforward
- CDLOD-style distance bands, hysteresis, and concentric focus-driven residency still shape the API toward larger-world evolution later

What was adopted from the references:

- from CDLOD: distance-band LOD selection, quadtree-style power-of-two range thinking, hysteresis to reduce threshold chatter, and chunk-local grid reuse
- from geometry clipmaps: focus-centric residency, stable concentric update behavior, and the design goal of keeping transitions temporally stable as the focus moves
- from chunked clipmap prior art: keeping terrain data as CPU-truth separate from render chunks, with the runtime free to rebuild visual chunks or collider payloads from the same source

What was rejected for v1:

- full clipmap ring meshes and toroidal texture updates
  Reason: that pushes the crate too far toward a specialized renderer before the reusable data and sampling interfaces are stable.
- built-in GPU morphing and packed fine/coarse height textures
  Reason: the first pass favors a small, testable CPU pipeline.
- a physics-backend dependency
  Reason: shared crates in this workspace stay Bevy-only on the runtime surface.

## Chunk Lifecycle

`TerrainRoot` owns the static source handle and configuration.

Each frame:

1. focus entities and explicit `TerrainFocusPoints` are transformed into terrain-local space
2. the runtime computes the desired chunk coordinates inside the configured visual radius
3. each chunk coordinate resolves to a target LOD using distance bands and hysteresis
4. missing chunks spawn, obsolete chunks despawn, and changed chunks are marked dirty
5. dirty chunks either reuse cached build artifacts or queue async rebuild tasks
6. ready build artifacts attach a mesh, bounds, and optional collider payload

Chunk residency is therefore driven by focus positions, not by camera assumptions.

## Mesh Generation Flow

The meshing pipeline is CPU-based and deterministic:

1. choose a vertex resolution for the chunk LOD
2. sample height, normal, and explicit weight channels on the source
3. evaluate material-layer blending from explicit weights plus height and slope rules
4. bake vertex colors for the active mesh debug mode or natural mode
5. append skirts around the chunk perimeter
6. emit one `Mesh` plus chunk-local bounds

Chunks are meshed in chunk-local coordinates and attached under chunk entities whose transforms carry the terrain-local origin. The same source samples drive both gameplay sampling and visual meshes, which keeps seams, probes, and debug overlays consistent.

## Material Layering Flow

`TerrainMaterialProfile` stays renderer-neutral and describes layer logic rather than shader internals.

For each sample:

1. read explicit weight channels from the terrain source
2. evaluate optional height ranges
3. evaluate optional slope ranges
4. multiply the contributions, normalize, and choose a dominant layer
5. blend the final tint into vertex colors

This v1 path intentionally uses `StandardMaterial` plus vertex colors. It is practical, stable across chunk boundaries, and easy to debug. A later custom material path can reuse the same layer description types.

## Collider Lifecycle

Collider generation is **data generation**, not built-in physics binding.

When collider generation is enabled:

1. chunk builds also bake a lower-resolution `TerrainColliderPatch`
2. the runtime keeps the patch in CPU memory on the chunk runtime state
3. only chunks inside the collider radius receive the public `TerrainColliderData` component
4. collider patches are chunk-local so consumers can attach heightfields, triangle meshes, or nav bake inputs directly to the chunk entity without re-offsetting them

This keeps the crate portable across Avian, Rapier, custom nav preprocessors, or no physics at all.

## Async Job Lifecycle

Chunk builds run on `AsyncComputeTaskPool`.

- every dirty chunk increments a build generation
- queued tasks carry that generation number
- if the chunk changes again before the task completes, the old result is rejected as stale
- stale results are not attached to the entity and the chunk returns to `Queued`
- completed results are cached by terrain root, revision, chunk key, and debug color mode

The async path exists to bound spikes while still keeping the runtime pure Bevy.

## Seam Prevention Strategy

The v1 seam strategy is intentionally simple and robust:

- same-LOD chunk borders are consistent because both sides sample the same source positions
- chunk transforms only apply the chunk origin once, so neighboring tiles meet in world space instead of being double-offset
- normals stay consistent because edge normals sample outside the chunk interior from the same source truth
- different-LOD borders use skirts to hide cracks
- hysteresis reduces visible thrash when chunks hover near distance thresholds

Deferred idea:

- geomorphing between fine and coarse height samples
  Reason deferred: it is worth doing, but it wants a dedicated shader path or per-vertex delta packing to stay efficient.

## CPU vs GPU Responsibilities

CPU:

- source sampling
- residency decisions
- LOD selection and hysteresis
- mesh generation
- collider payload generation
- debug diagnostics

GPU:

- standard PBR rendering of the baked mesh
- Bevy visibility, lighting, and material evaluation

This split favors testability and portable integration over raw maximum throughput in the first pass.

## Large-World Evolution

The API is shaped so the runtime can grow without breaking the consumer-facing contracts:

- `TerrainSource` can later back tiled datasets or procedural streaming
- `TerrainChunkReady` and `TerrainColliderReady` stay valid if the renderer moves to clipmap rings
- debug color modes and sampling helpers are defined on terrain truth, not on one mesh topology

Documented deferrals:

- full geometry clipmap mesh reuse
- geomorphing
- hole masks
- source-asset hot reload beyond replacing the terrain bundle's source handle
- runtime editing and brush tools
- floating-origin helpers beyond transform-relative sampling

Those are deferred because the shared crate needs a reliable terrain truth model first.
