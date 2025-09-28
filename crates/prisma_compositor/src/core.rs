/// Core WGPU abstractions and device management
use std::sync::Arc;
use wgpu::{Device as WgpuDevice, Queue, Instance, Adapter, Surface as WgpuSurface};
use winit::window::Window as WinitWindow;
use anyhow::Result;

/// WGPU device wrapper with optimized settings for UI rendering
pub struct Device {
    pub device: Arc<WgpuDevice>,
    pub queue: Arc<Queue>,
    pub adapter: Adapter,
    pub instance: Instance,
}

impl Device {
    /// Create a new device with optimal settings for UI rendering
    pub async fn new() -> Result<Self> {
        // Create WGPU instance with all backends for maximum compatibility
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        // Request adapter with high performance preference
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to create WGPU adapter"))?;

        // Request device with features optimized for 2D UI rendering
        let required_features = wgpu::Features::empty();
        let required_limits = wgpu::Limits {
            max_texture_dimension_2d: 8192,
            max_bind_groups: 8,
            max_uniform_buffer_binding_size: 65536,
            max_storage_buffer_binding_size: 134217728,
            max_vertex_buffers: 16,
            max_vertex_attributes: 16,
            max_vertex_buffer_array_stride: 2048,
            max_push_constant_size: 128,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("PrismaUI Compositor Device"),
                    required_features: required_features,
                    required_limits: required_limits,
                },
                None,
            )
            .await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter,
            instance,
        })
    }

    /// Get device limits
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }

    /// Get device features
    pub fn features(&self) -> wgpu::Features {
        self.device.features()
    }
}

/// Surface wrapper for window rendering
pub struct Surface {
    pub surface: WgpuSurface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub window_size: winit::dpi::PhysicalSize<u32>,
}

impl Surface {
    /// Create a new surface for the given window
    pub fn new(
        window: Arc<WinitWindow>,
        device: &Device,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<Self> {
        // Create surface
        let surface = device.instance.create_surface(window)?;

        // Get surface capabilities
        let surface_caps = surface.get_capabilities(&device.adapter);

        // Choose format - prefer sRGB for UI
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Configure surface
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device.device, &config);

        Ok(Self {
            surface,
            config,
            window_size: size,
        })
    }

    /// Resize the surface
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, device: &Device) {
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&device.device, &self.config);
        }
    }

    /// Get current surface texture for rendering
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture> {
        self.surface
            .get_current_texture()
            .map_err(|e| anyhow::anyhow!("Failed to get surface texture: {}", e))
    }
}

/// Render context with shared resources
pub struct Context {
    pub device: Arc<Device>,
    pub surface: Option<Surface>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Context {
    /// Create a new render context
    pub async fn new() -> Result<Self> {
        let device = Arc::new(Device::new().await?);

        Ok(Self {
            device,
            surface: None,
            size: winit::dpi::PhysicalSize::new(1920, 1080),
        })
    }

    /// Create surface for window
    pub fn create_surface(&mut self, window: Arc<WinitWindow>) -> Result<()> {
        let size = window.inner_size();
        self.size = size;
        self.surface = Some(Surface::new(window, &self.device, size)?);
        Ok(())
    }

    /// Resize context
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        if let Some(surface) = &mut self.surface {
            surface.resize(new_size, &self.device);
        }
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
    }

    /// Create orthographic projection matrix for 2D UI (Y down, X right)
    pub fn orthographic_projection(&self) -> [[f32; 4]; 4] {
        let width = self.size.width as f32;
        let height = self.size.height as f32;

        // Standard orthographic projection for 2D UI
        // Maps (0,0) to (-1,1) and (width,height) to (1,-1)
        [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [-1.0, 1.0, 0.5, 1.0],
        ]
    }

    /// Convert screen coordinates to normalized device coordinates
    pub fn screen_to_ndc(&self, x: f32, y: f32) -> (f32, f32) {
        let width = self.size.width as f32;
        let height = self.size.height as f32;

        let ndc_x = (2.0 * x) / width - 1.0;
        let ndc_y = 1.0 - (2.0 * y) / height;

        (ndc_x, ndc_y)
    }

    /// Convert normalized device coordinates to screen coordinates
    pub fn ndc_to_screen(&self, ndc_x: f32, ndc_y: f32) -> (f32, f32) {
        let width = self.size.width as f32;
        let height = self.size.height as f32;

        let x = (ndc_x + 1.0) * width / 2.0;
        let y = (1.0 - ndc_y) * height / 2.0;

        (x, y)
    }
}

/// GPU buffer wrapper with type safety
pub struct Buffer<T> {
    pub buffer: wgpu::Buffer,
    pub capacity: usize,
    pub len: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod> Buffer<T> {
    /// Create a new buffer
    pub fn new(
        device: &WgpuDevice,
        usage: wgpu::BufferUsages,
        capacity: usize,
        label: Option<&str>,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: (capacity * std::mem::size_of::<T>()) as u64,
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            capacity,
            len: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Write data to buffer
    pub fn write(&mut self, queue: &Queue, data: &[T]) {
        if data.len() > self.capacity {
            panic!("Buffer overflow: trying to write {} items to buffer with capacity {}",
                   data.len(), self.capacity);
        }

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        self.len = data.len();
    }

    /// Get buffer slice
    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..)
    }

    /// Get current length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}