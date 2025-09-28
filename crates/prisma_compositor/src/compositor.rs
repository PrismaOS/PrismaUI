/// Main compositor orchestrating the GPU-accelerated desktop environment
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::Window as WinitWindow,
};
use anyhow::Result;

use crate::{
    core::Context,
    renderer::Renderer,
    window::{WindowManager, WindowId},
    ui::{UITree, Container, Button, Text, Rectangle, LayoutDirection, UIElement, LayoutConstraints},
    events::{EventDispatcher, Event, InputEvent, MouseButton, ButtonState, Modifiers},
    text::TextRenderer,
    assets::AssetManager,
    geometry::{Rect, Size, Color},
};

/// Configuration for the compositor
#[derive(Debug, Clone)]
pub struct CompositorConfig {
    /// Window title
    pub title: String,
    /// Initial window size
    pub size: PhysicalSize<u32>,
    /// Target framerate
    pub target_fps: u32,
    /// Enable VSync
    pub vsync: bool,
    /// Enable performance monitoring
    pub debug_mode: bool,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            title: "PrismaUI Compositor".to_string(),
            size: PhysicalSize::new(1920, 1080),
            target_fps: 60,
            vsync: true,
            debug_mode: false,
        }
    }
}

/// Performance metrics for monitoring
#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub cpu_time_ms: f32,
    pub gpu_time_ms: f32,
    pub memory_usage_mb: f32,
    pub draw_calls: u32,
    pub triangles_rendered: u32,
}

/// Main compositor state
pub struct Compositor {
    // Core rendering
    context: Context,
    renderer: Renderer,

    // UI and window management
    window_manager: WindowManager,
    event_dispatcher: EventDispatcher,
    desktop_ui: UITree,

    // Asset management
    asset_manager: AssetManager,
    text_renderer: TextRenderer,

    // State
    config: CompositorConfig,
    running: bool,
    last_frame_time: Instant,
    frame_count: u64,
    performance_metrics: PerformanceMetrics,
    cursor_position: crate::geometry::Point, // Track cursor position

    // Demo content (TODO: remove when porting actual UI)
    demo_window_id: Option<WindowId>,
}

impl Compositor {
    /// Create a new compositor
    pub async fn new(config: CompositorConfig) -> Result<Self> {
        let context = Context::new().await?;

        // Create renderer with a placeholder surface format
        let renderer = Renderer::new(&context.device, wgpu::TextureFormat::Bgra8UnormSrgb)?;

        let window_manager = WindowManager::new(Rect::new(0.0, 0.0, 1920.0, 1080.0));
        let event_dispatcher = EventDispatcher::new();
        let desktop_ui = UITree::new();

        let asset_manager = AssetManager::new(&context.device)?;
        let text_renderer = TextRenderer::new();

        Ok(Self {
            context,
            renderer,
            window_manager,
            event_dispatcher,
            desktop_ui,
            asset_manager,
            text_renderer,
            config,
            running: false,
            last_frame_time: Instant::now(),
            frame_count: 0,
            performance_metrics: PerformanceMetrics::default(),
            cursor_position: crate::geometry::Point::new(0.0, 0.0),
            demo_window_id: None,
        })
    }

