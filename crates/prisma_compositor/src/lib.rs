/// High-performance WGPU-based compositor for PrismaUI
///
/// This compositor is designed for performance with:
/// - Multi-threaded rendering pipeline
/// - Zero-copy buffer management
/// - GPU-accelerated compositing
/// - Advanced memory management
/// - Efficient resource pooling

pub mod core;
pub mod renderer;
pub mod window;
pub mod ui;
pub mod assets;
pub mod threading;
pub mod memory;

pub use core::{Compositor, CompositorConfig};
pub use renderer::{WgpuRenderer, RenderCommand, RenderFrame};
pub use window::{WindowManager, Window as CompositorWindow, WindowId};
pub use ui::{UILayer, UIElement, UIRect, UIText, UIImage};
pub use assets::{AssetManager, AssetCache, TextureAtlas};