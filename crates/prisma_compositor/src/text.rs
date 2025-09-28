/// GPU-accelerated text rendering using glyph atlas
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use fontdb::{Database, ID as FontId};
use ttf_parser::{Face, GlyphId};
use crate::{
    geometry::{Rect, Point, Size, Color},
    core::Device,
    assets::Texture,
};

/// Font weight enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Normal = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

/// Font style enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Font properties for font matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontProperties {
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl Default for FontProperties {
    fn default() -> Self {
        Self {
            family: "System".to_string(),
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }
    }
}

/// Glyph metrics and positioning information
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance_width: f32,
    pub left_side_bearing: f32,
    pub bounding_box: Rect,
}

/// Cached glyph information for atlas
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    pub metrics: GlyphMetrics,
    pub atlas_rect: Rect, // Position in atlas texture
    pub font_size: f32,
}

/// High-performance glyph atlas for GPU text rendering
pub struct GlyphAtlas {
    /// Atlas texture containing rasterized glyphs
    atlas_texture: Option<Texture>,
    /// Atlas size
    atlas_size: Size,
    /// Current packing position
    pack_position: Point,
    /// Row height for current row
    row_height: f32,
    /// Cached glyphs by (font_id, glyph_id, size_key)
    glyph_cache: HashMap<(FontId, GlyphId, u32), CachedGlyph>,
    /// Font data for rasterization
    font_data: HashMap<FontId, Vec<u8>>,
    /// Needs rebuild flag
    needs_rebuild: bool,
}

impl GlyphAtlas {
    /// Create a new glyph atlas
    pub fn new(atlas_size: Size) -> Self {
        Self {
            atlas_texture: None,
            atlas_size,
            pack_position: Point::new(0.0, 0.0),
            row_height: 0.0,
            glyph_cache: HashMap::new(),
            font_data: HashMap::new(),
            needs_rebuild: false,
        }
    }

    /// Add font data to the atlas
    pub fn add_font(&mut self, font_id: FontId, font_data: Vec<u8>) {
        self.font_data.insert(font_id, font_data);
    }

    /// Get or rasterize a glyph
    pub fn get_glyph(&mut self, font_id: FontId, glyph_id: GlyphId, font_size: f32) -> Option<&CachedGlyph> {
        let size_key = (font_size * 64.0) as u32; // Fixed-point for cache key
        let cache_key = (font_id, glyph_id, size_key);

        if !self.glyph_cache.contains_key(&cache_key) {
            if let Some(cached_glyph) = self.rasterize_glyph(font_id, glyph_id, font_size) {
                self.glyph_cache.insert(cache_key, cached_glyph);
                self.needs_rebuild = true;
            } else {
                return None;
            }
        }

        self.glyph_cache.get(&cache_key)
    }

    /// Rasterize a glyph and add it to the atlas
    fn rasterize_glyph(&mut self, font_id: FontId, glyph_id: GlyphId, font_size: f32) -> Option<CachedGlyph> {
        let font_data = self.font_data.get(&font_id)?;
        let face = Face::parse(font_data, 0).ok()?;

        // Get glyph metrics
        let units_per_em = face.units_per_em() as f32;
        let scale = font_size / units_per_em;

        let advance_width = face.glyph_hor_advance(glyph_id)? as f32 * scale;
        let left_side_bearing = face.glyph_hor_side_bearing(glyph_id)? as f32 * scale;

        // Get glyph bounding box
        let bbox = face.glyph_bounding_box(glyph_id)?;
        let bounding_box = Rect::new(
            bbox.x_min as f32 * scale,
            bbox.y_min as f32 * scale,
            (bbox.x_max - bbox.x_min) as f32 * scale,
            (bbox.y_max - bbox.y_min) as f32 * scale,
        );

        // Calculate raster size (add padding for anti-aliasing)
        let raster_width = (bounding_box.size.width.ceil() + 4.0) as u32;
        let raster_height = (bounding_box.size.height.ceil() + 4.0) as u32;

        // Check if we can fit this glyph in current row
        if self.pack_position.x + raster_width as f32 > self.atlas_size.width {
            // Move to next row
            self.pack_position.x = 0.0;
            self.pack_position.y += self.row_height;
            self.row_height = 0.0;
        }

        // Check if we have space in atlas
        if self.pack_position.y + raster_height as f32 > self.atlas_size.height {
            // Atlas is full - TODO: implement atlas resizing or multiple atlases
            eprintln!("Warning: Glyph atlas is full, cannot add more glyphs");
            return None;
        }

        // Reserve space in atlas
        let atlas_rect = Rect::new(
            self.pack_position.x,
            self.pack_position.y,
            raster_width as f32,
            raster_height as f32,
        );

        // Update packing position
        self.pack_position.x += raster_width as f32;
        self.row_height = self.row_height.max(raster_height as f32);

        // TODO: Actual glyph rasterization would go here
        // For now, we just reserve the space

        Some(CachedGlyph {
            metrics: GlyphMetrics {
                advance_width,
                left_side_bearing,
                bounding_box,
            },
            atlas_rect,
            font_size,
        })
    }

