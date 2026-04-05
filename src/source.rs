use std::sync::Arc;

use bevy::{prelude::*, render::render_resource::TextureFormat};

pub trait TerrainSource: Send + Sync + 'static {
    fn height_dimensions(&self) -> UVec2;
    fn sample_height(&self, uv: Vec2) -> f32;
    fn explicit_weight_channel_count(&self) -> usize;
    fn sample_explicit_weight(&self, channel: usize, uv: Vec2) -> f32;

    fn sample_hole(&self, _uv: Vec2) -> f32 {
        0.0
    }
}

#[derive(Clone, Debug)]
pub struct TerrainWeightMap {
    pub dimensions: UVec2,
    data: Arc<[[f32; 4]]>,
}

impl TerrainWeightMap {
    pub fn from_rgba(dimensions: UVec2, weights: Vec<[f32; 4]>) -> Result<Self, String> {
        if dimensions.x == 0 || dimensions.y == 0 {
            return Err("weight map dimensions must be greater than zero".into());
        }

        let expected = dimensions.x as usize * dimensions.y as usize;
        if expected != weights.len() {
            return Err(format!(
                "weight map expected {expected} texels but received {}",
                weights.len()
            ));
        }

        Ok(Self {
            dimensions,
            data: Arc::from(weights),
        })
    }

    pub fn sample_channel(&self, channel: usize, uv: Vec2) -> f32 {
        if channel >= 4 {
            return 0.0;
        }

        sample_bilinear_rgba(self.dimensions, &self.data, uv)[channel]
    }
}

#[derive(Clone, Debug)]
pub struct TerrainDataset {
    dimensions: UVec2,
    heights: Arc<[f32]>,
    weight_maps: Vec<TerrainWeightMap>,
    hole_mask: Option<TerrainHoleMask>,
}

#[derive(Clone, Debug)]
pub struct TerrainHoleMask {
    pub dimensions: UVec2,
    data: Arc<[f32]>,
}

impl TerrainHoleMask {
    pub fn from_values(dimensions: UVec2, values: Vec<f32>) -> Result<Self, String> {
        if dimensions.x == 0 || dimensions.y == 0 {
            return Err("hole mask dimensions must be greater than zero".into());
        }

        let expected = dimensions.x as usize * dimensions.y as usize;
        if expected != values.len() {
            return Err(format!(
                "hole mask expected {expected} texels but received {}",
                values.len()
            ));
        }

        Ok(Self {
            dimensions,
            data: Arc::from(values),
        })
    }

    pub fn sample(&self, uv: Vec2) -> f32 {
        sample_bilinear_scalar(self.dimensions, &self.data, uv)
    }
}

impl TerrainDataset {
    pub fn from_heights(dimensions: UVec2, heights: Vec<f32>) -> Result<Self, String> {
        if dimensions.x == 0 || dimensions.y == 0 {
            return Err("height dataset dimensions must be greater than zero".into());
        }

        let expected = dimensions.x as usize * dimensions.y as usize;
        if expected != heights.len() {
            return Err(format!(
                "height dataset expected {expected} texels but received {}",
                heights.len()
            ));
        }

        Ok(Self {
            dimensions,
            heights: Arc::from(heights),
            weight_maps: Vec::new(),
            hole_mask: None,
        })
    }

    pub fn from_fn(
        dimensions: UVec2,
        mut height_fn: impl FnMut(UVec2, Vec2) -> f32,
    ) -> Result<Self, String> {
        let mut heights = Vec::with_capacity(dimensions.x as usize * dimensions.y as usize);
        for y in 0..dimensions.y {
            for x in 0..dimensions.x {
                let uv = Vec2::new(
                    if dimensions.x <= 1 {
                        0.0
                    } else {
                        x as f32 / (dimensions.x - 1) as f32
                    },
                    if dimensions.y <= 1 {
                        0.0
                    } else {
                        y as f32 / (dimensions.y - 1) as f32
                    },
                );
                heights.push(height_fn(UVec2::new(x, y), uv));
            }
        }

        Self::from_heights(dimensions, heights)
    }

