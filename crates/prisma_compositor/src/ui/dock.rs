/// Beautiful macOS-style dock with glassy effects and app launching
use crate::{
    geometry::{Rect, Size, Color, Transform, Point},
    renderer::{RenderCommand, RenderLayer},
    ui::{UIElement, Layout, LayoutConstraints, EdgeInsets},
    events::{Event, EventHandler, InputEvent, ButtonState, MouseButton},
};

/// Dock events for app launching and management
#[derive(Debug, Clone)]
pub enum DockEvent {
    AppLaunched(String),
    AppClicked(String),
    AppRightClicked(String),
}

/// Individual dock app icon
pub struct DockApp {
    pub id: String,
    pub name: String,
    pub icon_path: Option<String>,
    pub texture_id: Option<u32>,
    pub running: bool,
    pub active: bool,
    pub hover_scale: f32,
    pub click_scale: f32,
}

impl DockApp {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            icon_path: None,
            texture_id: None,
            running: false,
            active: false,
            hover_scale: 1.0,
            click_scale: 1.0,
        }
    }

    /// Get app-specific color for built-in apps
    fn get_app_color(&self) -> Color {
        match self.id.as_str() {
            "finder" => Color::from_hex("#4A90E2").unwrap_or(Color::BLUE),
            "terminal" => Color::from_hex("#2D3748").unwrap_or(Color::new(0.18, 0.22, 0.28, 1.0)),
            "code" => Color::from_hex("#007ACC").unwrap_or(Color::BLUE),
            "browser" => Color::from_hex("#FF6B6B").unwrap_or(Color::RED),
            "calculator" => Color::from_hex("#48BB78").unwrap_or(Color::GREEN),
            "settings" => Color::from_hex("#718096").unwrap_or(Color::new(0.44, 0.5, 0.59, 1.0)),
            "music" => Color::from_hex("#E53E3E").unwrap_or(Color::RED),
            "photos" => Color::from_hex("#FFD700").unwrap_or(Color::new(1.0, 0.84, 0.0, 1.0)),
            _ => Color::from_hex("#667EEA").unwrap_or(Color::BLUE),
        }
    }

    /// Render the dock app icon with beautiful styling
    pub fn render(&self, bounds: Rect, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let icon_size = bounds.size.width;

        // Apply scaling for hover and click animations
        let total_scale = self.hover_scale * self.click_scale;
        let scaled_size = icon_size * total_scale;
        let offset = (icon_size - scaled_size) / 2.0;

        let icon_rect = Rect::new(
            bounds.origin.x + offset,
            bounds.origin.y + offset,
            scaled_size,
            scaled_size,
        );

        // Beautiful shadow for depth
        let shadow_offset = 2.0 * total_scale;
        let shadow_rect = Rect::new(
            icon_rect.origin.x + shadow_offset,
            icon_rect.origin.y + shadow_offset,
            icon_rect.size.width,
            icon_rect.size.height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: shadow_rect,
            corner_radius: scaled_size * 0.2,
            color: Color::new(0.0, 0.0, 0.0, 0.3 * total_scale),
            transform: Transform::identity(),
            z_index: z_index - 0.1,
        });

        // Main icon background
        let corner_radius = scaled_size * 0.22; // 22% for modern macOS look

        if let Some(texture_id) = self.texture_id {
            // Render textured icon
            commands.push(RenderCommand::TexturedRectangle {
                rect: icon_rect,
                texture_id,
                uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                color: Color::WHITE,
                transform: Transform::identity(),
                z_index,
            });
        } else {
            // Render built-in app icon with beautiful gradient
            let app_color = self.get_app_color();
            let lighter_color = Color::new(
                (app_color.r + 0.3).min(1.0),
                (app_color.g + 0.3).min(1.0),
                (app_color.b + 0.3).min(1.0),
                app_color.a,
            );

            // Gradient background
            commands.push(RenderCommand::GradientRectangle {
                rect: icon_rect,
                start_color: lighter_color,
                end_color: app_color,
                direction: std::f32::consts::PI * 0.75, // Diagonal
                transform: Transform::identity(),
                z_index,
            });

            // Icon symbol area
            let symbol_rect = Rect::new(
                icon_rect.origin.x + scaled_size * 0.25,
                icon_rect.origin.y + scaled_size * 0.25,
                scaled_size * 0.5,
                scaled_size * 0.5,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: symbol_rect,
                corner_radius: corner_radius * 0.5,
                color: Color::new(1.0, 1.0, 1.0, 0.9),
                transform: Transform::identity(),
                z_index: z_index + 0.1,
            });
        }

        // Glassy highlight for 3D effect
        let highlight_rect = Rect::new(
            icon_rect.origin.x + scaled_size * 0.1,
            icon_rect.origin.y + scaled_size * 0.1,
            scaled_size * 0.8,
            scaled_size * 0.3,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: highlight_rect,
            corner_radius: corner_radius * 0.8,
            color: Color::new(1.0, 1.0, 1.0, 0.3),
            transform: Transform::identity(),
            z_index: z_index + 0.2,
        });

        // Running indicator (small dot)
        if self.running {
            let dot_size = 4.0;
            let dot_rect = Rect::new(
                bounds.origin.x + (icon_size - dot_size) / 2.0,
                bounds.origin.y + icon_size + 6.0,
                dot_size,
                dot_size,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: dot_rect,
                corner_radius: dot_size / 2.0,
                color: if self.active {
                    Color::new(1.0, 1.0, 1.0, 0.9) // White when active
                } else {
                    Color::new(0.6, 0.6, 0.6, 0.8) // Gray when just running
                },
                transform: Transform::identity(),
                z_index: z_index + 0.3,
            });
        }

        commands
    }
}

