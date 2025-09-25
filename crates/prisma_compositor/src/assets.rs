/// High-performance asset management with GPU texture atlases
use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;
use wgpu::*;
use image::{DynamicImage, ImageFormat};

use crate::memory::TexturePool;

/// Asset types supported by the compositor
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetType {
    Image,
    Font,
    Audio,
    Video,
    Model3D,
}

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub id: u32,
    pub name: String,
    pub asset_type: AssetType,
    pub size: usize,
    pub last_used: std::time::Instant,
    pub usage_count: u64,
    pub gpu_resident: bool,
}

/// Texture atlas for efficient GPU memory usage
pub struct TextureAtlas {
    texture: Arc<Texture>,
    texture_view: Arc<TextureView>,
    width: u32,
    height: u32,
    format: TextureFormat,

    /// Atlas allocation tracking
    allocated_regions: Arc<RwLock<Vec<AtlasRegion>>>,
    free_regions: Arc<RwLock<Vec<AtlasRegion>>>,

    /// Performance tracking
    usage_stats: Arc<RwLock<AtlasStats>>,
}

/// Region within a texture atlas
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub asset_id: Option<u32>,
}

/// Atlas performance statistics
#[derive(Debug, Clone)]
pub struct AtlasStats {
    pub total_area: u32,
    pub used_area: u32,
    pub fragmentation: f32,
    pub allocation_count: u64,
    pub deallocation_count: u64,
}

