/// Asset management system for textures, images, and other resources
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use anyhow::Result;
use image::{DynamicImage, ImageBuffer};
use crate::{
    core::Device,
    geometry::Size,
};

/// GPU texture wrapper
#[derive(Debug, Clone)]
pub struct Texture {
    pub texture: Arc<wgpu::Texture>,
    pub size: Size,
    pub format: wgpu::TextureFormat,
}

impl Texture {
    /// Create texture from image data
    pub fn from_image(device: &Device, image: &DynamicImage, label: Option<&str>) -> Result<Self> {
        let rgba_image = image.to_rgba8();
        let dimensions = rgba_image.dimensions();

        let size = Size::new(dimensions.0 as f32, dimensions.1 as f32);

        let texture = device.device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        Ok(Self {
            texture: Arc::new(texture),
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Create texture from raw RGBA data
    pub fn from_rgba_bytes(
        device: &Device,
        data: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>
    ) -> Result<Self> {
        let size = Size::new(width as f32, height as f32);

        let texture = device.device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(Self {
            texture: Arc::new(texture),
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        })
    }

    /// Create a solid color texture
    pub fn solid_color(device: &Device, color: crate::geometry::Color, size: Size) -> Result<Self> {
        let width = size.width as u32;
        let height = size.height as u32;

        let color_bytes = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            data.extend_from_slice(&color_bytes);
        }

        Self::from_rgba_bytes(device, &data, width, height, Some("Solid Color Texture"))
    }

    /// Create texture view
    pub fn create_view(&self) -> wgpu::TextureView {
        self.texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Create sampler for this texture
    pub fn create_sampler(&self, device: &Device, filter: wgpu::FilterMode) -> wgpu::Sampler {
        device.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }
}

/// Texture atlas for efficient GPU memory usage
#[derive(Debug)]
pub struct TextureAtlas {
    /// Main atlas texture
    texture: Texture,
    /// Allocated regions in the atlas
    regions: HashMap<String, AtlasRegion>,
    /// Current packing position
    pack_x: u32,
    pack_y: u32,
    /// Current row height
    row_height: u32,
    /// Atlas dimensions
    width: u32,
    height: u32,
}

/// Region within a texture atlas
#[derive(Debug, Clone)]
pub struct AtlasRegion {
    /// Position in atlas (pixel coordinates)
    pub x: u32,
    pub y: u32,
    /// Size in pixels
    pub width: u32,
    pub height: u32,
    /// UV coordinates (0.0 to 1.0)
    pub uv_rect: crate::geometry::Rect,
}

impl TextureAtlas {
    /// Create a new texture atlas
    pub fn new(device: &Device, width: u32, height: u32) -> Result<Self> {
        // Create empty white texture for the atlas
        let data = vec![255u8; (width * height * 4) as usize];
        let texture = Texture::from_rgba_bytes(device, &data, width, height, Some("Texture Atlas"))?;

        Ok(Self {
            texture,
            regions: HashMap::new(),
            pack_x: 0,
            pack_y: 0,
            row_height: 0,
            width,
            height,
        })
    }

    /// Add a texture to the atlas
    pub fn add_texture(&mut self, device: &Device, name: String, image: &DynamicImage) -> Result<AtlasRegion> {
        let rgba_image = image.to_rgba8();
        let (img_width, img_height) = rgba_image.dimensions();

        // Check if we can fit the texture in current row
        if self.pack_x + img_width > self.width {
            // Move to next row
            self.pack_x = 0;
            self.pack_y += self.row_height;
            self.row_height = 0;
        }

        // Check if we have space at all
        if self.pack_y + img_height > self.height {
            return Err(anyhow::anyhow!("Texture atlas is full"));
        }

        // Calculate UV coordinates
        let uv_rect = crate::geometry::Rect::new(
            self.pack_x as f32 / self.width as f32,
            self.pack_y as f32 / self.height as f32,
            img_width as f32 / self.width as f32,
            img_height as f32 / self.height as f32,
        );

        let region = AtlasRegion {
            x: self.pack_x,
            y: self.pack_y,
            width: img_width,
            height: img_height,
            uv_rect,
        };

        // Write texture data to atlas
        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.pack_x,
                    y: self.pack_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * img_width),
                rows_per_image: Some(img_height),
            },
            wgpu::Extent3d {
                width: img_width,
                height: img_height,
                depth_or_array_layers: 1,
            },
        );

        // Update packing position
        self.pack_x += img_width;
        self.row_height = self.row_height.max(img_height);

        // Store region
        self.regions.insert(name, region.clone());

        Ok(region)
    }