    /// Rebuild the atlas texture if needed
    pub fn rebuild_atlas_if_needed(&mut self, device: &Device) -> Result<()> {
        if !self.needs_rebuild && self.atlas_texture.is_some() {
            return Ok(());
        }

        // Create new atlas texture
        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: self.atlas_size.width as u32,
                height: self.atlas_size.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // Single channel for alpha
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = device.device.create_texture(&texture_descriptor);

        // TODO: Rasterize all glyphs and write to texture
        // For now, create empty texture
        let texture_data = vec![0u8; (self.atlas_size.width * self.atlas_size.height) as usize];

        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.atlas_size.width as u32),
                rows_per_image: Some(self.atlas_size.height as u32),
            },
            texture_descriptor.size,
        );

        self.atlas_texture = Some(Texture {
            texture: Arc::new(texture),
            size: self.atlas_size,
            format: wgpu::TextureFormat::R8Unorm,
        });

        self.needs_rebuild = false;
        Ok(())
    }

    /// Get the atlas texture
    pub fn texture(&self) -> Option<&Texture> {
        self.atlas_texture.as_ref()
    }

    /// Clear the atlas (for testing or font changes)
    pub fn clear(&mut self) {
        self.glyph_cache.clear();
        self.pack_position = Point::new(0.0, 0.0);
        self.row_height = 0.0;
        self.needs_rebuild = true;
    }
}

/// Font manager for loading and caching fonts
pub struct FontManager {
    /// Font database for system fonts
    database: Database,
    /// Custom loaded fonts
    custom_fonts: Vec<FontId>,
    /// Glyph atlas
    atlas: GlyphAtlas,
}

impl FontManager {
    /// Create a new font manager
    pub fn new() -> Self {
        let mut database = Database::new();
        database.load_system_fonts();

        Self {
            database,
            custom_fonts: Vec::new(),
            atlas: GlyphAtlas::new(Size::new(1024.0, 1024.0)), // 1024x1024 atlas
        }
    }

    /// Load a custom font from bytes
    pub fn load_font_from_bytes(&mut self, font_data: Vec<u8>) -> Result<FontId> {
        self.database.load_font_data(font_data.clone());
        // TODO: Get actual font ID after loading
        let font_id = FontId::default(); // Placeholder until proper implementation
        self.atlas.add_font(font_id, font_data);
        self.custom_fonts.push(font_id);
        Ok(font_id)
    }

    /// Find font by properties
    pub fn find_font(&self, properties: &FontProperties) -> Option<FontId> {
        let style = match properties.style {
            FontStyle::Normal => fontdb::Style::Normal,
            FontStyle::Italic => fontdb::Style::Italic,
            FontStyle::Oblique => fontdb::Style::Oblique,
        };

        let weight = fontdb::Weight(properties.weight as u16);

        self.database.query(&fontdb::Query {
            families: &[fontdb::Family::Name(&properties.family)],
            weight,
            stretch: fontdb::Stretch::Normal,
            style,
        })
    }

    /// Get glyph atlas
    pub fn atlas(&mut self) -> &mut GlyphAtlas {
        &mut self.atlas
    }

    /// Shape text into positioned glyphs (simplified for now)
    pub fn shape_text(&mut self, text: &str, font_id: FontId, font_size: f32) -> Vec<PositionedGlyph> {
        // TODO: Use a proper text shaping library like rustybuzz for complex scripts
        // For now, just do simple character-to-glyph mapping

        let mut positioned_glyphs = Vec::new();
        let mut x_offset = 0.0;

        for ch in text.chars() {
            if let Some(font_data) = self.atlas.font_data.get(&font_id) {
                if let Ok(face) = Face::parse(font_data, 0) {
                    if let Some(glyph_id) = face.glyph_index(ch) {
                        if let Some(cached_glyph) = self.atlas.get_glyph(font_id, glyph_id, font_size) {
                            positioned_glyphs.push(PositionedGlyph {
                                glyph_id,
                                position: Point::new(x_offset, 0.0),
                                cached_glyph: cached_glyph.clone(),
                            });

                            x_offset += cached_glyph.metrics.advance_width;
                        }
                    }
                }
            }
        }

        positioned_glyphs
    }

