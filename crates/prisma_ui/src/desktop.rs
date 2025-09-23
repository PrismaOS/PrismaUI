/// Desktop environment - main shell orchestrating all components
use gpui::{
    div, px, size, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Point, Render, Size, Styled, Window, AppContext
};
use gpui_component::{ActiveTheme, StyledExt};
use serde::{Deserialize, Serialize};

use crate::{
    window_manager::{WindowManager, WindowEvent},
    components::{AppMenu, CommandPalette, Taskbar, Wallpaper},
    shell::SystemShell,
};

/// Desktop events
#[derive(Clone, Debug)]
pub enum DesktopEvent {
    /// Desktop resolution changed
    ResolutionChanged { size: Size<gpui::Pixels> },
    /// Desktop theme changed
    ThemeChanged,
    /// Desktop locked
    Locked,
    /// Desktop unlocked
    Unlocked,
}

/// Desktop component state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesktopState {
    pub wallpaper_path: Option<String>,
    pub theme: String,
    pub taskbar_position: String,
}

/// Main desktop environment coordinating the entire OS UI
pub struct Desktop {
    /// Current desktop bounds (screen resolution)
    bounds: Bounds<gpui::Pixels>,
    /// Wallpaper component
    wallpaper: Entity<Wallpaper>,
    /// Window manager for application windows
    window_manager: Entity<WindowManager>,
    /// System shell managing UI interactions
    shell: Entity<SystemShell>,
    /// App menu for launching applications
    app_menu: Entity<AppMenu>,
    /// Command palette for quick actions
    command_palette: Entity<CommandPalette>,
    /// Taskbar for window switching and system tray
    taskbar: Entity<Taskbar>,
    /// Whether desktop is locked
    locked: bool,
    /// Focus handle for the desktop
    focus_handle: FocusHandle,
}

impl Desktop {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Get initial screen bounds
        let bounds = if let Some(display) = cx.primary_display() {
            display.bounds()
        } else {
            Bounds {
                origin: Point::default(),
                size: size(px(1920.0), px(1080.0)),
            }
        };

        // Create core components
        let wallpaper = Wallpaper::create(bounds, cx);
        let app_menu = AppMenu::create(window, cx);
        let command_palette = CommandPalette::create(window, cx);
        let taskbar = Taskbar::create(bounds, app_menu.clone(), command_palette.clone(), window, cx);

        // Get available desktop area (excluding taskbar)
        let desktop_area = taskbar.read(cx).desktop_area();
        let window_manager = WindowManager::new(desktop_area, cx);

        // Connect taskbar to window manager
        taskbar.update(cx, |taskbar, cx| {
            taskbar.set_window_manager(window_manager.clone(), cx);
        });

        let shell = SystemShell::create(
            bounds,
            window_manager.clone(),
            app_menu.clone(),
            command_palette.clone(),
            cx,
        );

        // Set up wallpaper with default image
        wallpaper.update(cx, |wallpaper, cx| {
            // Try to load a default wallpaper
            wallpaper.set_image(Some("default_wallpaper.jpg".to_string()), cx);
        });

        // Subscribe to window manager events
        cx.subscribe(&window_manager, |this, window_manager, event, cx| {
            this.handle_window_event(window_manager, event, cx);
        })
        .detach();

