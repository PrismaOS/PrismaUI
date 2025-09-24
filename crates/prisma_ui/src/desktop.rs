/// Desktop environment - main shell orchestrating all components
use gpui::{
    div, px, size, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Point, Render, Size, Styled, Window, AppContext
};
use gpui::prelude::FluentBuilder;
use gpui_component::{ActiveTheme, StyledExt};
use serde::{Deserialize, Serialize};

use crate::{
    window_manager::{WindowManager, WindowEvent},
    components::{AppMenu, CommandPalette, Taskbar, Wallpaper, app_menu::AppMenuAction},
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
    /// Pending app launch requests
    pending_app_launches: Vec<String>,
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
            // Load the default wallpaper
            eprintln!("Desktop: Setting wallpaper to wallpapers/default_wallpaper.jpg");
            wallpaper.set_image(Some("wallpapers/default_wallpaper.jpg".to_string()), cx);
        });

        // Subscribe to window manager events
        cx.subscribe(&window_manager, |this, window_manager, event, cx| {
            this.handle_window_event(window_manager, event, cx);
        })
        .detach();

        // Subscribe to app menu events
        cx.subscribe(&app_menu, |this, app_menu, event: &AppMenuAction, cx| {
            this.handle_app_menu_event(app_menu, event, cx);
        })
        .detach();

        // Set up periodic time updates for taskbar
        cx.spawn(async move |desktop, cx| {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                _ = desktop.update(cx, |desktop, cx| {
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
            pending_app_launches: Vec::new(),
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

    /// Handle app menu events
    fn handle_app_menu_event(
        &mut self,
        _app_menu: Entity<AppMenu>,
        event: &AppMenuAction,
        cx: &mut Context<Self>,
    ) {
        match event {
            AppMenuAction::LaunchApp(app_id) => {
                self.launch_application(app_id, cx);
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

    /// Launch an application by ID
    fn launch_application(&mut self, app_id: &str, cx: &mut Context<Self>) {
        self.pending_app_launches.push(app_id.to_string());
        tracing::info!("Queuing application launch: {}", app_id);
        cx.notify();
    }

    /// Process pending app launches - called from render where we have window access
    fn process_pending_launches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let launches = std::mem::take(&mut self.pending_app_launches);

        for app_id in launches {
            match app_id.as_str() {
                "terminal" => {
                    let content = cx.new(|_| TerminalApp::new());
                    self.create_app_window(
                        "Terminal".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(200.0), y: px(150.0) },
                            size: size(px(800.0), px(600.0)),
                        }),
                        window,
                        cx,
                    );
                }
                "code_editor" => {
                    let content = cx.new(|_| CodeEditorApp::new());
                    self.create_app_window(
                        "Code Editor".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(150.0), y: px(100.0) },
                            size: size(px(1000.0), px(700.0)),
                        }),
                        window,
                        cx,
                    );
                }
                "file_manager" => {
                    let content = cx.new(|_| FileManagerApp::new());
                    self.create_app_window(
                        "File Manager".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(300.0), y: px(200.0) },
                            size: size(px(900.0), px(600.0)),
                        }),
                        window,
                        cx,
                    );
                }
                "web_browser" => {
                    let content = cx.new(|_| WebBrowserApp::new());
                    self.create_app_window(
                        "Web Browser".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(100.0), y: px(50.0) },
                            size: size(px(1200.0), px(800.0)),
                        }),
                        window,
                        cx,
                    );
                }
                "calculator" => {
                    let content = cx.new(|_| CalculatorApp::new());
                    self.create_app_window(
                        "Calculator".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(400.0), y: px(250.0) },
                            size: size(px(400.0), px(500.0)),
                        }),
                        window,
                        cx,
                    );
                }
                "settings" => {
                    let content = cx.new(|_| SettingsApp::new());
                    self.create_app_window(
                        "System Settings".to_string(),
                        content,
                        Some(Bounds {
                            origin: Point { x: px(250.0), y: px(150.0) },
                            size: size(px(800.0), px(600.0)),
                        }),
                        window,
                        cx,
                    );
                }
                _ => {
                    // Default demo app for unknown IDs
                    self.create_demo_window(window, cx);
                }
            }
        }
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
        // Process any pending app launches
        self.process_pending_launches(window, cx);

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
                        .bg(gpui::rgba(0x000000_CC))
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
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "Space" if event.keystroke.modifiers.control => {
                        // Ctrl/Cmd+Space for command palette
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
                    "d" if event.keystroke.modifiers.control => {
                        // Ctrl/Cmd+D to create demo window
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
        use gpui_component::{v_flex, button::{Button, ButtonVariants as _}};

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

/// Terminal Application
pub struct TerminalApp {
    commands: Vec<String>,
    current_command: String,
}

impl TerminalApp {
    pub fn new() -> Self {
        Self {
            commands: vec![
                "Welcome to PrismaUI Terminal".to_string(),
                "Type 'help' for available commands".to_string(),
            ],
            current_command: String::new(),
        }
    }
}

impl Render for TerminalApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex};

        v_flex()
            .size_full()
            .bg(gpui::black())
            .text_color(gpui::green())
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_lg()
                    .font_bold()
                    .text_color(gpui::white())
                    .child("Terminal")
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .child(
                        v_flex()
                            .gap_1()
                            .children(self.commands.iter().map(|cmd| {
                                div()
                                    .text_sm()
                                    .font_family("Monaco")
                                    .child(format!("$ {}", cmd))
                            }))
                    )
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_family("Monaco")
                            .child("$ _")
                    )
            )
    }
}

