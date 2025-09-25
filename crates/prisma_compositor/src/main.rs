/// PrismaUI Compositor - High-Performance WGPU Desktop Environment
///
/// This is a complete rewrite of the PrismaUI desktop environment using WGPU
/// for maximum performance, designed for smoothness and responsiveness.

use std::sync::Arc;
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::LogicalSize,
};

use prisma_compositor::{
    core::{Compositor, CompositorConfig},
    ui::{UISystem, UIElement, UIElementType, UIRect},
    window::WindowManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🚀 PrismaUI Compositor - High-Performance Desktop Environment");
    println!("   Built with WGPU for maximum performance and efficiency");

    // Create event loop
    let event_loop = EventLoop::new();

    // Create main window
    let window = WindowBuilder::new()
        .with_title("PrismaUI - High-Performance Desktop")
        .with_inner_size(LogicalSize::new(1920, 1080))
        .with_decorations(false) // We'll handle our own decorations
        .build(&event_loop)?;

    let window = Arc::new(window);

    // Configure compositor for high performance
    let config = CompositorConfig {
        max_render_threads: num_cpus::get().min(8),
        max_compute_threads: (num_cpus::get() / 2).max(1),
        parallel_command_recording: true,
        buffer_pool_size: 512, // 512MB buffer pool
        texture_pool_size: 1024, // 1GB texture pool
        max_frames_in_flight: 3,
        gpu_culling: true,
        temporal_reprojection: true,
        vsync: true,
        msaa_samples: 4,
    };

    println!("🔧 Compositor Configuration:");
    println!("   - Render Threads: {}", config.max_render_threads);
    println!("   - Compute Threads: {}", config.max_compute_threads);
    println!("   - Buffer Pool: {}MB", config.buffer_pool_size);
    println!("   - Texture Pool: {}MB", config.texture_pool_size);
    println!("   - MSAA: {}x", config.msaa_samples);

    // Initialize compositor
    println!("🔄 Initializing high-performance compositor...");
    let compositor = Compositor::new(Arc::clone(&window), config).await?;

    println!("✨ Creating desktop environment...");

    // Create desktop UI
    create_desktop_ui(&compositor).await?;

    println!("🎮 Starting main render loop...");
    println!("   Press Ctrl+C to exit");

    // Start main loop
    compositor.run(event_loop).await?;

    println!("👋 PrismaUI Compositor shutdown complete");
    Ok(())
}

/// Create the desktop UI with all components
async fn create_desktop_ui(compositor: &Arc<Compositor>) -> Result<(), Box<dyn std::error::Error>> {
    let ui_system = compositor.window_manager.read().unwrap().get_ui_system();

    // Create desktop layers
    let wallpaper_layer = ui_system.create_layer("Wallpaper".to_string());
    let desktop_layer = ui_system.create_layer("Desktop".to_string());
    let windows_layer = ui_system.create_layer("Windows".to_string());
    let taskbar_layer = ui_system.create_layer("Taskbar".to_string());

    // Create wallpaper
    let wallpaper = UIElement::rect(
        1,
        UIRect::new(0.0, 0.0, 1920.0, 1080.0),
        [0.1, 0.2, 0.3, 1.0], // Dark blue gradient-like background
    );
    ui_system.add_element_to_layer(wallpaper_layer, wallpaper);

    // Create taskbar
    create_taskbar(&ui_system, taskbar_layer)?;

    // Create sample windows
    create_sample_windows(&ui_system, windows_layer).await?;

    println!("✅ Desktop environment created successfully");
    Ok(())
}

