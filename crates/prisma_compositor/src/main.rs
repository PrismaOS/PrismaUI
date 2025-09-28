/// PrismaUI Compositor - High-Performance WGPU Desktop Environment
///
/// This is a complete rewrite of the PrismaUI desktop environment using WGPU
/// for maximum performance, designed for smoothness and responsiveness.

use prisma_compositor::{Compositor, CompositorConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("🚀 Launching PrismaUI Compositor - GPU-Accelerated Desktop Environment");
    println!("=====================================================================");
    println!();

    // Create compositor configuration
    let config = CompositorConfig {
        title: "PrismaUI - GPU Desktop Environment".to_string(),
        size: winit::dpi::PhysicalSize::new(1920, 1080),
        target_fps: 144, // Higher refresh rate for smoother experience
        vsync: false, // Disable vsync for maximum performance
        debug_mode: false, // Disable debug for better performance
    };

    println!("🔧 Initializing compositor with config:");
    println!("   - Resolution: {}x{}", config.size.width, config.size.height);
    println!("   - Target FPS: {}", config.target_fps);
    println!("   - VSync: {}", config.vsync);
    println!("   - Debug Mode: {}", config.debug_mode);
    println!();

    // Create and initialize the compositor
    println!("⚡ Creating GPU-accelerated compositor...");
    let compositor = Compositor::new(config).await?;

    println!("✅ Compositor initialized successfully!");
    println!();
    println!("🎨 Architecture features:");
    println!("   • WGPU-based rendering pipeline with efficient batching");
    println!("   • Hardware-accelerated text rendering with glyph atlas");
    println!("   • Advanced window management with decorations");
    println!("   • Comprehensive event system with hit testing");
    println!("   • Asset management for textures and resources");
    println!("   • Animation system architecture (ready for implementation)");
    println!();

    // Launch the compositor
    println!("🚀 Launching compositor event loop...");
    compositor.run()?;

    println!("👋 Compositor shutdown complete.");
    Ok(())
}