    /// Run the compositor event loop
    pub fn run(mut self) -> Result<()> {
        use winit::{
            event::{Event as WinitEvent, WindowEvent},
            event_loop::{ControlFlow, EventLoopWindowTarget},
            window::WindowBuilder,
        };

        let event_loop = EventLoop::new()?;
        let window = Arc::new(WindowBuilder::new()
            .with_title(&self.config.title)
            .with_inner_size(self.config.size)
            .with_visible(true)
            .build(&event_loop)?);

        // Initialize compositor with the window
        self.initialize(window.clone())?;

        println!("🖥️  Window created and compositor ready!");
        println!("🎮 Starting event loop...");

        let mut last_update = std::time::Instant::now();
        let target_frame_time = Duration::from_secs_f32(1.0 / self.config.target_fps as f32);

        // Control flow will be set in the event loop

        event_loop.run(move |event, target: &EventLoopWindowTarget<()>| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                WinitEvent::WindowEvent { window_id, event } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            println!("👋 Close requested - shutting down compositor");
                            target.exit();
                        }
                        WindowEvent::Resized(physical_size) => {
                            if physical_size.width > 0 && physical_size.height > 0 {
                                self.resize(physical_size);
                            }
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            let size = window.inner_size();
                            self.resize(size);
                        }
                        WindowEvent::MouseInput { button, state, .. } => {
                            let input_event = InputEvent::MouseButton {
                                button: self.convert_mouse_button(button),
                                state: self.convert_button_state(state),
                                position: self.cursor_position,
                                modifiers: crate::events::Modifiers::new(),
                            };
                            self.handle_input_event(input_event);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let new_position = crate::geometry::Point::new(position.x as f32, position.y as f32);
                            let delta = new_position - self.cursor_position;
                            self.cursor_position = new_position;

                            let input_event = InputEvent::MouseMove {
                                position: new_position,
                                delta,
                            };
                            self.handle_input_event(input_event);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let scroll_delta = match delta {
                                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                    crate::geometry::Point::new(x, y)
                                }
                                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                    crate::geometry::Point::new(pos.x as f32, pos.y as f32)
                                }
                            };
                            let input_event = InputEvent::MouseWheel {
                                delta: scroll_delta,
                                position: self.cursor_position,
                                modifiers: crate::events::Modifiers::new(),
                            };
                            self.handle_input_event(input_event);
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let Some(text) = &event.text {
                                let input_event = InputEvent::TextInput {
                                    text: text.to_string(),
                                };
                                self.handle_input_event(input_event);
                            } else {
                                // Handle key press/release
                                let key = match event.logical_key {
                                    winit::keyboard::Key::Named(named_key) => {
                                        match named_key {
                                            winit::keyboard::NamedKey::Escape => crate::events::Key::Escape,
                                            winit::keyboard::NamedKey::Tab => crate::events::Key::Tab,
                                            winit::keyboard::NamedKey::Enter => crate::events::Key::Enter,
                                            winit::keyboard::NamedKey::Space => crate::events::Key::Space,
                                            winit::keyboard::NamedKey::Backspace => crate::events::Key::Backspace,
                                            winit::keyboard::NamedKey::Delete => crate::events::Key::Delete,
                                            winit::keyboard::NamedKey::ArrowUp => crate::events::Key::ArrowUp,
                                            winit::keyboard::NamedKey::ArrowDown => crate::events::Key::ArrowDown,
                                            winit::keyboard::NamedKey::ArrowLeft => crate::events::Key::ArrowLeft,
                                            winit::keyboard::NamedKey::ArrowRight => crate::events::Key::ArrowRight,
                                            winit::keyboard::NamedKey::Home => crate::events::Key::Home,
                                            winit::keyboard::NamedKey::End => crate::events::Key::End,
                                            winit::keyboard::NamedKey::PageUp => crate::events::Key::PageUp,
                                            winit::keyboard::NamedKey::PageDown => crate::events::Key::PageDown,
                                            winit::keyboard::NamedKey::F1 => crate::events::Key::F1,
                                            winit::keyboard::NamedKey::F2 => crate::events::Key::F2,
                                            winit::keyboard::NamedKey::F3 => crate::events::Key::F3,
                                            winit::keyboard::NamedKey::F4 => crate::events::Key::F4,
                                            winit::keyboard::NamedKey::F5 => crate::events::Key::F5,
                                            winit::keyboard::NamedKey::F6 => crate::events::Key::F6,
                                            winit::keyboard::NamedKey::F7 => crate::events::Key::F7,
                                            winit::keyboard::NamedKey::F8 => crate::events::Key::F8,
                                            winit::keyboard::NamedKey::F9 => crate::events::Key::F9,
                                            winit::keyboard::NamedKey::F10 => crate::events::Key::F10,
                                            winit::keyboard::NamedKey::F11 => crate::events::Key::F11,
                                            winit::keyboard::NamedKey::F12 => crate::events::Key::F12,
                                            _ => crate::events::Key::Unknown,
                                        }
                                    }
                                    winit::keyboard::Key::Character(ch) => {
                                        crate::events::Key::Character(ch.to_string())
                                    }
                                    _ => crate::events::Key::Unknown,
                                };

                                let input_event = InputEvent::Keyboard {
                                    key,
                                    state: self.convert_button_state(event.state),
                                    modifiers: crate::events::Modifiers::new(), // TODO: Get actual modifiers
                                };
                                self.handle_input_event(input_event);
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_update) >= target_frame_time || !self.config.vsync {
                                self.update();
                                if let Err(e) = self.render() {
                                    if self.config.debug_mode {
                                        eprintln!("Render error: {}", e);
                                    }
                                }
                                last_update = now;
                            }
                        }
                        _ => {}
                    }
                }
                WinitEvent::AboutToWait => {
                    // Request redraw
                    window.request_redraw();
                }
                WinitEvent::DeviceEvent { .. } => {
                    // Handle device events if needed
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    /// Initialize after window creation
    fn initialize(&mut self, window: Arc<WinitWindow>) -> Result<()> {
        // Create surface using the context
        self.context.create_surface(window)?;

        // Get the surface format for the renderer
        let surface_format = if let Some(ref surface) = self.context.surface {
            surface.config.format
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        };

        // Update renderer with correct surface format
        self.renderer = crate::renderer::Renderer::new(&self.context.device, surface_format)?;

        // Create desktop UI
        self.create_desktop_ui();

        // Create demo window to test the system
        self.create_demo_window();

        // Preload assets
        self.asset_manager.preload_ui_assets(&self.context.device)?;

        self.running = true;
        println!("✅ Surface created and compositor fully initialized!");
        Ok(())
    }

    /// Create macOS-style desktop UI with dock and proper layout
    fn create_desktop_ui(&mut self) {
        // Create macOS-style dock at the bottom
        let mut dock = Container::new("dock".to_string())
            .with_direction(LayoutDirection::Horizontal);

        // Dock constraints - centered, glass effect, proper macOS sizing
        dock.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(400.0, 68.0)), // macOS dock height
            min_size: Size::new(300.0, 68.0),
            max_size: Size::new(600.0, 68.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: None,
        };

        // macOS-style dock background (translucent glass effect)
        let mut dock_background = Rectangle::new("dock_background".to_string())
            .with_color(Color::new(0.1, 0.1, 0.1, 0.8)) // Dark translucent
            .with_corner_radius(16.0); // Rounded corners like macOS

        // Create dock app icons with proper macOS styling
        let mut finder_icon = Button::new("finder".to_string(), "📁".to_string())
            .with_colors(
                Color::new(0.2, 0.2, 0.2, 0.9), // Subtle background
                Color::new(0.3, 0.3, 0.3, 0.9), // Hover
                Color::new(0.4, 0.4, 0.4, 0.9), // Active
            );
        finder_icon.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(52.0, 52.0)), // macOS dock icon size
            min_size: Size::new(48.0, 48.0),
            max_size: Size::new(64.0, 64.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: Some(1.0), // Square icons
        };

        let mut safari_icon = Button::new("safari".to_string(), "🌐".to_string())
            .with_colors(
                Color::new(0.2, 0.2, 0.2, 0.9),
                Color::new(0.3, 0.3, 0.3, 0.9),
                Color::new(0.4, 0.4, 0.4, 0.9),
            );
        safari_icon.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(52.0, 52.0)),
            min_size: Size::new(48.0, 48.0),
            max_size: Size::new(64.0, 64.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: Some(1.0),
        };

        let mut terminal_icon = Button::new("terminal".to_string(), "⚫".to_string())
            .with_colors(
                Color::new(0.2, 0.2, 0.2, 0.9),
                Color::new(0.3, 0.3, 0.3, 0.9),
                Color::new(0.4, 0.4, 0.4, 0.9),
            );
        terminal_icon.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(52.0, 52.0)),
            min_size: Size::new(48.0, 48.0),
            max_size: Size::new(64.0, 64.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: Some(1.0),
        };

        let mut settings_icon = Button::new("settings".to_string(), "⚙️".to_string())
            .with_colors(
                Color::new(0.2, 0.2, 0.2, 0.9),
                Color::new(0.3, 0.3, 0.3, 0.9),
                Color::new(0.4, 0.4, 0.4, 0.9),
            );
        settings_icon.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(52.0, 52.0)),
            min_size: Size::new(48.0, 48.0),
            max_size: Size::new(64.0, 64.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: Some(1.0),
        };

        // Add icons to dock with proper spacing
        dock.add_child(Box::new(finder_icon));
        dock.add_child(Box::new(safari_icon));
        dock.add_child(Box::new(terminal_icon));
        dock.add_child(Box::new(settings_icon));

        // Create desktop container with stack layout
        let mut desktop = Container::new("desktop".to_string())
            .with_direction(LayoutDirection::Stack);

        desktop.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(self.context.size.width as f32, self.context.size.height as f32)),
            min_size: Size::new(0.0, 0.0),
            max_size: Size::new(f32::INFINITY, f32::INFINITY),
            flex_grow: 1.0,
            flex_shrink: 0.0,
            aspect_ratio: None,
        };

        // macOS-style gradient wallpaper
        let mut wallpaper = Rectangle::new("wallpaper".to_string())
            .with_color(Color::from_hex("#4c1d95").unwrap_or(Color::new(0.3, 0.11, 0.58, 1.0))); // macOS purple gradient
        wallpaper.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(self.context.size.width as f32, self.context.size.height as f32)),
            min_size: Size::new(0.0, 0.0),
            max_size: Size::new(f32::INFINITY, f32::INFINITY),
            flex_grow: 1.0,
            flex_shrink: 0.0,
            aspect_ratio: None,
        };

        desktop.add_child(Box::new(wallpaper));
        desktop.add_child(Box::new(dock));

        self.desktop_ui.set_root(Box::new(desktop));
    }

    /// Create a demo window with macOS-style design
    fn create_demo_window(&mut self) {
        // Create macOS-style window content with proper styling
        let mut demo_content = Container::new("demo_content".to_string())
            .with_direction(LayoutDirection::Vertical);

        // Window content padding
        // demo_content.layout_mut().padding = crate::ui::Padding {
        //     top: 16.0,
        //     bottom: 16.0,
        //     left: 16.0,
        //     right: 16.0,
        // };

        // macOS-style title with proper typography
        let title = Text::new("demo_title".to_string(), "PrismaUI Desktop Environment".to_string())
            .with_font_size(20.0)
            .with_color(Color::new(0.1, 0.1, 0.1, 1.0)); // Dark text for contrast

        // macOS-style buttons with proper colors
        let mut button1 = Button::new("demo_button1".to_string(), "Open Finder".to_string())
            .with_colors(
                Color::new(0.0, 0.48, 1.0, 1.0),     // macOS blue
                Color::new(0.0, 0.42, 0.9, 1.0),     // Hover
                Color::new(0.0, 0.36, 0.8, 1.0),     // Active
            );
        button1.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(140.0, 32.0)),
            min_size: Size::new(100.0, 28.0),
            max_size: Size::new(200.0, 36.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: None,
        };

        let mut button2 = Button::new("demo_button2".to_string(), "Launch Terminal".to_string())
            .with_colors(
                Color::new(0.55, 0.55, 0.55, 1.0),   // macOS gray
                Color::new(0.65, 0.65, 0.65, 1.0),   // Hover
                Color::new(0.45, 0.45, 0.45, 1.0),   // Active
            );
        button2.layout_mut().constraints = LayoutConstraints {
            preferred_size: Some(Size::new(140.0, 32.0)),
            min_size: Size::new(100.0, 28.0),
            max_size: Size::new(200.0, 36.0),
            flex_grow: 0.0,
            flex_shrink: 0.0,
            aspect_ratio: None,
        };

        // Subtle description text
        let description = Text::new("demo_description".to_string(),
            "A high-performance GPU-accelerated desktop environment built with WGPU and Rust.".to_string())
            .with_font_size(14.0)
            .with_color(Color::new(0.4, 0.4, 0.4, 1.0)); // Subtle gray

        demo_content.add_child(Box::new(title));
        demo_content.add_child(Box::new(button1));
        demo_content.add_child(Box::new(button2));
        demo_content.add_child(Box::new(description));

        // Create the window with macOS-style decorations
        let window_id = self.window_manager.create_window(
            "PrismaUI Demo".to_string(), // macOS-style window title
            Box::new(demo_content),
        );

        self.demo_window_id = Some(window_id);
    }

    /// Handle window resize
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.context.resize(new_size);

        // Update desktop bounds
        let desktop_bounds = Rect::new(0.0, 0.0, new_size.width as f32, new_size.height as f32);
        self.window_manager.set_desktop_bounds(desktop_bounds);

        // Update desktop UI layout
        let desktop_size = Size::new(new_size.width as f32, new_size.height as f32);
        self.desktop_ui.layout(desktop_size);
        println!("📐 Resized to {}x{}, desktop layout updated", new_size.width, new_size.height);
    }

    /// Main update loop
    fn update(&mut self) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Update performance metrics
        self.update_performance_metrics(delta_time);

        // Update window manager
        self.window_manager.update();

        // Update text layouts for UI elements that need it
        // TODO: This should be more efficient, only updating dirty text
        // self.update_text_layouts();
    }

    /// Render a frame
    fn render(&mut self) -> Result<()> {
        let frame_start = Instant::now();

        // Get surface texture from context
        let surface = self.context.surface.as_ref().ok_or_else(|| anyhow::anyhow!("No surface available for rendering"))?;
        let output = surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create encoder
        let mut encoder = self.context.device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.15,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render desktop UI
            let desktop_layers = self.desktop_ui.render();

            // Render windows
            let window_layers = self.window_manager.render();

            // Combine all layers
            let mut all_layers = Vec::new();
            all_layers.extend(desktop_layers);
            all_layers.extend(window_layers);

            // Debug: Print total number of render commands (less frequent)
            if self.config.debug_mode {
                let total_commands: usize = all_layers.iter().map(|layer| layer.commands.len()).sum();
                if self.frame_count % 300 == 0 && total_commands > 0 {
                    println!("🎨 Frame {}: {} render layers, {} total commands",
                        self.frame_count, all_layers.len(), total_commands);
                }
            }

            // Begin frame and render
            let time = self.frame_count as f32 * 0.016; // Approximate time
            self.renderer.begin_frame(&self.context, time);
            self.renderer.add_layers(all_layers);
            self.renderer.end_frame(&mut render_pass, &self.context.device)?;
        }

        // Submit commands
        self.context.device.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Update performance metrics
        let frame_time = frame_start.elapsed();
        self.performance_metrics.cpu_time_ms = frame_time.as_secs_f32() * 1000.0;
        self.performance_metrics.draw_calls = self.renderer.stats.draw_calls;
        self.performance_metrics.triangles_rendered = self.renderer.stats.triangles_rendered;

        self.frame_count += 1;

        Ok(())
    }

    /// Handle input events
    fn handle_input_event(&mut self, input_event: InputEvent) {
        let event = Event::Input(input_event);

        // Try desktop UI first
        if self.desktop_ui.handle_event(&event) {
            return;
        }

        // Then try window manager
        if self.window_manager.handle_event(&event) {
            return;
        }

        // Finally try event dispatcher
        self.event_dispatcher.dispatch(&event);
    }

    /// Convert winit events to compositor events
    fn convert_mouse_button(&self, button: winit::event::MouseButton) -> MouseButton {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Other(4),
            winit::event::MouseButton::Forward => MouseButton::Other(5),
            winit::event::MouseButton::Other(id) => MouseButton::Other(id as u8),
        }
    }

    fn convert_button_state(&self, state: winit::event::ElementState) -> ButtonState {
        match state {
            winit::event::ElementState::Pressed => ButtonState::Pressed,
            winit::event::ElementState::Released => ButtonState::Released,
        }
    }

    fn convert_modifiers(&self, modifiers: winit::keyboard::ModifiersState) -> Modifiers {
        Modifiers {
            shift: modifiers.shift_key(),
            ctrl: modifiers.control_key(),
            alt: modifiers.alt_key(),
            meta: modifiers.super_key(),
        }
    }

    /// Update performance metrics
    fn update_performance_metrics(&mut self, delta_time: Duration) {
        self.performance_metrics.frame_time_ms = delta_time.as_secs_f32() * 1000.0;

        if delta_time.as_secs_f32() > 0.0 {
            self.performance_metrics.fps = 1.0 / delta_time.as_secs_f32();
        }

        // Update memory usage (simplified)
        let asset_stats = self.asset_manager.memory_stats();
        self.performance_metrics.memory_usage_mb = asset_stats.total_memory_mb() as f32;

        // Print debug info if enabled (less frequent)
        if self.config.debug_mode && self.frame_count % 600 == 0 {
            println!("🚀 Performance: Frame {}, FPS: {:.1}, Frame Time: {:.2}ms, Memory: {:.1}MB, Draw Calls: {}, Triangles: {}",
                self.frame_count,
                self.performance_metrics.fps,
                self.performance_metrics.frame_time_ms,
                self.performance_metrics.memory_usage_mb,
                self.performance_metrics.draw_calls,
                self.performance_metrics.triangles_rendered
            );
        }
    }

    /// Get current performance metrics
    pub fn performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_metrics
    }

    /// Stop the compositor
    pub fn stop(&mut self) {
        self.running = false;
    }
}

// TODO: Application handler for winit event loop
// This will be implemented when we add proper event loop integration
// struct CompositorApp {
//     compositor: Compositor,
// }

// TODO: Advanced compositor features to be implemented:
//
// 1. Multi-monitor support
//    - Monitor detection and configuration
//    - Per-monitor DPI scaling
//    - Window spanning across monitors
//
// 2. Advanced rendering features
//    - GPU-accelerated blur effects
//    - Drop shadows and lighting
//    - Particle systems for effects
//    - Post-processing pipeline
//
// 3. Performance optimizations
//    - Frustum culling
//    - Level-of-detail rendering
//    - Async command buffer recording
//    - GPU memory pooling
//
// 4. Platform integration
//    - Native file dialogs
//    - System notifications
//    - Clipboard integration
//    - Drag and drop from external apps
//
// 5. Debugging and profiling
//    - GPU frame capture integration
//    - Performance overlay
//    - Memory leak detection
//    - Real-time profiling tools
