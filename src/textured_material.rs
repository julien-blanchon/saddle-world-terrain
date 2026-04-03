use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial},
    prelude::*,
    reflect::Reflect,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType},
        texture::GpuImage,
    },
    shader::{Shader, ShaderRef},
};

use crate::material::{TerrainMaterialProfile, TerrainTextureProjection};

pub type TerrainTextureMaterial = ExtendedMaterial<StandardMaterial, TerrainTextureExtension>;

pub(crate) const TERRAIN_TEXTURE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("d49130ab-f0d6-4307-9614-f14f5ec8a2f9");

#[derive(Clone, Debug, ShaderType)]
pub(crate) struct TerrainTextureGpuUniform {
    pub base_color: Vec4,
    pub surface: Vec4,
    pub flags: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
#[uniform(100, TerrainTextureGpuUniform)]
#[reflect(Default, Debug)]
pub struct TerrainTextureExtension {
    pub base_color: Color,
    pub perceptual_roughness: f32,
    pub metallic: f32,
    pub scale: Vec2,
    pub projection: TerrainTextureProjection,
    pub normal_map_strength: f32,
    pub layer_count: u32,
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo_array: Handle<Image>,
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    pub normal_array: Option<Handle<Image>>,
}

impl TerrainTextureExtension {
    pub fn from_profile(profile: &TerrainMaterialProfile) -> Option<Self> {
        let texture_arrays = profile.texture_arrays.clone()?;
        Some(Self {
            base_color: profile.base_color,
            perceptual_roughness: profile.perceptual_roughness,
            metallic: profile.metallic,
            scale: texture_arrays.scale,
            projection: texture_arrays.projection,
            normal_map_strength: texture_arrays.normal_map_strength,
            layer_count: profile.layers.len().max(1) as u32,
            albedo_array: texture_arrays.albedo_array,
            normal_array: texture_arrays.normal_array,
        })
    }
}

impl AsBindGroupShaderType<TerrainTextureGpuUniform> for TerrainTextureExtension {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<GpuImage>,
    ) -> TerrainTextureGpuUniform {
        TerrainTextureGpuUniform {
            base_color: self.base_color.to_linear().to_vec4(),
            surface: Vec4::new(
                self.perceptual_roughness,
                self.metallic,
                self.scale.x,
                self.scale.y,
            ),
            flags: Vec4::new(
                match self.projection {
                    TerrainTextureProjection::Uv => 0.0,
                    TerrainTextureProjection::Triplanar => 1.0,
                },
                if self.normal_array.is_some() { 1.0 } else { 0.0 },
                self.layer_count as f32,
                self.normal_map_strength,
            ),
        }
    }
}

impl MaterialExtension for TerrainTextureExtension {
    fn fragment_shader() -> ShaderRef {
        TERRAIN_TEXTURE_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        TERRAIN_TEXTURE_SHADER_HANDLE.into()
    }
}

pub(crate) fn build_textured_material(
    profile: &TerrainMaterialProfile,
) -> Option<TerrainTextureMaterial> {
    let extension = TerrainTextureExtension::from_profile(profile)?;
    Some(TerrainTextureMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: profile.perceptual_roughness,
            metallic: profile.metallic,
            double_sided: profile.double_sided,
            cull_mode: if profile.double_sided {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            ..default()
        },
        extension,
    })
}

pub(crate) fn plugin(app: &mut App) {
    load_internal_asset!(
        app,
        TERRAIN_TEXTURE_SHADER_HANDLE,
        "shaders/terrain_texture.wgsl",
        Shader::from_wgsl
    );
    app.add_plugins(MaterialPlugin::<TerrainTextureMaterial>::default());
}