    /// Get a region by name
    pub fn get_region(&self, name: &str) -> Option<&AtlasRegion> {
        self.regions.get(name)
    }

    /// Get the atlas texture
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Get all regions
    pub fn regions(&self) -> &HashMap<String, AtlasRegion> {
        &self.regions
    }

    /// Clear the atlas
    pub fn clear(&mut self, device: &Device) -> Result<()> {
        // Reset to white texture
        let data = vec![255u8; (self.width * self.height * 4) as usize];
        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.regions.clear();
        self.pack_x = 0;
        self.pack_y = 0;
        self.row_height = 0;

        Ok(())
    }
}

/// Asset manager for loading and caching resources
pub struct AssetManager {
    /// Individual textures
    textures: HashMap<String, Texture>,
    /// Texture atlases
    atlases: HashMap<String, TextureAtlas>,
    /// Default white pixel texture
    white_texture: Texture,
    /// Cache of loaded images
    image_cache: HashMap<String, DynamicImage>,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(device: &Device) -> Result<Self> {
        // Create default white texture
        let white_texture = Texture::solid_color(
            device,
            crate::geometry::Color::WHITE,
            Size::new(1.0, 1.0)
        )?;

        Ok(Self {
            textures: HashMap::new(),
            atlases: HashMap::new(),
            white_texture,
            image_cache: HashMap::new(),
        })
    }

    /// Load texture from file path
    pub fn load_texture_from_path<P: AsRef<Path>>(&mut self, device: &Device, name: String, path: P) -> Result<()> {
        let image = image::open(path)?;
        self.image_cache.insert(name.clone(), image.clone());
        let texture = Texture::from_image(device, &image, Some(&name))?;
        self.textures.insert(name, texture);
        Ok(())
    }

    /// Load texture from bytes
    pub fn load_texture_from_bytes(&mut self, device: &Device, name: String, bytes: &[u8]) -> Result<()> {
        let image = image::load_from_memory(bytes)?;
        self.image_cache.insert(name.clone(), image.clone());
        let texture = Texture::from_image(device, &image, Some(&name))?;
        self.textures.insert(name, texture);
        Ok(())
    }

    /// Create atlas
    pub fn create_atlas(&mut self, device: &Device, name: String, width: u32, height: u32) -> Result<()> {
        let atlas = TextureAtlas::new(device, width, height)?;
        self.atlases.insert(name, atlas);
        Ok(())
    }

    /// Add texture to atlas
    pub fn add_to_atlas(&mut self, device: &Device, atlas_name: &str, texture_name: String, image_name: &str) -> Result<AtlasRegion> {
        let image = self.image_cache.get(image_name)
            .ok_or_else(|| anyhow::anyhow!("Image '{}' not found in cache", image_name))?
            .clone();

        let atlas = self.atlases.get_mut(atlas_name)
            .ok_or_else(|| anyhow::anyhow!("Atlas '{}' not found", atlas_name))?;

        atlas.add_texture(device, texture_name, &image)
    }

    /// Get texture by name
    pub fn get_texture(&self, name: &str) -> Option<&Texture> {
        self.textures.get(name)
    }

