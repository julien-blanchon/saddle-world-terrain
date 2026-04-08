# Configuration

## `TerrainConfig`

| Field | Type | Default | Effect | Performance notes |
| --- | --- | --- | --- | --- |
| `size` | `Vec2` | `Vec2::new(512.0, 512.0)` | Local XZ extent of the terrain root | Larger extents increase chunk counts |
| `chunk_size` | `Vec2` | `Vec2::new(32.0, 32.0)` | World-space size of each streamed chunk in terrain-local units | Smaller chunks stream more often but localize rebuilds |
| `vertex_resolution` | `u32` | `64` | Base vertex density for LOD 0 | Higher values increase build time and vertex count |
| `height_scale` | `f32` | `72.0` | Converts normalized source heights into local Y distance | No direct cost; affects normal steepness |
| `height_offset` | `f32` | `0.0` | Adds a constant vertical bias to all samples | No direct cost |
| `skirt_depth` | `f32` | `4.0` | Depth of edge skirts used to hide LOD cracks | Larger skirts hide more cracks but can show on extreme silhouettes |
| `normal_sample_distance` | `f32` | `1.5` | Sample spacing used for central-difference normals | Larger values smooth normals; smaller values show more local detail |
| `lod` | `TerrainLodConfig` | see below | Distance-band LOD behavior | Controls visible density and rebuild churn |
| `streaming` | `TerrainStreamingConfig` | see below | Chunk residency and build budget | Direct impact on active chunk count and spikes |
| `collider` | `TerrainColliderConfig` | see below | Backend-agnostic collider payload baking | Enabled colliders add CPU work and memory |
| `cache` | `TerrainCacheConfig` | see below | Build artifact cache size | Larger cache reduces rebuilds at the cost of memory |
| `material` | `TerrainMaterialProfile` | one neutral layer | Material-layer blending and `StandardMaterial` template | More layers increase per-vertex evaluation cost |

## `TerrainStreamingConfig`

| Field | Type | Default | Effect | Performance notes |
| --- | --- | --- | --- | --- |
| `visual_radius` | `f32` | `320.0` | Radius around each focus that keeps visual chunks alive | Larger radius increases chunk count |
| `collider_radius` | `f32` | `128.0` | Radius around each focus that keeps collider payloads attached | Keep smaller than `visual_radius` unless needed |
| `max_builds_per_frame` | `usize` | `6` | Caps new async chunk jobs started per update | Lower values smooth spikes but delay visibility |

## `TerrainLodConfig`

| Field | Type | Default | Effect | Performance notes |
| --- | --- | --- | --- | --- |
| `lod_count` | `u8` | `5` | Number of distance bands | More levels add flexibility but more cache variants |
| `near_distance` | `f32` | `48.0` | Distance threshold for LOD 0 | Smaller values push coarser meshes sooner |
| `distance_multiplier` | `f32` | `2.0` | Growth factor between LOD bands | Powers-of-two align with clipmap and CDLOD thinking |
| `hysteresis` | `f32` | `8.0` | Buffer around thresholds to reduce churn | Higher values improve stability and delay transitions |
| `minimum_vertex_resolution` | `u32` | `8` | Floor for far-field vertex density | Prevents extremely coarse chunks |

## `TerrainColliderConfig`

| Field | Type | Default | Effect | Performance notes |
| --- | --- | --- | --- | --- |
| `enabled` | `bool` | `false` | Enables collider payload baking during chunk builds | Adds CPU work for each built chunk |
| `resolution_divisor` | `u32` | `2` | Lowers collider grid density relative to the visual mesh | Higher divisors reduce memory and bake cost |

## `TerrainCacheConfig`

| Field | Type | Default | Effect | Performance notes |
| --- | --- | --- | --- | --- |
| `max_entries` | `usize` | `256` | Maximum cached chunk build artifacts for a single terrain root | Increase if focus points revisit the same regions often |

## `TerrainMaterialProfile`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `base_color` | `Color` | `Color::WHITE` | Multiplies the baked vertex colors through `StandardMaterial` |
| `perceptual_roughness` | `f32` | `0.95` | Shared roughness for the terrain material |
| `metallic` | `f32` | `0.0` | Shared metallic value |
| `double_sided` | `bool` | `false` | Enables two-sided rendering when the terrain is viewed from below |
| `layers` | `Vec<TerrainLayer>` | one neutral layer | Defines the reusable layer blending rules |
| `texture_arrays` | `Option<TerrainTextureArraySettings>` | `None` | Enables the textured terrain material path with array-backed albedo and optional normals |