    /// Generate a dataset from a function that returns both height and RGBA weights.
    ///
    /// This is a convenience for `from_fn` + `with_weight_map` in a single pass.
    pub fn from_fn_with_weights(
        dimensions: UVec2,
        mut generator: impl FnMut(UVec2, Vec2) -> (f32, [f32; 4]),
    ) -> Result<Self, String> {
        let count = dimensions.x as usize * dimensions.y as usize;
        let mut heights = Vec::with_capacity(count);
        let mut weights = Vec::with_capacity(count);
        for y in 0..dimensions.y {
            for x in 0..dimensions.x {
                let uv = Vec2::new(
                    if dimensions.x <= 1 {
                        0.0
                    } else {
                        x as f32 / (dimensions.x - 1) as f32
                    },
                    if dimensions.y <= 1 {
                        0.0
                    } else {
                        y as f32 / (dimensions.y - 1) as f32
                    },
                );
                let (height, weight) = generator(UVec2::new(x, y), uv);
                heights.push(height);
                weights.push(weight);
            }
        }

        let dataset = Self::from_heights(dimensions, heights)?;
        let weight_map = TerrainWeightMap::from_rgba(dimensions, weights)?;
        Ok(dataset.with_weight_map(weight_map))
    }

    pub fn with_weight_map(mut self, map: TerrainWeightMap) -> Self {
        self.weight_maps.push(map);
        self
    }

    pub fn with_hole_mask(mut self, hole_mask: TerrainHoleMask) -> Self {
        self.hole_mask = Some(hole_mask);
        self
    }

    pub fn from_height_image(image: &Image) -> Result<Self, String> {
        let dimensions = UVec2::new(
            image.texture_descriptor.size.width,
            image.texture_descriptor.size.height,
        );
        let format = image.texture_descriptor.format;
        let data = image
            .data
            .as_ref()
            .ok_or_else(|| "height image does not contain CPU-readable data".to_string())?;
        let mut heights = Vec::with_capacity(dimensions.x as usize * dimensions.y as usize);

        match format {
            TextureFormat::R8Unorm => {
                for byte in data.iter().copied() {
                    heights.push(byte as f32 / 255.0);
                }
            }
            TextureFormat::R16Unorm => {
                for bytes in data.chunks_exact(2) {
                    let value = u16::from_le_bytes([bytes[0], bytes[1]]);
                    heights.push(value as f32 / u16::MAX as f32);
                }
            }
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
                for texel in data.chunks_exact(4) {
                    let luminance = (0.2126 * texel[0] as f32
                        + 0.7152 * texel[1] as f32
                        + 0.0722 * texel[2] as f32)
                        / 255.0;
                    heights.push(luminance);
                }
            }
            _ => {
                return Err(format!(
                    "unsupported height image format {format:?}; expected R8, R16, or RGBA8"
                ));
            }
        }