    /// Rebuild atlas if needed
    pub fn rebuild_atlas_if_needed(&mut self, device: &Device) -> Result<()> {
        self.atlas.rebuild_atlas_if_needed(device)
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Positioned glyph for text layout
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub glyph_id: GlyphId,
    pub position: Point,
    pub cached_glyph: CachedGlyph,
}

/// High-level text renderer
pub struct TextRenderer {
    font_manager: FontManager,
    default_font: Option<FontId>,
}

impl TextRenderer {
    /// Create a new text renderer
    pub fn new() -> Self {
        let font_manager = FontManager::new();

        // Try to find a good default font
        let default_font = font_manager.find_font(&FontProperties::default())
            .or_else(|| {
                // Fallback fonts
                font_manager.find_font(&FontProperties {
                    family: "Arial".to_string(),
                    ..FontProperties::default()
                })
            })
            .or_else(|| {
                font_manager.find_font(&FontProperties {
                    family: "Helvetica".to_string(),
                    ..FontProperties::default()
                })
            });

        Self {
            font_manager,
            default_font,
        }
    }

    /// Render text and return positioned glyphs
    pub fn render_text(
        &mut self,
        text: &str,
        font_properties: &FontProperties,
        font_size: f32,
        color: Color,
        position: Point,
    ) -> TextLayout {
        let font_id = self.font_manager.find_font(font_properties)
            .or(self.default_font)
            .unwrap_or_else(|| {
                eprintln!("Warning: No font found, text rendering may fail");
                FontId::default() // Fallback
            });

        let mut positioned_glyphs = self.font_manager.shape_text(text, font_id, font_size);

        // Apply base position offset
        for glyph in &mut positioned_glyphs {
            glyph.position.x += position.x;
            glyph.position.y += position.y;
        }

        let bounds = self.calculate_text_bounds(&positioned_glyphs);

        TextLayout {
            glyphs: positioned_glyphs,
            bounds,
            color,
            font_size,
        }
    }

    /// Calculate bounding box for text
    fn calculate_text_bounds(&self, glyphs: &[PositionedGlyph]) -> Rect {
        if glyphs.is_empty() {
            return Rect::ZERO;
        }

        let min_x = glyphs.iter()
            .map(|g| g.position.x + g.cached_glyph.metrics.bounding_box.origin.x)
            .fold(f32::INFINITY, f32::min);

        let max_x = glyphs.iter()
            .map(|g| g.position.x + g.cached_glyph.metrics.bounding_box.origin.x + g.cached_glyph.metrics.bounding_box.size.width)
            .fold(f32::NEG_INFINITY, f32::max);

        let min_y = glyphs.iter()
            .map(|g| g.position.y + g.cached_glyph.metrics.bounding_box.origin.y)
            .fold(f32::INFINITY, f32::min);

        let max_y = glyphs.iter()
            .map(|g| g.position.y + g.cached_glyph.metrics.bounding_box.origin.y + g.cached_glyph.metrics.bounding_box.size.height)
            .fold(f32::NEG_INFINITY, f32::max);

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Get font manager for atlas operations
    pub fn font_manager(&mut self) -> &mut FontManager {
        &mut self.font_manager
    }

    /// Load custom font
    pub fn load_font(&mut self, font_data: Vec<u8>) -> Result<FontId> {
        self.font_manager.load_font_from_bytes(font_data)
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Laid out text ready for rendering
#[derive(Debug, Clone)]
pub struct TextLayout {
    pub glyphs: Vec<PositionedGlyph>,
    pub bounds: Rect,
    pub color: Color,
    pub font_size: f32,
}

impl TextLayout {
    /// Convert to render commands for the GPU renderer
    pub fn to_render_commands(&self, z_index: f32) -> Vec<crate::renderer::RenderCommand> {
        self.glyphs.iter().map(|glyph| {
            crate::renderer::RenderCommand::Text {
                rect: Rect::new(
                    glyph.position.x,
                    glyph.position.y,
                    glyph.cached_glyph.atlas_rect.size.width,
                    glyph.cached_glyph.atlas_rect.size.height,
                ),
                glyph_atlas_region: glyph.cached_glyph.atlas_rect,
                color: self.color,
                transform: crate::geometry::Transform::identity(),
                z_index,
            }
        }).collect()
    }
}

// TODO: Advanced text features to be implemented:
//
// 1. Complex text shaping with rustybuzz/harfbuzz
// 2. Multi-line text layout with line breaking
// 3. Rich text formatting (spans with different styles)
// 4. Text selection and editing
// 5. Right-to-left (RTL) text support
// 6. Subpixel rendering for ultra-crisp text
// 7. SDF (signed distance field) fonts for scalable text
// 8. Text animation effects (fade, slide, scale)
// 9. Emoji and color font support
// 10. Text caching and dirty tracking for performance