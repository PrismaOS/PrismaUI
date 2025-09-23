/// PrismaUI - Complete OS system UI built with GPUI
pub mod assets;
pub mod components;
pub mod desktop;
pub mod shell;
pub mod window_manager;

pub use assets::Assets;
pub use desktop::Desktop;
pub use shell::SystemShell;
pub use window_manager::{WindowManager, WindowId, WindowEvent};
pub use components::{AppMenu, CommandPalette, Taskbar, Wallpaper};

/// Initialize PrismaUI - call this before using any components
pub fn init(cx: &mut gpui::App) {
    gpui_component::init(cx);
}