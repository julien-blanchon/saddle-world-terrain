use bevy::{color::LinearRgba, prelude::*, reflect::Reflect};

#[derive(Reflect, Clone, Copy, Debug)]
pub struct TerrainBlendRange {
    pub start: f32,
    pub end: f32,
    pub falloff: f32,
}

impl TerrainBlendRange {
    pub fn new(start: f32, end: f32) -> Self {
        Self {
            start,
            end,
            falloff: 0.15,
        }
    }

    pub fn weight(self, value: f32) -> f32 {
        let width = (self.end - self.start).abs().max(f32::EPSILON);
        let normalized = ((value - self.start) / width).clamp(0.0, 1.0);
        let feather = self.falloff.clamp(0.0, 1.0) * 0.5;
        smoothstep(feather, 1.0 - feather, normalized)
    }
}

impl Default for TerrainBlendRange {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainLayer {
    pub name: String,
    pub color: Color,
    pub texture_index: Option<u32>,
    pub explicit_weight_index: Option<usize>,
    pub height_range: Option<TerrainBlendRange>,
    pub slope_range_degrees: Option<TerrainBlendRange>,
    pub strength: f32,
}

impl TerrainLayer {
    pub fn tinted(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            color,
            texture_index: None,
            explicit_weight_index: None,
            height_range: None,
            slope_range_degrees: None,
            strength: 1.0,
        }
    }

    pub fn with_weight_channel(mut self, channel: usize) -> Self {
        self.explicit_weight_index = Some(channel);
        self
    }

    pub fn with_texture_index(mut self, texture_index: u32) -> Self {
        self.texture_index = Some(texture_index);
        self
    }

    pub fn with_height_range(mut self, range: TerrainBlendRange) -> Self {
        self.height_range = Some(range);
        self
    }

    pub fn with_slope_range(mut self, range: TerrainBlendRange) -> Self {
        self.slope_range_degrees = Some(range);
        self
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainTextureProjection {
    #[default]
    Uv,
    Triplanar,
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainTextureArraySettings {
    pub albedo_array: Handle<Image>,
    pub normal_array: Option<Handle<Image>>,
    pub scale: Vec2,
    pub projection: TerrainTextureProjection,
    pub normal_map_strength: f32,
}

impl Default for TerrainTextureArraySettings {
    fn default() -> Self {
        Self {
            albedo_array: Handle::default(),
            normal_array: None,
            scale: Vec2::splat(1.0 / 12.0),
            projection: TerrainTextureProjection::Uv,
            normal_map_strength: 1.0,
        }
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct TerrainMaterialProfile {
    pub base_color: Color,
    pub perceptual_roughness: f32,
    pub metallic: f32,
    pub double_sided: bool,
    pub layers: Vec<TerrainLayer>,
    pub texture_arrays: Option<TerrainTextureArraySettings>,
}

impl Default for TerrainMaterialProfile {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            metallic: 0.0,
            double_sided: false,
            layers: vec![TerrainLayer::tinted(
                "Terrain",
                Color::srgb(0.62, 0.64, 0.66),
            )],
            texture_arrays: None,
        }
    }
}

impl TerrainMaterialProfile {
    pub fn standard_material(&self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.base_color,
            perceptual_roughness: self.perceptual_roughness,
            metallic: self.metallic,
            double_sided: self.double_sided,
            ..default()
        }
    }

    pub fn uses_texture_arrays(&self) -> bool {
        self.texture_arrays.is_some()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TerrainLayerBlend {
    pub dominant_layer: Option<usize>,
    pub dominant_color: Color,
    pub weights: Vec<f32>,
    pub color: Color,
}

pub fn evaluate_layer_blend(
    profile: &TerrainMaterialProfile,
    height_normalized: f32,
    slope_degrees: f32,
    explicit_weights: &[f32],
) -> TerrainLayerBlend {
    if profile.layers.is_empty() {
        return TerrainLayerBlend {
            dominant_layer: None,
            dominant_color: profile.base_color,
            weights: Vec::new(),
            color: profile.base_color,
        };
    }

    let mut raw_weights = Vec::with_capacity(profile.layers.len());
    for layer in &profile.layers {
        let explicit = layer
            .explicit_weight_index
            .and_then(|index| explicit_weights.get(index).copied())
            .unwrap_or(1.0);
        let height_weight = layer
            .height_range
            .map(|range| range.weight(height_normalized))
            .unwrap_or(1.0);
        let slope_weight = layer
            .slope_range_degrees
            .map(|range| range.weight(slope_degrees))
            .unwrap_or(1.0);

        raw_weights
            .push((explicit * height_weight * slope_weight * layer.strength.max(0.0)).max(0.0));
    }

    let sum: f32 = raw_weights.iter().sum();
    let weights = if sum <= f32::EPSILON {
        let mut fallback = vec![0.0; raw_weights.len()];
        fallback[0] = 1.0;
        fallback
    } else {
        raw_weights.into_iter().map(|value| value / sum).collect()
    };

    let dominant_layer = weights
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index);

    let mut linear = LinearRgba::BLACK;
    for (layer, weight) in profile.layers.iter().zip(weights.iter().copied()) {
        let tint = layer.color.to_linear();
        linear.red += tint.red * weight;
        linear.green += tint.green * weight;
        linear.blue += tint.blue * weight;
        linear.alpha += tint.alpha * weight;
    }

    let dominant_color = dominant_layer
        .and_then(|index| profile.layers.get(index))
        .map(|layer| layer.color)
        .unwrap_or(profile.base_color);

    TerrainLayerBlend {
        dominant_layer,
        dominant_color,
        weights,
        color: Color::LinearRgba(linear),
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(f32::EPSILON)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