    /// Get atlas by name
    pub fn get_atlas(&self, name: &str) -> Option<&TextureAtlas> {
        self.atlases.get(name)
    }

    /// Get default white texture
    pub fn white_texture(&self) -> &Texture {
        &self.white_texture
    }

    /// Preload common UI assets
    pub fn preload_ui_assets(&mut self, device: &Device) -> Result<()> {
        // Create UI atlas for icons and small textures
        self.create_atlas(device, "ui".to_string(), 512, 512)?;

        // Load placeholder icons (you would load actual icon files)
        self.create_placeholder_icons(device)?;

        Ok(())
    }

    fn create_placeholder_icons(&mut self, device: &Device) -> Result<()> {
        // Create simple placeholder icons
        let icon_size = 32u32;

        // Folder icon (blue square)
        let folder_data = self.create_solid_icon(icon_size, [100, 150, 255, 255]);
        let folder_image = DynamicImage::ImageRgba8(
            ImageBuffer::from_raw(icon_size, icon_size, folder_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create folder icon"))?
        );
        self.image_cache.insert("folder_icon".to_string(), folder_image);

        // File icon (gray square)
        let file_data = self.create_solid_icon(icon_size, [150, 150, 150, 255]);
        let file_image = DynamicImage::ImageRgba8(
            ImageBuffer::from_raw(icon_size, icon_size, file_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create file icon"))?
        );
        self.image_cache.insert("file_icon".to_string(), file_image);

        // Add to atlas
        self.add_to_atlas(device, "ui", "folder_icon".to_string(), "folder_icon")?;
        self.add_to_atlas(device, "ui", "file_icon".to_string(), "file_icon")?;

        Ok(())
    }

    fn create_solid_icon(&self, size: u32, color: [u8; 4]) -> Vec<u8> {
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..(size * size) {
            data.extend_from_slice(&color);
        }
        data
    }

    /// Clear all assets
    pub fn clear(&mut self) {
        self.textures.clear();
        self.atlases.clear();
        self.image_cache.clear();
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> AssetMemoryStats {
        let mut total_texture_memory = 0u64;
        let mut total_atlas_memory = 0u64;

        for texture in self.textures.values() {
            total_texture_memory += self.calculate_texture_memory(&texture);
        }

        for atlas in self.atlases.values() {
            total_atlas_memory += self.calculate_texture_memory(atlas.texture());
        }

        AssetMemoryStats {
            texture_count: self.textures.len(),
            atlas_count: self.atlases.len(),
            cached_images: self.image_cache.len(),
            total_texture_memory,
            total_atlas_memory,
        }
    }

    fn calculate_texture_memory(&self, texture: &Texture) -> u64 {
        let bytes_per_pixel = match texture.format {
            wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::R8Unorm => 1,
            _ => 4, // Default assumption
        };

        (texture.size.width as u64) * (texture.size.height as u64) * bytes_per_pixel
    }
}

/// Memory usage statistics for assets
#[derive(Debug, Clone)]
pub struct AssetMemoryStats {
    pub texture_count: usize,
    pub atlas_count: usize,
    pub cached_images: usize,
    pub total_texture_memory: u64,
    pub total_atlas_memory: u64,
}

impl AssetMemoryStats {
    pub fn total_memory(&self) -> u64 {
        self.total_texture_memory + self.total_atlas_memory
    }

    pub fn total_memory_mb(&self) -> f64 {
        self.total_memory() as f64 / (1024.0 * 1024.0)
    }
}

// TODO: Advanced asset features to be implemented:
//
// 1. Async asset loading with progress callbacks
// 2. Asset streaming for large textures
// 3. Texture compression (BC7, ASTC, etc.)
// 4. Mipmap generation for better filtering
// 5. Asset dependency management
// 6. Hot reloading for development
// 7. Memory budget management with automatic unloading
// 8. Asset versioning and caching
// 9. Multi-threaded asset processing
// 10. GPU texture compression and decompression