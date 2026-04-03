#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    pbr_deferred_functions::deferred_output,
    prepass_io::{FragmentOutput, VertexOutput},
}
#else
#import bevy_pbr::forward_io::{FragmentOutput, VertexOutput}
#endif

struct TerrainTextureUniform {
    base_color: vec4<f32>,
    surface: vec4<f32>,
    flags: vec4<f32>,
}

const PROJECTION_UV: f32 = 0.0;

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> terrain_texture: TerrainTextureUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var terrain_albedo_array: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var terrain_albedo_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var terrain_normal_array: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var terrain_normal_sampler: sampler;

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn terrain_layer(in: VertexOutput) -> i32 {
    return i32(clamp(round(in.uv_b.x), 0.0, terrain_texture.flags.z - 1.0));
}

fn terrain_tint(in: VertexOutput) -> vec3<f32> {
#ifdef VERTEX_COLORS
    return in.color.rgb;
#else
    return vec3<f32>(1.0);
#endif
}

fn use_triplanar() -> bool {
    return terrain_texture.flags.x > PROJECTION_UV + 0.5;
}

fn sample_albedo_uv(in: VertexOutput, layer: i32) -> vec3<f32> {
    let uv = in.uv * terrain_texture.surface.zw;
    return textureSample(terrain_albedo_array, terrain_albedo_sampler, uv, layer).rgb;
}

fn triplanar_blend_weights(normal: vec3<f32>) -> vec3<f32> {
    let blend = pow(abs(normalize(normal)), vec3<f32>(4.0));
    return blend / max(dot(blend, vec3<f32>(1.0)), 1e-4);
}

fn sample_albedo_triplanar(in: VertexOutput, layer: i32) -> vec3<f32> {
    let blend = triplanar_blend_weights(in.world_normal);
    let scale = terrain_texture.surface.zw;
    let sample_x = textureSample(
        terrain_albedo_array,
        terrain_albedo_sampler,
        in.world_position.yz * scale,
        layer,
    ).rgb;
    let sample_y = textureSample(
        terrain_albedo_array,
        terrain_albedo_sampler,
        in.world_position.xz * scale,
        layer,
    ).rgb;
    let sample_z = textureSample(
        terrain_albedo_array,
        terrain_albedo_sampler,
        in.world_position.xy * scale,
        layer,
    ).rgb;
    return sample_x * blend.x + sample_y * blend.y + sample_z * blend.z;
}

fn sample_albedo(in: VertexOutput, layer: i32) -> vec3<f32> {
    if use_triplanar() {
        return sample_albedo_triplanar(in, layer);
    }
    return sample_albedo_uv(in, layer);
}

fn sample_normal_map(in: VertexOutput, layer: i32) -> vec3<f32> {
    let uv = in.uv * terrain_texture.surface.zw;
    let encoded = textureSample(terrain_normal_array, terrain_normal_sampler, uv, layer).xyz;
    let tangent_space = normalize(vec3<f32>(
        (encoded.x * 2.0 - 1.0) * terrain_texture.flags.w,
        (encoded.y * 2.0 - 1.0) * terrain_texture.flags.w,
        encoded.z * 2.0 - 1.0,
    ));
    let base_normal = normalize(in.world_normal);
    let tangent = normalize(in.world_tangent.xyz);
    let bitangent = normalize(cross(base_normal, tangent) * in.world_tangent.w);
    return normalize(
        tangent * tangent_space.x +
        bitangent * tangent_space.y +
        base_normal * tangent_space.z
    );
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let layer = terrain_layer(in);
    let sampled_albedo = sample_albedo(in, layer);
    let tinted = sampled_albedo * terrain_tint(in) * terrain_texture.base_color.rgb;

    pbr_input.material.base_color = vec4<f32>(tinted, 1.0);
    pbr_input.material.perceptual_roughness = terrain_texture.surface.x;
    pbr_input.material.metallic = terrain_texture.surface.y;

#ifdef VERTEX_TANGENTS
    if terrain_texture.flags.y > 0.5 && !use_triplanar() {
        pbr_input.N = sample_normal_map(in, layer);
    }
#endif

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