/// Code Editor Application
pub struct CodeEditorApp {
    content: String,
}

impl CodeEditorApp {
    pub fn new() -> Self {
        Self {
            content: "// Welcome to PrismaUI Code Editor\nfn main() {\n    println!(\"Hello, World!\");\n}".to_string(),
        }
    }
}

impl Render for CodeEditorApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, button::{Button, ButtonVariants as _}};

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Toolbar
                h_flex()
                    .h(px(40.0))
                    .items_center()
                    .px_4()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .gap_2()
                    .child(Button::new("file").ghost().label("File"))
                    .child(Button::new("edit").ghost().label("Edit"))
                    .child(Button::new("view").ghost().label("View"))
            )
            .child(
                // Editor area
                div()
                    .flex_1()
                    .p_4()
                    .bg(cx.theme().background)
                    .child(
                        div()
                            .text_sm()
                            .font_family("Monaco")
                            .text_color(cx.theme().foreground)
                            .child(self.content.clone())
                    )
            )
    }
}

/// File Manager Application
pub struct FileManagerApp {
    current_path: String,
    files: Vec<String>,
}

impl FileManagerApp {
    pub fn new() -> Self {
        Self {
            current_path: "/home/user".to_string(),
            files: vec![
                "Documents".to_string(),
                "Downloads".to_string(),
                "Pictures".to_string(),
                "Videos".to_string(),
                "Music".to_string(),
            ],
        }
    }
}

impl Render for FileManagerApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, button::{Button, ButtonVariants as _}, Icon, IconName};

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Toolbar
                h_flex()
                    .h(px(40.0))
                    .items_center()
                    .px_4()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .gap_2()
                    .child(Button::new("back").ghost().child(Icon::new(IconName::ArrowLeft).size_4()))
                    .child(Button::new("forward").ghost().child(Icon::new(IconName::ArrowRight).size_4()))
                    .child(
                        div()
                            .flex_1()
                            .px_3()
                            .py_1()
                            .bg(cx.theme().input)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(self.current_path.clone())
                    )
            )
            .child(
                // File list
                div()
                    .flex_1()
                    .p_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .children(self.files.iter().map(|file| {
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
                                    .child(Icon::new(IconName::Folder).size_5().text_color(cx.theme().primary))
                                    .child(file.clone())
                            }))
                    )
            )
    }
}

/// Web Browser Application
pub struct WebBrowserApp {
    url: String,
    title: String,
}