/// Beautiful macOS-style dock
pub struct Dock {
    id: String,
    layout: Layout,
    apps: Vec<DockApp>,
    background_color: Color,
    hover_app_index: Option<usize>,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
    // Animation state
    magnification: f32,
    dock_scale: f32,
}

impl Dock {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            apps: Vec::new(),
            background_color: Color::new(0.1, 0.1, 0.1, 0.8), // Dark translucent
            hover_app_index: None,
            visible: true,
            needs_layout: true,
            needs_visual: true,
            magnification: 1.0,
            dock_scale: 1.0,
        }
    }

    /// Add default macOS-style apps
    pub fn add_default_apps(&mut self) {
        let default_apps = [
            ("finder", "Finder"),
            ("terminal", "Terminal"),
            ("code", "Code Editor"),
            ("browser", "Safari"),
            ("music", "Music"),
            ("photos", "Photos"),
            ("calculator", "Calculator"),
            ("settings", "System Preferences"),
        ];

        for (id, name) in default_apps {
            let mut app = DockApp::new(id.to_string(), name.to_string());
            if id == "finder" {
                app.running = true;
                app.active = true;
            }
            self.apps.push(app);
        }

        self.needs_layout = true;
        self.needs_visual = true;
    }

    /// Set hover state for magnification effect
    pub fn set_hover_app(&mut self, app_index: Option<usize>) {
        if self.hover_app_index != app_index {
            self.hover_app_index = app_index;
            self.update_magnification();
            self.needs_visual = true;
        }
    }

    /// Update magnification effect based on hover
    fn update_magnification(&mut self) {
        if let Some(hover_index) = self.hover_app_index {
            for (i, app) in self.apps.iter_mut().enumerate() {
                let distance = (i as f32 - hover_index as f32).abs();

                // macOS-style magnification curve
                app.hover_scale = if distance == 0.0 {
                    1.5 // 50% larger when directly hovered
                } else if distance == 1.0 {
                    1.3 // 30% larger for adjacent icons
                } else if distance == 2.0 {
                    1.15 // 15% larger for next icons
                } else {
                    1.0 // Normal size for distant icons
                };
            }
        } else {
            // Reset all to normal size
            for app in &mut self.apps {
                app.hover_scale = 1.0;
            }
        }
    }

    /// Launch an app
    pub fn launch_app(&mut self, app_id: &str) -> Option<DockEvent> {
        if let Some(app) = self.apps.iter_mut().find(|a| a.id == app_id) {
            app.running = true;
            app.active = true;

            // Make other apps inactive
            for other_app in &mut self.apps {
                if other_app.id != app_id {
                    other_app.active = false;
                }
            }

            self.needs_visual = true;
            Some(DockEvent::AppLaunched(app_id.to_string()))
        } else {
            None
        }
    }

    /// Render the beautiful dock background
    fn render_dock_background(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let bounds = self.layout.bounds;

        // Beautiful rounded dock background with transparency
        let dock_bg_rect = Rect::new(
            bounds.origin.x - 16.0,
            bounds.origin.y - 12.0,
            bounds.size.width + 32.0,
            bounds.size.height + 24.0,
        );

        // Dock shadow for depth
        let shadow_rect = Rect::new(
            dock_bg_rect.origin.x + 2.0,
            dock_bg_rect.origin.y + 4.0,
            dock_bg_rect.size.width,
            dock_bg_rect.size.height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: shadow_rect,
            corner_radius: 20.0,
            color: Color::new(0.0, 0.0, 0.0, 0.4),
            transform: Transform::identity(),
            z_index: z_index - 0.2,
        });

        // Main dock background with glass effect
        commands.push(RenderCommand::RoundedRectangle {
            rect: dock_bg_rect,
            corner_radius: 18.0,
            color: Color::new(0.08, 0.08, 0.12, 0.85), // Dark translucent
            transform: Transform::identity(),
            z_index: z_index - 0.1,
        });

        // Glass highlight on top edge
        let highlight_rect = Rect::new(
            dock_bg_rect.origin.x + 2.0,
            dock_bg_rect.origin.y + 2.0,
            dock_bg_rect.size.width - 4.0,
            3.0,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: highlight_rect,
            corner_radius: 15.0,
            color: Color::new(1.0, 1.0, 1.0, 0.1),
            transform: Transform::identity(),
            z_index,
        });

        // Subtle border
        let border_rect = Rect::new(
            dock_bg_rect.origin.x - 0.5,
            dock_bg_rect.origin.y - 0.5,
            dock_bg_rect.size.width + 1.0,
            dock_bg_rect.size.height + 1.0,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: border_rect,
            corner_radius: 18.5,
            color: Color::new(1.0, 1.0, 1.0, 0.15),
            transform: Transform::identity(),
            z_index: z_index - 0.15,
        });

        commands
    }
}

