/// Core compositor architecture with advanced threading and memory management
use std::sync::{Arc, RwLock, Mutex};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use wgpu::*;
use wgpu::Gles3MinorVersion;
use winit::{
    event_loop::{EventLoop, ControlFlow},
    window::Window as WinitWindow,
    event::{Event, WindowEvent, DeviceEvent},
};

use crate::{
    renderer::{WgpuRenderer, RenderCommand, RenderFrame},
    simple_renderer::SimpleRenderer,
    window::{WindowManager, WindowId},
    threading::{ThreadPool, RenderThread, ComputeThread},
    memory::{MemoryPool, BufferPool, TexturePool},
    assets::AssetManager,
};

/// High-performance compositor configuration
#[derive(Debug, Clone)]
pub struct CompositorConfig {
    /// Maximum number of render threads
    pub max_render_threads: usize,
    /// Maximum number of compute threads
    pub max_compute_threads: usize,
    /// Enable multi-threaded command buffer recording
    pub parallel_command_recording: bool,
    /// Buffer pool size in MB
    pub buffer_pool_size: usize,
    /// Texture pool size in MB
    pub texture_pool_size: usize,
    /// Maximum frames in flight
    pub max_frames_in_flight: usize,
    /// Enable GPU-based culling
    pub gpu_culling: bool,
    /// Enable temporal reprojection for performance
    pub temporal_reprojection: bool,
    /// Vsync mode
    pub vsync: bool,
    /// Multi-sampling level
    pub msaa_samples: u32,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            max_render_threads: num_cpus::get().min(8),
            max_compute_threads: (num_cpus::get() / 2).max(1),
            parallel_command_recording: true,
            buffer_pool_size: 256, // 256MB
            texture_pool_size: 512, // 512MB
            max_frames_in_flight: 3,
            gpu_culling: true,
            temporal_reprojection: true,
            vsync: true,
            msaa_samples: 4,
        }
    }
}

/// Frame timing statistics
#[derive(Debug, Clone)]
pub struct FrameStats {
    pub frame_time: f64,
    pub render_time: f64,
    pub cpu_time: f64,
    pub gpu_time: f64,
    pub memory_usage: usize,
    pub draw_calls: u32,
    pub triangles: u64,
}

/// Main compositor managing high-performance UI rendering
pub struct Compositor {
    /// WGPU instance and core rendering
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Mutex<wgpu::Surface<'static>>>,
    pub surface_config: Arc<RwLock<SurfaceConfiguration>>,

    /// Multi-threaded rendering system
    pub renderer: Arc<WgpuRenderer>,
    pub simple_renderer: Arc<SimpleRenderer>,
    pub thread_pool: Arc<ThreadPool>,
    pub render_threads: Vec<RenderThread>,
    pub compute_threads: Vec<ComputeThread>,

    /// Memory management
    pub memory_pool: Arc<MemoryPool>,
    pub buffer_pool: Arc<BufferPool>,
    pub texture_pool: Arc<TexturePool>,

    /// Window and UI management
    pub window_manager: Arc<RwLock<WindowManager>>,
    pub asset_manager: Arc<AssetManager>,

    /// Performance tracking
    pub frame_counter: AtomicU64,
    pub frame_stats: Arc<RwLock<FrameStats>>,
    pub running: AtomicBool,

    /// Command queues for multi-threading
    pub render_command_queue: Arc<Mutex<VecDeque<RenderCommand>>>,
    pub compute_command_queue: Arc<Mutex<VecDeque<Box<dyn Fn() + Send>>>>,

    /// Configuration
    pub config: CompositorConfig,
}