impl TextureAtlas {
    /// Create a new texture atlas
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        label: Option<&str>,
    ) -> Self {
        let texture = Arc::new(device.create_texture(&TextureDescriptor {
            label,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        }));

        let texture_view = Arc::new(texture.create_view(&TextureViewDescriptor::default()));

        // Initialize with one large free region
        let mut free_regions = Vec::new();
        free_regions.push(AtlasRegion {
            x: 0,
            y: 0,
            width,
            height,
            asset_id: None,
        });

        let stats = AtlasStats {
            total_area: width * height,
            used_area: 0,
            fragmentation: 0.0,
            allocation_count: 0,
            deallocation_count: 0,
        };

        Self {
            texture,
            texture_view,
            width,
            height,
            format,
            allocated_regions: Arc::new(RwLock::new(Vec::new())),
            free_regions: Arc::new(RwLock::new(free_regions)),
            usage_stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Allocate a region in the atlas
    pub fn allocate(&self, width: u32, height: u32, asset_id: u32) -> Option<AtlasRegion> {
        let mut free_regions = self.free_regions.write().ok()?;
        let mut allocated_regions = self.allocated_regions.write().ok()?;

        // Find best-fit free region
        let best_fit_idx = free_regions
            .iter()
            .enumerate()
            .filter(|(_, region)| region.width >= width && region.height >= height)
            .min_by_key(|(_, region)| region.width * region.height)
            .map(|(idx, _)| idx)?;

        let free_region = free_regions[best_fit_idx];

        // Remove the free region
        free_regions.swap_remove(best_fit_idx);

        // Create allocated region
        let allocated_region = AtlasRegion {
            x: free_region.x,
            y: free_region.y,
            width,
            height,
            asset_id: Some(asset_id),
        };

        allocated_regions.push(allocated_region);

        // Split remaining space
        if free_region.width > width {
            // Right remainder
            free_regions.push(AtlasRegion {
                x: free_region.x + width,
                y: free_region.y,
                width: free_region.width - width,
                height,
                asset_id: None,
            });
        }

        if free_region.height > height {
            // Bottom remainder
            free_regions.push(AtlasRegion {
                x: free_region.x,
                y: free_region.y + height,
                width: free_region.width,
                height: free_region.height - height,
                asset_id: None,
            });
        }

        // Update statistics
        if let Ok(mut stats) = self.usage_stats.write() {
            stats.used_area += width * height;
            stats.allocation_count += 1;
            stats.fragmentation = self.calculate_fragmentation(&free_regions);
        }

        Some(allocated_region)
    }

    /// Deallocate a region
    pub fn deallocate(&self, region: AtlasRegion) {
        if let (Ok(mut allocated_regions), Ok(mut free_regions)) = (
            self.allocated_regions.write(),
            self.free_regions.write(),
        ) {
            // Remove from allocated regions
            allocated_regions.retain(|r| {
                !(r.x == region.x
                    && r.y == region.y
                    && r.width == region.width
                    && r.height == region.height)
            });

            // Add to free regions
            free_regions.push(AtlasRegion {
                x: region.x,
                y: region.y,
                width: region.width,
                height: region.height,
                asset_id: None,
            });

            // Merge adjacent free regions
            self.merge_free_regions(&mut free_regions);

            // Update statistics
            if let Ok(mut stats) = self.usage_stats.write() {
                stats.used_area -= region.width * region.height;
                stats.deallocation_count += 1;
                stats.fragmentation = self.calculate_fragmentation(&free_regions);
            }
        }
    }

    /// Calculate fragmentation ratio
    fn calculate_fragmentation(&self, free_regions: &[AtlasRegion]) -> f32 {
        if free_regions.is_empty() {
            return 0.0;
        }

        let total_free_area: u32 = free_regions.iter().map(|r| r.width * r.height).sum();
        let largest_free_area = free_regions
            .iter()
            .map(|r| r.width * r.height)
            .max()
            .unwrap_or(0);

        if total_free_area == 0 {
            0.0
        } else {
            1.0 - (largest_free_area as f32 / total_free_area as f32)
        }
    }

    /// Merge adjacent free regions to reduce fragmentation
    fn merge_free_regions(&self, free_regions: &mut Vec<AtlasRegion>) {
        let mut merged = true;
        while merged {
            merged = false;
            for i in 0..free_regions.len() {
                for j in (i + 1)..free_regions.len() {
                    if let Some(merged_region) = self.try_merge_regions(free_regions[i], free_regions[j]) {
                        free_regions[i] = merged_region;
                        free_regions.swap_remove(j);
                        merged = true;
                        break;
                    }
                }
                if merged {
                    break;
                }
            }
        }
    }

    /// Try to merge two regions if they're adjacent
    fn try_merge_regions(&self, a: AtlasRegion, b: AtlasRegion) -> Option<AtlasRegion> {
        // Check if regions are horizontally adjacent
        if a.y == b.y && a.height == b.height {
            if a.x + a.width == b.x {
                return Some(AtlasRegion {
                    x: a.x,
                    y: a.y,
                    width: a.width + b.width,
                    height: a.height,
                    asset_id: None,
                });
            } else if b.x + b.width == a.x {
                return Some(AtlasRegion {
                    x: b.x,
                    y: b.y,
                    width: b.width + a.width,
                    height: b.height,
                    asset_id: None,
                });
            }
        }

        // Check if regions are vertically adjacent
        if a.x == b.x && a.width == b.width {
            if a.y + a.height == b.y {
                return Some(AtlasRegion {
                    x: a.x,
                    y: a.y,
                    width: a.width,
                    height: a.height + b.height,
                    asset_id: None,
                });
            } else if b.y + b.height == a.y {
                return Some(AtlasRegion {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height + a.height,
                    asset_id: None,
                });
            }
        }

        None
    }

    /// Get texture reference
    pub fn get_texture(&self) -> Arc<Texture> {
        Arc::clone(&self.texture)
    }

    /// Get texture view reference
    pub fn get_texture_view(&self) -> Arc<TextureView> {
        Arc::clone(&self.texture_view)
    }

    /// Get usage statistics
    pub fn get_stats(&self) -> AtlasStats {
        self.usage_stats.read().unwrap().clone()
    }
}

/// Font atlas for text rendering
pub struct FontAtlas {
    texture_atlas: TextureAtlas,
    glyph_cache: Arc<RwLock<HashMap<(u32, char), AtlasRegion>>>, // (font_id, character) -> region
    font_data: Arc<RwLock<HashMap<u32, FontData>>>,
}

/// Font data and metrics
#[derive(Debug, Clone)]
pub struct FontData {
    pub id: u32,
    pub name: String,
    pub size: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

impl FontAtlas {
    /// Create a new font atlas
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let texture_atlas = TextureAtlas::new(
            device,
            width,
            height,
            TextureFormat::R8Unorm, // Single channel for SDF fonts
            Some("Font Atlas"),
        );

        Self {
            texture_atlas,
            glyph_cache: Arc::new(RwLock::new(HashMap::new())),
            font_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or load a glyph
    pub fn get_glyph(&self, font_id: u32, character: char) -> Option<AtlasRegion> {
        // Check cache first
        if let Ok(cache) = self.glyph_cache.read() {
            if let Some(&region) = cache.get(&(font_id, character)) {
                return Some(region);
            }
        }

        // Load glyph if not cached
        self.load_glyph(font_id, character)
    }

    /// Load a glyph into the atlas
    fn load_glyph(&self, font_id: u32, character: char) -> Option<AtlasRegion> {
        // This would involve font rasterization
        // For now, we'll allocate a placeholder region
        let glyph_size = 32; // Placeholder size

        if let Some(region) = self.texture_atlas.allocate(glyph_size, glyph_size, font_id) {
            // Cache the glyph
            if let Ok(mut cache) = self.glyph_cache.write() {
                cache.insert((font_id, character), region);
            }

            Some(region)
        } else {
            None
        }
    }
}

/// Asset cache for efficient resource management
#[derive(Default)]
pub struct AssetCache {
    images: Arc<RwLock<HashMap<u32, DynamicImage>>>,
    image_info: Arc<RwLock<HashMap<u32, AssetInfo>>>,
    next_asset_id: std::sync::atomic::AtomicU32,
}

impl AssetCache {
    /// Create new asset cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Load image from bytes
    pub fn load_image_from_bytes(&self, name: String, data: &[u8]) -> Result<u32, Box<dyn std::error::Error>> {
        let image = image::load_from_memory(data)?;
        let asset_id = self.next_asset_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let info = AssetInfo {
            id: asset_id,
            name,
            asset_type: AssetType::Image,
            size: data.len(),
            last_used: std::time::Instant::now(),
            usage_count: 0,
            gpu_resident: false,
        };

        {
            let mut images = self.images.write().unwrap();
            images.insert(asset_id, image);
        }

        {
            let mut image_info = self.image_info.write().unwrap();
            image_info.insert(asset_id, info);
        }

        Ok(asset_id)
    }

    /// Get image by ID
    pub fn get_image(&self, asset_id: u32) -> Option<DynamicImage> {
        let images = self.images.read().unwrap();
        images.get(&asset_id).cloned()
    }

    /// Update usage statistics
    pub fn mark_used(&self, asset_id: u32) {
        if let Ok(mut image_info) = self.image_info.write() {
            if let Some(info) = image_info.get_mut(&asset_id) {
                info.last_used = std::time::Instant::now();
                info.usage_count += 1;
            }
        }
    }
}

/// High-performance asset manager
pub struct AssetManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    texture_pool: Arc<TexturePool>,

    /// Asset caches
    asset_cache: AssetCache,

    /// GPU texture atlases
    image_atlas: Arc<Mutex<TextureAtlas>>,
    font_atlas: Arc<Mutex<FontAtlas>>,

    /// Asset mapping (CPU asset ID -> GPU texture atlas region)
    image_mapping: Arc<RwLock<HashMap<u32, AtlasRegion>>>,
    font_mapping: Arc<RwLock<HashMap<u32, FontData>>>,
}

impl AssetManager {
    /// Create new asset manager
    pub async fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        texture_pool: Arc<TexturePool>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let image_atlas = Arc::new(Mutex::new(TextureAtlas::new(
            &device,
            2048,
            2048,
            TextureFormat::Rgba8UnormSrgb,
            Some("Image Atlas"),
        )));

        let font_atlas = Arc::new(Mutex::new(FontAtlas::new(&device, 1024, 1024)));

        Ok(Self {
            device,
            queue,
            texture_pool,
            asset_cache: AssetCache::new(),
            image_atlas,
            font_atlas,
            image_mapping: Arc::new(RwLock::new(HashMap::new())),
            font_mapping: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load image asset
    pub fn load_image(&self, name: String, data: &[u8]) -> Result<u32, Box<dyn std::error::Error>> {
        // Load into CPU cache
        let asset_id = self.asset_cache.load_image_from_bytes(name, data)?;

        // Upload to GPU atlas
        self.upload_image_to_atlas(asset_id)?;

        Ok(asset_id)
    }

    /// Upload image to GPU atlas
    fn upload_image_to_atlas(&self, asset_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(image) = self.asset_cache.get_image(asset_id) {
            let rgba_image = image.to_rgba8();
            let (width, height) = rgba_image.dimensions();

            if let Ok(mut atlas) = self.image_atlas.lock() {
                if let Some(region) = atlas.allocate(width, height, asset_id) {
                    // Upload image data to the atlas
                    self.queue.write_texture(
                        ImageCopyTexture {
                            texture: &atlas.get_texture(),
                            mip_level: 0,
                            origin: Origin3d {
                                x: region.x,
                                y: region.y,
                                z: 0,
                            },
                            aspect: TextureAspect::All,
                        },
                        &rgba_image,
                        ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * width),
                            rows_per_image: Some(height),
                        },
                        Extent3d {
                            width: region.width,
                            height: region.height,
                            depth_or_array_layers: 1,
                        },
                    );

                    // Store mapping
                    if let Ok(mut mapping) = self.image_mapping.write() {
                        mapping.insert(asset_id, region);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get image atlas
    pub fn get_image_atlas(&self) -> Arc<Mutex<TextureAtlas>> {
        Arc::clone(&self.image_atlas)
    }

    /// Get font atlas
    pub fn get_font_atlas(&self) -> Arc<Mutex<FontAtlas>> {
        Arc::clone(&self.font_atlas)
    }

    /// Get image region in atlas
    pub fn get_image_region(&self, asset_id: u32) -> Option<AtlasRegion> {
        let mapping = self.image_mapping.read().unwrap();
        mapping.get(&asset_id).copied()
    }
}