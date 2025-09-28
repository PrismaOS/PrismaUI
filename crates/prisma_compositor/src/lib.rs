/// High-performance WGPU-based compositor for PrismaUI
///
/// This compositor is designed for maximum performance with:
/// - GPU-accelerated rendering pipeline
/// - Efficient batching and instancing
/// - Zero-copy buffer management
/// - Multi-threaded architecture
/// - Advanced memory management

pub mod core;
pub mod renderer;
pub mod ui;
pub mod window;
pub mod compositor;
pub mod geometry;
pub mod text;
pub mod assets;
pub mod events;
pub mod animation; // TODO: Animation system module - currently placeholder

// Re-exports for public API
pub use core::{Device, Surface, Context};
pub use renderer::{Renderer, RenderLayer, RenderCommand};
pub use ui::{UIElement, UITree, Layout, LayoutConstraints};
pub use window::{Window, WindowManager, WindowId};
pub use compositor::{Compositor, CompositorConfig};
pub use geometry::{Rect, Point, Size, Transform, Color};
pub use text::{TextRenderer, FontManager, GlyphAtlas};
pub use assets::{AssetManager, Texture, TextureAtlas};
pub use events::{Event, InputEvent, WindowEvent};

/// Initialize the compositor system (async)
pub async fn init() -> anyhow::Result<Compositor> {
    Compositor::new(CompositorConfig::default()).await
}