impl UIElement for Dock {
    fn id(&self) -> &str {
        &self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn measure(&mut self, available_size: Size) -> Size {
        let icon_size = 56.0; // Base icon size
        let spacing = 8.0;
        let padding = 16.0;

        let total_width = (self.apps.len() as f32 * (icon_size + spacing)) - spacing + (padding * 2.0);
        let height = icon_size + padding + 16.0; // Extra space for running indicators

        Size::new(total_width.min(available_size.width), height)
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;
        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        let mut commands = Vec::new();

        // Render dock background
        commands.extend(self.render_dock_background(z_index));

        // Render apps
        let icon_size = 56.0;
        let spacing = 8.0;
        let padding = 16.0;

        for (i, app) in self.apps.iter().enumerate() {
            let x = self.layout.bounds.origin.x + padding + (i as f32 * (icon_size + spacing));
            let y = self.layout.bounds.origin.y + 8.0;

            let app_bounds = Rect::new(x, y, icon_size, icon_size);
            commands.extend(app.render(app_bounds, z_index + 0.1 + (i as f32 * 0.01)));
        }

        commands
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.needs_visual = true;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate_layout(&mut self) {
        self.needs_layout = true;
    }

    fn invalidate_visual(&mut self) {
        self.needs_visual = true;
    }
}

impl EventHandler for Dock {
    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Input(InputEvent::MouseMove { position, .. }) => {
                // Check which app is being hovered
                let icon_size = 56.0;
                let spacing = 8.0;
                let padding = 16.0;

                let mut hover_index = None;

                for (i, _app) in self.apps.iter().enumerate() {
                    let x = self.layout.bounds.origin.x + padding + (i as f32 * (icon_size + spacing));
                    let y = self.layout.bounds.origin.y + 8.0;
                    let app_bounds = Rect::new(x, y, icon_size, icon_size);

                    if app_bounds.contains_point(*position) {
                        hover_index = Some(i);
                        break;
                    }
                }

                self.set_hover_app(hover_index);
                hover_index.is_some()
            }
            Event::Input(InputEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                position,
                ..
            }) => {
                // Check for app clicks
                let icon_size = 56.0;
                let spacing = 8.0;
                let padding = 16.0;

                for (i, app) in self.apps.iter().enumerate() {
                    let x = self.layout.bounds.origin.x + padding + (i as f32 * (icon_size + spacing));
                    let y = self.layout.bounds.origin.y + 8.0;
                    let app_bounds = Rect::new(x, y, icon_size, icon_size);

                    if app_bounds.contains_point(*position) {
                        // Launch the app - need to clone the id to avoid borrowing issues
                        let app_id = app.id.clone();
                        self.launch_app(&app_id);

                        // Queue the app to be launched by the compositor
                        crate::compositor::queue_app_launch(app_id.clone());

                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}