impl WebBrowserApp {
    pub fn new() -> Self {
        Self {
            url: "https://prismaui.dev".to_string(),
            title: "Welcome to PrismaUI".to_string(),
        }
    }
}

impl Render for WebBrowserApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, button::{Button, ButtonVariants as _}, Icon, IconName};

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Browser toolbar
                h_flex()
                    .h(px(50.0))
                    .items_center()
                    .px_4()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .gap_2()
                    .child(Button::new("back").ghost().child(Icon::new(IconName::ArrowLeft).size_4()))
                    .child(Button::new("forward").ghost().child(Icon::new(IconName::ArrowRight).size_4()))
                    .child(Button::new("refresh").ghost().label("⟳"))
                    .child(
                        div()
                            .flex_1()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().input)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_full()
                            .child(self.url.clone())
                    )
                    .child(Button::new("menu").ghost().child(Icon::new(IconName::Menu).size_4()))
            )
            .child(
                // Page content
                div()
                    .flex_1()
                    .p_8()
                    .bg(gpui::white())
                    .text_color(gpui::black())
                    .child(
                        v_flex()
                            .gap_4()
                            .child(
                                div()
                                    .text_3xl()
                                    .font_bold()
                                    .child(self.title.clone())
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .child("This is a simulated web browser showing content for PrismaUI.")
                            )
                    )
            )
    }
}

/// Calculator Application
pub struct CalculatorApp {
    display: String,
    last_operation: Option<String>,
}

impl CalculatorApp {
    pub fn new() -> Self {
        Self {
            display: "0".to_string(),
            last_operation: None,
        }
    }
}

impl Render for CalculatorApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, button::{Button, ButtonVariants as _}};

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .p_4()
            .gap_4()
            .child(
                // Display
                div()
                    .h(px(80.0))
                    .w_full()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_4()
                    .child(
                        div()
                            .text_2xl()
                            .font_family("Monaco")
                            .text_color(cx.theme().foreground)
                            .child(self.display.clone())
                    )
            )
            .child(
                // Button grid
                v_flex()
                    .gap_2()
                    .children((0..4).map(|row| {
                        h_flex()
                            .gap_2()
                            .children((0..4).map(|col| {
                                let button_text = match (row, col) {
                                    (0, 0) => "C", (0, 1) => "±", (0, 2) => "%", (0, 3) => "÷",
                                    (1, 0) => "7", (1, 1) => "8", (1, 2) => "9", (1, 3) => "×",
                                    (2, 0) => "4", (2, 1) => "5", (2, 2) => "6", (2, 3) => "−",
                                    (3, 0) => "1", (3, 1) => "2", (3, 2) => "3", (3, 3) => "+",
                                    _ => "0"
                                };

                                Button::new(("calc", row as usize * 4 + col as usize))
                                    .size(px(60.0))
                                    .ghost()
                                    .child(button_text)
                            }))
                    }))
            )
    }
}

/// Settings Application
pub struct SettingsApp {
    active_section: String,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self {
            active_section: "General".to_string(),
        }
    }
}

impl Render for SettingsApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, button::{Button, ButtonVariants as _}, Icon, IconName, Selectable as _};

        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Settings sidebar
                v_flex()
                    .w(px(200.0))
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .p_3()
                    .gap_2()
                    .children(["General", "Display", "Audio", "Network", "Privacy", "Updates"].iter().enumerate().map(|(idx, section)| {
                        let is_active = section == &self.active_section;
                        Button::new(("setting", idx))
                            .w_full()
                            .ghost()
                            .justify_start()
                            .when(is_active, |btn| btn.selected(true))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .child(Icon::new(IconName::Settings).size_4())
                                    .child(section.to_string())
                            )
                    }))
            )
            .child(
                // Settings content
                v_flex()
                    .flex_1()
                    .p_6()
                    .gap_4()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child(self.active_section.clone())
                    )
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Configure {} settings for your system.", self.active_section.to_lowercase()))
                    )
            )
    }
}