## `TerrainLayer`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `name` | `String` | required | Logical layer name used by consumers and debugging |
| `color` | `Color` | required | Vertex-color tint baked into the terrain mesh |
| `texture_index` | `Option<u32>` | `None` | Selects the texture-array slice used when `texture_arrays` is enabled; defaults to the layer index |
| `explicit_weight_index` | `Option<usize>` | `None` | Reads a flattened explicit weight channel from the source |
| `height_range` | `Option<TerrainBlendRange>` | `None` | Applies a normalized-height weight gate |
| `slope_range_degrees` | `Option<TerrainBlendRange>` | `None` | Applies a slope-in-degrees weight gate |
| `strength` | `f32` | `1.0` | Multiplies the total layer contribution before normalization |

## `TerrainTextureArraySettings`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `albedo_array` | `Handle<Image>` | default handle | Texture array sampled for terrain albedo |
| `normal_array` | `Option<Handle<Image>>` | `None` | Optional texture array sampled for terrain normals |
| `scale` | `Vec2` | `Vec2::splat(1.0 / 12.0)` | UV or triplanar texture repetition scale |
| `projection` | `TerrainTextureProjection` | `Uv` | Chooses UV or triplanar projection in the textured shader path |
| `normal_map_strength` | `f32` | `1.0` | Strength multiplier applied to sampled normal maps |

## `TerrainDebugConfig`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `show_chunk_bounds` | `bool` | `false` | Draw chunk AABBs |
| `show_focus_rings` | `bool` | `false` | Draw visual and collider radii around active focus entities and explicit `TerrainFocusPoints` entries |
| `show_collider_bounds` | `bool` | `false` | Highlight chunks with collider payloads attached |
| `show_sample_probes` | `bool` | `false` | Draw probe hit points and normals |
| `color_mode` | `TerrainDebugColorMode` | `Natural` | Chooses natural shading, mesh debug colors, or chunk-state bound colors |

## `TerrainDiagnostics`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `active_roots` | `u32` | `0` | Number of active terrain root entities |
| `total_chunks` | `u32` | `0` | Total spawned chunk entities |
| `pending_chunks` | `u32` | `0` | Chunks queued or building |
| `ready_chunks` | `u32` | `0` | Chunks with meshes attached |
| `collider_chunks` | `u32` | `0` | Chunks with collider payloads |
| `cache_entries` | `u32` | `0` | Cached chunk build artifacts |
| `focus_points` | `u32` | `0` | Active focus entities plus explicit focus points |
| `estimated_vertex_count` | `u64` | `0` | Estimated total vertices across all ready chunks |
| `estimated_triangle_count` | `u64` | `0` | Estimated total triangles across all ready chunks |

## Helper Methods on `TerrainConfig`

| Method | Returns | Description |
| --- | --- | --- |
| `total_chunk_count()` | `u32` | Total chunks that would cover the full terrain |
| `chunk_vertex_count(lod)` | `u32` | Estimated vertex count for a single chunk at the given LOD |
| `chunk_dimensions()` | `UVec2` | Grid dimensions in chunk units |
| `with_size(size)` | `Self` | Builder: set terrain size |
| `with_chunk_size(size)` | `Self` | Builder: set chunk size |
| `with_vertex_resolution(res)` | `Self` | Builder: set vertex resolution |
| `with_height_scale(scale)` | `Self` | Builder: set height scale |
| `with_height_offset(offset)` | `Self` | Builder: set height offset |
| `with_lod(config)` | `Self` | Builder: set LOD configuration |
| `with_streaming(config)` | `Self` | Builder: set streaming configuration |
| `with_collider(config)` | `Self` | Builder: set collider configuration |
| `with_material(profile)` | `Self` | Builder: set material profile |

## Valid Ranges And Guidance

- `vertex_resolution` should be at least `8` if skirts are enabled and at least `4` for collider payloads.
- `distance_multiplier` should stay above `1.0`; powers of two work best.
- `collider_radius` is usually best at `25%` to `60%` of `visual_radius`.
- `normal_sample_distance` should roughly match the spatial scale of the terrain texels after `size` is applied.
- For large open worlds, prefer `vertex_resolution` of 48 over 64 — the visual difference is minimal but vertex count drops by ~44%.
- Monitor `TerrainDiagnostics::estimated_vertex_count` at runtime to detect geometry budgets spiraling out of control.