        // Set up periodic time updates for taskbar
        cx.spawn(|(desktop, mut cx)| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                _ = desktop.update(&mut cx, |desktop, cx| {
                    desktop.taskbar.update(cx, |taskbar, cx| {
                        taskbar.update_time(cx);
                    });
                });
            }
        })
        .detach();

        Self {
            bounds,
            wallpaper,
            window_manager,
            shell,
            app_menu,
            command_palette,
            taskbar,
            locked: false,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Handle window manager events
    fn handle_window_event(
        &mut self,
        _window_manager: Entity<WindowManager>,
        event: &WindowEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            WindowEvent::Closed(_) => {
                // Update taskbar when windows change
                cx.notify();
            }
            WindowEvent::Focused(_) => {
                // Update focus state
                cx.notify();
            }
            WindowEvent::Moved { .. } | WindowEvent::Resized { .. } => {
                // Update for potential effects
                cx.notify();
            }
            _ => {}
        }
    }

    /// Create a new application window
    pub fn create_app_window<V: 'static + Render>(
        &mut self,
        title: String,
        content: Entity<V>,
        initial_bounds: Option<Bounds<gpui::Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> crate::window_manager::WindowId {
        self.window_manager.update(cx, |wm, cx| {
            wm.create_window(title, content, initial_bounds, window, cx)
        })
    }

    /// Lock the desktop
    pub fn lock(&mut self, cx: &mut Context<Self>) {
        self.locked = true;
        cx.emit(DesktopEvent::Locked);
        cx.notify();
    }

    /// Unlock the desktop
    pub fn unlock(&mut self, cx: &mut Context<Self>) {
        self.locked = false;
        cx.emit(DesktopEvent::Unlocked);
        cx.notify();
    }

    /// Handle screen resolution changes
    pub fn handle_resolution_change(&mut self, new_size: Size<gpui::Pixels>, cx: &mut Context<Self>) {
        let new_bounds = Bounds {
            origin: self.bounds.origin,
            size: new_size,
        };
        self.bounds = new_bounds;

        // Update all components with new bounds
        self.wallpaper.update(cx, |wallpaper, cx| {
            wallpaper.set_bounds(new_bounds, cx);
        });

        self.taskbar.update(cx, |taskbar, cx| {
            taskbar.set_bounds(new_bounds, cx);
        });

        let desktop_area = self.taskbar.read(cx).desktop_area();
        self.window_manager.update(cx, |wm, cx| {
            wm.desktop_bounds = desktop_area;
            cx.notify();
        });

        cx.emit(DesktopEvent::ResolutionChanged { size: new_size });
        cx.notify();
    }

    /// Change wallpaper
    pub fn set_wallpaper(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        self.wallpaper.update(cx, |wallpaper, cx| {
            wallpaper.set_image(path, cx);
        });
    }

    /// Get desktop bounds
    pub fn bounds(&self) -> Bounds<gpui::Pixels> {
        self.bounds
    }

    /// Create a demo application window for testing
    pub fn create_demo_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Create a simple demo app content
        let demo_content = cx.new(|_| DemoApp::new());

        self.create_app_window(
            "Demo Application".to_string(),
            demo_content,
            Some(Bounds {
                origin: Point { x: px(100.0), y: px(100.0) },
                size: size(px(600.0), px(400.0)),
            }),
            window,
            cx,
        );
    }
}

impl EventEmitter<DesktopEvent> for Desktop {}

impl Focusable for Desktop {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(gpui::black()) // Fallback background
            // Wallpaper layer
            .child(self.wallpaper.clone())
            // Window manager layer
            .child(self.window_manager.clone())
            // System shell overlay
            .child(self.shell.clone())
            // App menu overlay
            .child(self.app_menu.clone())
            // Command palette overlay
            .child(self.command_palette.clone())
            // Taskbar (always on top)
            .child(self.taskbar.clone())
            // Lock screen overlay (if locked)
            .when(self.locked, |this| {
                this.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(gpui::rgba(0.0, 0.0, 0.0, 0.8))
                        .backdrop_blur()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .p_8()
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius)
                                .shadow_xl()
                                .child("Desktop Locked")
                        )
                )
            })
            // Global keyboard shortcuts
            .on_key_down(cx.listener(|this, event, window, cx| {
                match event.keystroke.key.as_str() {
                    "Space" if event.keystroke.modifiers.cmd => {
                        // Cmd+Space for command palette
                        this.command_palette.update(cx, |palette, cx| {
                            palette.toggle(window, cx);
                        });
                    }
                    "F4" if event.keystroke.modifiers.alt => {
                        // Alt+F4 to close focused window
                        // TODO: Implement window closing
                    }
                    "Tab" if event.keystroke.modifiers.alt => {
                        // Alt+Tab for window switching
                        // TODO: Implement window switcher
                    }
                    "d" if event.keystroke.modifiers.cmd => {
                        // Cmd+D to create demo window
                        this.create_demo_window(window, cx);
                    }
                    _ => {}
                }
            }))
    }
}

/// Simple demo application for testing the window system
pub struct DemoApp {
    counter: u32,
}

impl DemoApp {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl Render for DemoApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, button::Button};

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .p_6()
            .bg(cx.theme().background)
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(cx.theme().foreground)
                    .child("Demo Application")
            )
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Counter: {}", self.counter))
            )
            .child(
                Button::new("increment")
                    .primary()
                    .label("Increment")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.counter += 1;
                        cx.notify();
                    }))
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("This is a demo window to test the window management system.")
            )
    }
}