impl Compositor {
    /// Create a new high-performance compositor
    pub async fn new(
        window: Arc<WinitWindow>,
        config: CompositorConfig,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        // Initialize WGPU with optimal settings
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            dx12_shader_compiler: Dx12Compiler::default(),
            flags: InstanceFlags::default(),
            gles_minor_version: Gles3MinorVersion::Automatic,
        });

        let surface = unsafe {
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(
                instance.create_surface(window.as_ref())?
            )
        };

        // Request adapter with high-performance preferences
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable adapter")?;

        // Get optimal device limits
        let mut limits = Limits::default();
        limits.max_texture_dimension_2d = 8192;
        limits.max_buffer_size = 1024 * 1024 * 256; // 256MB max buffer
        limits.max_storage_buffer_binding_size = 1024 * 1024 * 128; // 128MB storage buffers

        // Get adapter features and filter out unsupported ones
        let adapter_features = adapter.features();
        let mut requested_features = Features::empty();

        // Add basic performance features if supported
        if adapter_features.contains(Features::MULTI_DRAW_INDIRECT) {
            requested_features |= Features::MULTI_DRAW_INDIRECT;
        }
        if adapter_features.contains(Features::INDIRECT_FIRST_INSTANCE) {
            requested_features |= Features::INDIRECT_FIRST_INSTANCE;
        }
        if adapter_features.contains(Features::TIMESTAMP_QUERY) {
            requested_features |= Features::TIMESTAMP_QUERY;
        }
        if adapter_features.contains(Features::PIPELINE_STATISTICS_QUERY) {
            requested_features |= Features::PIPELINE_STATISTICS_QUERY;
        }

        // Add texture compression features if supported
        if adapter_features.contains(Features::TEXTURE_COMPRESSION_BC) {
            requested_features |= Features::TEXTURE_COMPRESSION_BC;
        }
        if adapter_features.contains(Features::TEXTURE_COMPRESSION_ETC2) {
            requested_features |= Features::TEXTURE_COMPRESSION_ETC2;
        }
        if adapter_features.contains(Features::TEXTURE_COMPRESSION_ASTC) {
            requested_features |= Features::TEXTURE_COMPRESSION_ASTC;
        }

        println!("🎮 GPU Features:");
        println!("   - Multi-draw Indirect: {}", adapter_features.contains(Features::MULTI_DRAW_INDIRECT));
        println!("   - Timestamp Queries: {}", adapter_features.contains(Features::TIMESTAMP_QUERY));
        println!("   - BC Compression: {}", adapter_features.contains(Features::TEXTURE_COMPRESSION_BC));
        println!("   - ETC2 Compression: {}", adapter_features.contains(Features::TEXTURE_COMPRESSION_ETC2));
        println!("   - ASTC Compression: {}", adapter_features.contains(Features::TEXTURE_COMPRESSION_ASTC));

        // Request device with supported features
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("PrismaUI High-Performance Device"),
                    required_features: requested_features,
                    required_limits: limits,
                },
                None,
            )
            .await?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface for optimal performance
        let window_size = window.inner_size();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: window_size.width,
            height: window_size.height,
            present_mode: if config.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);
        let surface = Arc::new(Mutex::new(surface));
        let surface_config = Arc::new(RwLock::new(surface_config));

        // Initialize memory pools
        let memory_pool = Arc::new(MemoryPool::new(
            device.clone(),
            config.buffer_pool_size * 1024 * 1024,
        ));
        let buffer_pool = Arc::new(BufferPool::new(device.clone(), memory_pool.clone()));
        let texture_pool = Arc::new(TexturePool::new(
            device.clone(),
            config.texture_pool_size * 1024 * 1024,
        ));

        // Initialize threading system
        let thread_pool = Arc::new(ThreadPool::new(
            config.max_render_threads + config.max_compute_threads,
        ));

        let mut render_threads = Vec::with_capacity(config.max_render_threads);
        for i in 0..config.max_render_threads {
            render_threads.push(RenderThread::new(
                format!("RenderThread-{}", i),
                device.clone(),
                queue.clone(),
            )?);
        }

        let mut compute_threads = Vec::with_capacity(config.max_compute_threads);
        for i in 0..config.max_compute_threads {
            compute_threads.push(ComputeThread::new(
                format!("ComputeThread-{}", i),
                device.clone(),
                queue.clone(),
            )?);
        }

        // Initialize core systems
        let renderer = Arc::new(WgpuRenderer::new(
            device.clone(),
            queue.clone(),
            surface_config.clone(),
            &config,
        ).await?);

        let simple_renderer = Arc::new(SimpleRenderer::new(
            device.clone(),
            queue.clone(),
            surface_config.read().unwrap().format,
        ));

        let window_manager = Arc::new(RwLock::new(WindowManager::new()));
        let asset_manager = Arc::new(AssetManager::new(
            device.clone(),
            queue.clone(),
            texture_pool.clone(),
        ).await?);

        // Initialize performance tracking
        let frame_stats = Arc::new(RwLock::new(FrameStats {
            frame_time: 0.0,
            render_time: 0.0,
            cpu_time: 0.0,
            gpu_time: 0.0,
            memory_usage: 0,
            draw_calls: 0,
            triangles: 0,
        }));

        Ok(Arc::new(Self {
            device,
            queue,
            surface,
            surface_config,
            renderer,
            simple_renderer,
            thread_pool,
            render_threads,
            compute_threads,
            memory_pool,
            buffer_pool,
            texture_pool,
            window_manager,
            asset_manager,
            frame_counter: AtomicU64::new(0),
            frame_stats,
            running: AtomicBool::new(true),
            render_command_queue: Arc::new(Mutex::new(VecDeque::new())),
            compute_command_queue: Arc::new(Mutex::new(VecDeque::new())),
            config,
        }))
    }

    /// Start the compositor's main loop
    pub async fn run(
        self: Arc<Self>,
        event_loop: EventLoop<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _last_frame_time = std::time::Instant::now();

        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => {
                    self.handle_window_event(event);
                }
                Event::DeviceEvent { event, .. } => {
                    self.handle_device_event(event);
                }
                Event::AboutToWait => {
                    // Trigger redraw
                    self.request_redraw();

                    let frame_start = std::time::Instant::now();

                    // Multi-threaded frame rendering
                    if let Err(e) = pollster::block_on(self.render_frame()) {
                        eprintln!("Render error: {:?}", e);
                    }

                    // Update performance stats
                    let frame_time = frame_start.elapsed().as_secs_f64() * 1000.0;
                    if let Ok(mut stats) = self.frame_stats.write() {
                        stats.frame_time = frame_time;
                    }

                    self.frame_counter.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }

            if !self.running.load(Ordering::Relaxed) {
                elwt.exit();
            }
        })?;

        Ok(())
    }

    /// Render a frame with multi-threading optimizations
    pub async fn render_frame(&self) -> Result<(), Box<dyn std::error::Error>> {
        let frame_start = std::time::Instant::now();

        // Get current surface texture
        let surface_texture = {
            let surface = self.surface.lock().unwrap();
            surface.get_current_texture()?
        };

        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Get screen size
        let surface_config = self.surface_config.read().unwrap();
        let screen_width = surface_config.width as f32;
        let screen_height = surface_config.height as f32;
        drop(surface_config);

        // Get UI layers from the window manager's UI system
        let window_manager = self.window_manager.read().unwrap();
        let ui_system = window_manager.get_ui_system();
        let layers = ui_system.get_layers_for_rendering();

        println!("🎨 Testing coordinate conversion...");

        // Let's figure out what screen coordinates should produce our working NDC values
        // Working NDC: (-0.6, -0.3) to (0.4, 0.5)
        // Reverse calculate what screen coords these should be:

        // For x: ndc = (screen / screen_width) * 2.0 - 1.0
        // So: screen = (ndc + 1.0) * screen_width / 2.0
        let target_x1_screen = (-0.6 + 1.0) * screen_width / 2.0;  // Should be left edge
        let target_x2_screen = (0.4 + 1.0) * screen_width / 2.0;   // Should be right edge

        // For y: ndc = -((screen / screen_height) * 2.0 - 1.0)
        // So: screen = screen_height * (1.0 + ndc) / 2.0
        let target_y1_screen = screen_height * (1.0 + (-0.3)) / 2.0;  // Should be top edge
        let target_y2_screen = screen_height * (1.0 + 0.5) / 2.0;     // Should be bottom edge

        println!("   Working NDC (-0.6,-0.3) to (0.4,0.5) should correspond to screen:");
        println!("   X: {:.0} to {:.0} (width: {:.0})", target_x1_screen, target_x2_screen, target_x2_screen - target_x1_screen);
        println!("   Y: {:.0} to {:.0} (height: {:.0})", target_y1_screen, target_y2_screen, target_y2_screen - target_y1_screen);

        // Now test converting one of our UI elements
        let test_elem_x = 100.0;
        let test_elem_y = 100.0;
        let test_elem_w = 800.0;
        let test_elem_h = 600.0;

        let converted_x1 = (test_elem_x / screen_width) * 2.0 - 1.0;
        let converted_y1 = -((test_elem_y / screen_height) * 2.0 - 1.0);
        let converted_x2 = ((test_elem_x + test_elem_w) / screen_width) * 2.0 - 1.0;
        let converted_y2 = -(((test_elem_y + test_elem_h) / screen_height) * 2.0 - 1.0);

        println!("   UI Element: ({}, {}) {}x{} converts to NDC: ({:.3},{:.3}) to ({:.3},{:.3})",
            test_elem_x, test_elem_y, test_elem_w, test_elem_h, converted_x1, converted_y1, converted_x2, converted_y2);

        println!("🎨 Rendering {} layers with FIXED coordinate conversion", layers.len());
        println!("   Render order (by z-index):");
        for (idx, layer) in layers.iter().enumerate() {
            println!("     {} - '{}' (z: {})", idx, layer.name, layer.z_index);
        }

        // Create ONE render pass for ALL UI elements
        {
            let mut ui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        // Clear background first
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set the render pipeline once
            ui_pass.set_pipeline(&self.simple_renderer.render_pipeline);

            // Render all layers in the same render pass
            for (layer_idx, layer) in layers.iter().enumerate() {
                if !layer.visible {
                    println!("   Layer {} '{}' is not visible, skipping", layer_idx, layer.name);
                    continue;
                }

                println!("   Rendering Layer {} '{}' has {} elements (z-index: {})",
                    layer_idx, layer.name, layer.elements.len(), layer.z_index);

                for (elem_idx, element) in layer.elements.iter().enumerate() {
                    println!("     Element {} type: {:?}, rect: {:?}, color: {:?}",
                        elem_idx, element.element_type, element.rect, element.color);

                    // Render all UI elements as rectangles for now
                    match element.element_type {
                        crate::ui::UIElementType::Rect |
                        crate::ui::UIElementType::Button |
                        crate::ui::UIElementType::Panel |
                        crate::ui::UIElementType::Window |
                        crate::ui::UIElementType::Text => {  // Render text as colored rectangles for now
                            println!("       Rendering rect at ({}, {}) size {}x{}",
                                element.rect.x, element.rect.y, element.rect.width, element.rect.height);

                            // Skip tiny or zero-sized elements
                            if element.rect.width > 0.1 && element.rect.height > 0.1 {
                                self.simple_renderer.render_rect_in_pass(
                                    &mut ui_pass,
                                    element.rect.x,
                                    element.rect.y,
                                    element.rect.width,
                                    element.rect.height,
                                    element.color,
                                    screen_width,
                                    screen_height,
                                );
                            } else {
                                println!("       Skipping tiny element: {}x{}", element.rect.width, element.rect.height);
                            }
                        }
                        crate::ui::UIElementType::Image => {
                            println!("       Skipping image element (not implemented yet)");
                        }
                    }
                }
            }
        } // Single render pass ends here

        println!("✅ UI rendering completed with fixed coordinate conversion");

        // Submit the command buffer
        let command_buffer = encoder.finish();
        println!("📤 Submitting command buffer to GPU...");
        self.queue.submit(std::iter::once(command_buffer));

        // Present the frame
        surface_texture.present();

        // Update GPU stats
        let render_time = frame_start.elapsed().as_secs_f64() * 1000.0;
        if let Ok(mut stats) = self.frame_stats.write() {
            stats.render_time = render_time;
            stats.memory_usage = self.get_memory_usage();
        }

        Ok(())
    }

    /// Execute rendering commands in parallel across multiple threads
    async fn execute_parallel_rendering(
        &self,
        render_frame: &RenderFrame,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get render commands
        let commands = {
            let mut queue = self.render_command_queue.lock().unwrap();
            let mut commands = Vec::new();
            while let Some(cmd) = queue.pop_front() {
                commands.push(cmd);
            }
            commands
        };

        if commands.is_empty() {
            return Ok(());
        }

        // Split commands across render threads
        let chunk_size = (commands.len() / self.render_threads.len().max(1)).max(1);
        let command_chunks: Vec<_> = commands.chunks(chunk_size).collect();

        // Execute chunks in parallel
        let mut handles = Vec::new();
        for (i, chunk) in command_chunks.into_iter().enumerate() {
            let thread_idx = i % self.render_threads.len();
            let render_thread = &self.render_threads[thread_idx];
            let chunk = chunk.to_vec();
            let render_frame = render_frame.clone();

            let handle = render_thread.execute_commands(chunk, render_frame);
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.await?;
        }

        Ok(())
    }

    /// Handle window events
    fn handle_window_event(&self, event: WindowEvent) {
        match event {
            WindowEvent::Resized(new_size) => {
                self.resize_surface(new_size.width, new_size.height);
            }
            WindowEvent::CloseRequested => {
                self.running.store(false, Ordering::Relaxed);
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Ok(mut window_manager) = self.window_manager.write() {
                    window_manager.handle_cursor_move(position.x as f32, position.y as f32);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Ok(mut window_manager) = self.window_manager.write() {
                    window_manager.handle_mouse_input(button, state);
                }
            }
            _ => {}
        }
    }

    /// Handle device events for high-precision input
    fn handle_device_event(&self, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                // Handle high-precision mouse movement for smooth dragging
                if let Ok(mut window_manager) = self.window_manager.write() {
                    window_manager.handle_mouse_delta(delta.0 as f32, delta.1 as f32);
                }
            }
            _ => {}
        }
    }

    /// Resize the surface configuration
    fn resize_surface(&self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        if let Ok(mut config) = self.surface_config.write() {
            config.width = width;
            config.height = height;

            let surface = self.surface.lock().unwrap();
            surface.configure(&self.device, &config);
        }
    }

    /// Request a redraw
    fn request_redraw(&self) {
        // In a real implementation, this would trigger the window to redraw
    }

    /// Get current memory usage
    fn get_memory_usage(&self) -> usize {
        self.buffer_pool.get_memory_usage() + self.texture_pool.get_memory_usage()
    }

    /// Get current frame statistics
    pub fn get_frame_stats(&self) -> FrameStats {
        self.frame_stats.read().unwrap().clone()
    }

    /// Shutdown the compositor
    pub async fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);

        // Wait for all threads to finish
        for thread in &self.render_threads {
            thread.shutdown().await;
        }

        for thread in &self.compute_threads {
            thread.shutdown().await;
        }

        // Clean up resources
        self.buffer_pool.cleanup().await;
        self.texture_pool.cleanup().await;
        self.memory_pool.cleanup().await;
    }
}

unsafe impl Send for Compositor {}
unsafe impl Sync for Compositor {}