        Self::from_heights(dimensions, heights)
    }

    pub fn weight_map_from_image(image: &Image) -> Result<TerrainWeightMap, String> {
        let dimensions = UVec2::new(
            image.texture_descriptor.size.width,
            image.texture_descriptor.size.height,
        );
        let data = image
            .data
            .as_ref()
            .ok_or_else(|| "weight image does not contain CPU-readable data".to_string())?;
        match image.texture_descriptor.format {
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
                let weights = data
                    .chunks_exact(4)
                    .map(|texel| {
                        [
                            texel[0] as f32 / 255.0,
                            texel[1] as f32 / 255.0,
                            texel[2] as f32 / 255.0,
                            texel[3] as f32 / 255.0,
                        ]
                    })
                    .collect();
                TerrainWeightMap::from_rgba(dimensions, weights)
            }
            _ => Err(format!(
                "unsupported weight image format {:?}; expected RGBA8",
                image.texture_descriptor.format
            )),
        }
    }

    pub fn hole_mask_from_image(image: &Image) -> Result<TerrainHoleMask, String> {
        let dimensions = UVec2::new(
            image.texture_descriptor.size.width,
            image.texture_descriptor.size.height,
        );
        let data = image
            .data
            .as_ref()
            .ok_or_else(|| "hole image does not contain CPU-readable data".to_string())?;

        let values = match image.texture_descriptor.format {
            TextureFormat::R8Unorm => data.iter().map(|byte| *byte as f32 / 255.0).collect(),
            TextureFormat::R16Unorm => data
                .chunks_exact(2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as f32 / u16::MAX as f32)
                .collect(),
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => data
                .chunks_exact(4)
                .map(|texel| texel[3] as f32 / 255.0)
                .collect(),
            _ => {
                return Err(format!(
                    "unsupported hole image format {:?}; expected R8, R16, or RGBA8",
                    image.texture_descriptor.format
                ));
            }
        };

        TerrainHoleMask::from_values(dimensions, values)
    }
}

impl TerrainSource for TerrainDataset {
    fn height_dimensions(&self) -> UVec2 {
        self.dimensions
    }

    fn sample_height(&self, uv: Vec2) -> f32 {
        sample_bilinear_scalar(self.dimensions, &self.heights, uv)
    }

    fn explicit_weight_channel_count(&self) -> usize {
        self.weight_maps.len() * 4
    }

    fn sample_explicit_weight(&self, channel: usize, uv: Vec2) -> f32 {
        let map_index = channel / 4;
        let channel_index = channel % 4;
        self.weight_maps
            .get(map_index)
            .map(|map| map.sample_channel(channel_index, uv))
            .unwrap_or(0.0)
    }

    fn sample_hole(&self, uv: Vec2) -> f32 {
        self.hole_mask
            .as_ref()
            .map(|mask| mask.sample(uv))
            .unwrap_or(0.0)
    }
}

fn sample_bilinear_scalar(dimensions: UVec2, values: &[f32], uv: Vec2) -> f32 {
    let clamped = uv.clamp(Vec2::ZERO, Vec2::ONE);
    let x = clamped.x * (dimensions.x.saturating_sub(1)) as f32;
    let y = clamped.y * (dimensions.y.saturating_sub(1)) as f32;
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(dimensions.x.saturating_sub(1));
    let y1 = (y0 + 1).min(dimensions.y.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let get = |x: u32, y: u32| -> f32 { values[(y * dimensions.x + x) as usize] };
    let a = get(x0, y0);
    let b = get(x1, y0);
    let c = get(x0, y1);
    let d = get(x1, y1);
    let ab = a.lerp(b, tx);
    let cd = c.lerp(d, tx);
    ab.lerp(cd, ty)
}

fn sample_bilinear_rgba(dimensions: UVec2, values: &[[f32; 4]], uv: Vec2) -> [f32; 4] {
    let clamped = uv.clamp(Vec2::ZERO, Vec2::ONE);
    let x = clamped.x * (dimensions.x.saturating_sub(1)) as f32;
    let y = clamped.y * (dimensions.y.saturating_sub(1)) as f32;
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(dimensions.x.saturating_sub(1));
    let y1 = (y0 + 1).min(dimensions.y.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let get = |x: u32, y: u32| -> [f32; 4] { values[(y * dimensions.x + x) as usize] };
    let mix4 = |a: [f32; 4], b: [f32; 4], t: f32| -> [f32; 4] {
        [
            a[0].lerp(b[0], t),
            a[1].lerp(b[1], t),
            a[2].lerp(b[2], t),
            a[3].lerp(b[3], t),
        ]
    };

    let ab = mix4(get(x0, y0), get(x1, y0), tx);
    let cd = mix4(get(x0, y1), get(x1, y1), tx);
    mix4(ab, cd, ty)
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