/// Create the taskbar with system tray and window buttons
fn create_taskbar(ui_system: &Arc<UISystem>, layer_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let taskbar_height = 48.0;
    let screen_width = 1920.0;
    let screen_height = 1080.0;

    // Taskbar background
    let taskbar_bg = UIElement::rect(
        2,
        UIRect::new(0.0, screen_height - taskbar_height, screen_width, taskbar_height),
        [0.1, 0.1, 0.1, 0.95], // Semi-transparent dark background
    );
    ui_system.add_element_to_layer(layer_id, taskbar_bg);

    // Start button
    let start_button = UIElement::button(
        3,
        UIRect::new(8.0, screen_height - taskbar_height + 8.0, 32.0, 32.0),
        "⊞".to_string(), // Windows-like start button
        0,
    );
    ui_system.add_element_to_layer(layer_id, start_button);

    // Search button
    let search_button = UIElement::button(
        4,
        UIRect::new(48.0, screen_height - taskbar_height + 8.0, 32.0, 32.0),
        "🔍".to_string(),
        0,
    );
    ui_system.add_element_to_layer(layer_id, search_button);

    // System tray area
    let tray_width = 200.0;
    let tray_bg = UIElement::rect(
        5,
        UIRect::new(
            screen_width - tray_width - 8.0,
            screen_height - taskbar_height + 8.0,
            tray_width,
            32.0,
        ),
        [0.15, 0.15, 0.15, 0.9],
    );
    ui_system.add_element_to_layer(layer_id, tray_bg);

    // Clock
    let clock = UIElement::text(
        6,
        UIRect::new(
            screen_width - 80.0,
            screen_height - taskbar_height + 12.0,
            70.0,
            24.0,
        ),
        "12:34 PM".to_string(),
        0,
        [1.0, 1.0, 1.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, clock);

    println!("📊 Taskbar created with system tray");
    Ok(())
}

/// Create sample windows to demonstrate the window management system
async fn create_sample_windows(ui_system: &Arc<UISystem>, layer_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample text editor window
    create_text_editor_window(ui_system, layer_id, 7, 100.0, 100.0, 800.0, 600.0)?;

    // Create a sample file manager window
    create_file_manager_window(ui_system, layer_id, 100, 200.0, 150.0, 700.0, 500.0)?;

    // Create a sample terminal window
    create_terminal_window(ui_system, layer_id, 200, 300.0, 200.0, 600.0, 400.0)?;

    println!("🪟 Sample windows created");
    Ok(())
}

/// Create a text editor window
fn create_text_editor_window(
    ui_system: &Arc<UISystem>,
    layer_id: u64,
    base_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Window background
    let window_bg = UIElement::rect(
        base_id,
        UIRect::new(x, y, width, height),
        [0.95, 0.95, 0.95, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, window_bg);

    // Title bar
    let title_bar = UIElement::rect(
        base_id + 1,
        UIRect::new(x, y, width, 32.0),
        [0.3, 0.5, 0.8, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_bar);

    // Title text
    let title_text = UIElement::text(
        base_id + 2,
        UIRect::new(x + 8.0, y + 6.0, width - 100.0, 20.0),
        "Text Editor".to_string(),
        0,
        [1.0, 1.0, 1.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_text);

    // Close button
    let close_button = UIElement::button(
        base_id + 3,
        UIRect::new(x + width - 28.0, y + 4.0, 24.0, 24.0),
        "×".to_string(),
        0,
    );
    ui_system.add_element_to_layer(layer_id, close_button);

    // Content area
    let content_area = UIElement::rect(
        base_id + 4,
        UIRect::new(x + 4.0, y + 36.0, width - 8.0, height - 40.0),
        [1.0, 1.0, 1.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, content_area);

    // Sample text content
    let sample_text = UIElement::text(
        base_id + 5,
        UIRect::new(x + 12.0, y + 44.0, width - 24.0, 24.0),
        "// Welcome to PrismaUI High-Performance Text Editor".to_string(),
        0,
        [0.0, 0.0, 0.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, sample_text);

    Ok(())
}

/// Create a file manager window
fn create_file_manager_window(
    ui_system: &Arc<UISystem>,
    layer_id: u64,
    base_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Window background
    let window_bg = UIElement::rect(
        base_id,
        UIRect::new(x, y, width, height),
        [0.9, 0.9, 0.9, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, window_bg);

    // Title bar
    let title_bar = UIElement::rect(
        base_id + 1,
        UIRect::new(x, y, width, 32.0),
        [0.2, 0.7, 0.4, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_bar);

    // Title text
    let title_text = UIElement::text(
        base_id + 2,
        UIRect::new(x + 8.0, y + 6.0, width - 100.0, 20.0),
        "File Manager".to_string(),
        0,
        [1.0, 1.0, 1.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_text);

    // Toolbar
    let toolbar = UIElement::rect(
        base_id + 3,
        UIRect::new(x + 4.0, y + 36.0, width - 8.0, 32.0),
        [0.8, 0.8, 0.8, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, toolbar);

    // Address bar
    let address_bar = UIElement::rect(
        base_id + 4,
        UIRect::new(x + 8.0, y + 40.0, width - 16.0, 24.0),
        [1.0, 1.0, 1.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, address_bar);

    // Address text
    let address_text = UIElement::text(
        base_id + 5,
        UIRect::new(x + 12.0, y + 43.0, width - 24.0, 18.0),
        "C:\\Users\\User\\Documents".to_string(),
        0,
        [0.0, 0.0, 0.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, address_text);

    Ok(())
}

/// Create a terminal window
fn create_terminal_window(
    ui_system: &Arc<UISystem>,
    layer_id: u64,
    base_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Window background (dark terminal)
    let window_bg = UIElement::rect(
        base_id,
        UIRect::new(x, y, width, height),
        [0.05, 0.05, 0.05, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, window_bg);

    // Title bar
    let title_bar = UIElement::rect(
        base_id + 1,
        UIRect::new(x, y, width, 32.0),
        [0.1, 0.1, 0.1, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_bar);

    // Title text
    let title_text = UIElement::text(
        base_id + 2,
        UIRect::new(x + 8.0, y + 6.0, width - 100.0, 20.0),
        "Terminal".to_string(),
        0,
        [0.8, 0.8, 0.8, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, title_text);

    // Terminal content
    let terminal_line1 = UIElement::text(
        base_id + 3,
        UIRect::new(x + 8.0, y + 40.0, width - 16.0, 16.0),
        "user@prisma:~$ ls -la".to_string(),
        0,
        [0.0, 1.0, 0.0, 1.0], // Green terminal text
    );
    ui_system.add_element_to_layer(layer_id, terminal_line1);

    let terminal_line2 = UIElement::text(
        base_id + 4,
        UIRect::new(x + 8.0, y + 58.0, width - 16.0, 16.0),
        "drwxr-xr-x  5 user user  4096 Dec 25 12:34 Documents".to_string(),
        0,
        [0.8, 0.8, 0.8, 1.0], // Gray terminal output
    );
    ui_system.add_element_to_layer(layer_id, terminal_line2);

    let terminal_line3 = UIElement::text(
        base_id + 5,
        UIRect::new(x + 8.0, y + 76.0, width - 16.0, 16.0),
        "drwxr-xr-x  3 user user  4096 Dec 25 12:34 Desktop".to_string(),
        0,
        [0.8, 0.8, 0.8, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, terminal_line3);

    // Cursor
    let cursor = UIElement::rect(
        base_id + 6,
        UIRect::new(x + 8.0, y + 100.0, 8.0, 16.0),
        [0.0, 1.0, 0.0, 1.0],
    );
    ui_system.add_element_to_layer(layer_id, cursor);

    Ok(())
}