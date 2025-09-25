/// PrismaUI - Complete OS system UI built with GPUI
pub mod animations;
pub mod assets;
pub mod components;
pub mod desktop;
pub mod shell;
pub mod window_manager;

pub use animations::PremiumAnimations;
pub use assets::Assets;
pub use components::{AppMenu, CommandPalette, Taskbar, Wallpaper};
pub use desktop::Desktop;
pub use shell::SystemShell;
pub use window_manager::{WindowEvent, WindowId, WindowManager};

/// Initialize PrismaUI - call this before using any components
pub fn init(cx: &mut gpui::App) {
    gpui_component::init(cx);
}
