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

    /// Render the dock app icon with authentic macOS styling
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

        // Authentic macOS shadow with multiple layers for depth
        let shadow_blur = 3.0 * total_scale;
        let shadow_offset_y = 2.5 * total_scale;

        // Main shadow
        let shadow_rect = Rect::new(
            icon_rect.origin.x + 1.0,
            icon_rect.origin.y + shadow_offset_y,
            icon_rect.size.width,
            icon_rect.size.height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: shadow_rect,
            corner_radius: scaled_size * 0.225, // Perfect macOS radius
            color: Color::new(0.0, 0.0, 0.0, 0.25 * total_scale),
            transform: Transform::identity(),
            z_index: z_index - 0.3,
        });

        // Softer outer shadow
        let outer_shadow_rect = Rect::new(
            icon_rect.origin.x - 1.0,
            icon_rect.origin.y + shadow_offset_y - 1.0,
            icon_rect.size.width + 2.0,
            icon_rect.size.height + 2.0,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: outer_shadow_rect,
            corner_radius: scaled_size * 0.235,
            color: Color::new(0.0, 0.0, 0.0, 0.1 * total_scale),
            transform: Transform::identity(),
            z_index: z_index - 0.4,
        });

        // Main icon background with perfect macOS corner radius
        let corner_radius = scaled_size * 0.225; // Authentic macOS radius

        if let Some(texture_id) = self.texture_id {
            // Render textured icon with rounded corners
            commands.push(RenderCommand::TexturedRectangle {
                rect: icon_rect,
                texture_id,
                uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                color: Color::WHITE,
                transform: Transform::identity(),
                z_index,
            });
        } else {
            // Render built-in app icon with authentic macOS styling
            let app_color = self.get_app_color();

            // Main icon background
            commands.push(RenderCommand::RoundedRectangle {
                rect: icon_rect,
                corner_radius,
                color: app_color,
                transform: Transform::identity(),
                z_index,
            });

            // Icon symbol (simplified for now)
            let symbol_size = scaled_size * 0.45;
            let symbol_rect = Rect::new(
                icon_rect.origin.x + (scaled_size - symbol_size) / 2.0,
                icon_rect.origin.y + (scaled_size - symbol_size) / 2.0,
                symbol_size,
                symbol_size,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: symbol_rect,
                corner_radius: symbol_size * 0.15,
                color: Color::new(1.0, 1.0, 1.0, 0.95),
                transform: Transform::identity(),
                z_index: z_index + 0.1,
            });
        }

        // Authentic macOS glassy highlight
        let highlight_height = scaled_size * 0.35;
        let highlight_rect = Rect::new(
            icon_rect.origin.x + scaled_size * 0.05,
            icon_rect.origin.y + scaled_size * 0.05,
            scaled_size * 0.9,
            highlight_height,
        );

        // Top highlight gradient effect
        commands.push(RenderCommand::GradientRectangle {
            rect: highlight_rect,
            start_color: Color::new(1.0, 1.0, 1.0, 0.4),
            end_color: Color::new(1.0, 1.0, 1.0, 0.0),
            direction: std::f32::consts::PI / 2.0, // Vertical gradient
            transform: Transform::identity(),
            z_index: z_index + 0.2,
        });

        // Authentic macOS running indicator
        if self.running {
            let dot_size = 5.0; // Slightly larger like real macOS
            let dot_rect = Rect::new(
                bounds.origin.x + (icon_size - dot_size) / 2.0,
                bounds.origin.y + icon_size + 8.0, // Perfect positioning
                dot_size,
                dot_size,
            );

            // Dot shadow for depth
            let dot_shadow_rect = Rect::new(
                dot_rect.origin.x + 0.5,
                dot_rect.origin.y + 1.0,
                dot_size,
                dot_size,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: dot_shadow_rect,
                corner_radius: dot_size / 2.0,
                color: Color::new(0.0, 0.0, 0.0, 0.3),
                transform: Transform::identity(),
                z_index: z_index + 0.25,
            });

            // Main dot
            commands.push(RenderCommand::RoundedRectangle {
                rect: dot_rect,
                corner_radius: dot_size / 2.0,
                color: if self.active {
                    Color::new(0.95, 0.95, 0.95, 0.95) // Bright white when active
                } else {
                    Color::new(0.7, 0.7, 0.7, 0.9) // Subtle gray when just running
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

    /// Update magnification effect with authentic macOS curve
    fn update_magnification(&mut self) {
        if let Some(hover_index) = self.hover_app_index {
            for (i, app) in self.apps.iter_mut().enumerate() {
                let distance = (i as f32 - hover_index as f32).abs();

                // Authentic macOS magnification curve with smooth falloff
                app.hover_scale = if distance == 0.0 {
                    1.6 // 60% larger when directly hovered (authentic macOS)
                } else if distance <= 1.0 {
                    1.4 // 40% larger for adjacent icons
                } else if distance <= 2.0 {
                    1.2 // 20% larger for next adjacent
                } else if distance <= 3.0 {
                    1.1 // 10% larger for distant neighbors
                } else {
                    1.0 // Normal size for far icons
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

    /// Render the authentic macOS dock background
    fn render_dock_background(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let bounds = self.layout.bounds;

        // Perfect macOS dock background dimensions
        let dock_bg_rect = Rect::new(
            bounds.origin.x - 12.0,
            bounds.origin.y - 8.0,
            bounds.size.width + 24.0,
            bounds.size.height + 16.0,
        );

        // Authentic macOS dock shadow with multiple layers
        let shadow_offset_y = 3.0;
        let shadow_blur = 8.0;

        // Deep shadow
        let deep_shadow_rect = Rect::new(
            dock_bg_rect.origin.x + 1.0,
            dock_bg_rect.origin.y + shadow_offset_y + 2.0,
            dock_bg_rect.size.width,
            dock_bg_rect.size.height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: deep_shadow_rect,
            corner_radius: 16.0,
            color: Color::new(0.0, 0.0, 0.0, 0.35),
            transform: Transform::identity(),
            z_index: z_index - 0.4,
        });

        // Medium shadow
        let mid_shadow_rect = Rect::new(
            dock_bg_rect.origin.x + 0.5,
            dock_bg_rect.origin.y + shadow_offset_y,
            dock_bg_rect.size.width,
            dock_bg_rect.size.height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: mid_shadow_rect,
            corner_radius: 16.0,
            color: Color::new(0.0, 0.0, 0.0, 0.2),
            transform: Transform::identity(),
            z_index: z_index - 0.3,
        });

        // Soft outer shadow
        let soft_shadow_rect = Rect::new(
            dock_bg_rect.origin.x - 2.0,
            dock_bg_rect.origin.y + shadow_offset_y - 2.0,
            dock_bg_rect.size.width + 4.0,
            dock_bg_rect.size.height + 4.0,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: soft_shadow_rect,
            corner_radius: 18.0,
            color: Color::new(0.0, 0.0, 0.0, 0.08),
            transform: Transform::identity(),
            z_index: z_index - 0.5,
        });

        // Main dock background - authentic macOS translucent glass
        commands.push(RenderCommand::RoundedRectangle {
            rect: dock_bg_rect,
            corner_radius: 16.0, // Perfect macOS radius
            color: Color::new(0.12, 0.12, 0.15, 0.92), // Authentic macOS dock color
            transform: Transform::identity(),
            z_index: z_index - 0.1,
        });

        // Dock inner highlight for glass effect
        let inner_highlight_rect = Rect::new(
            dock_bg_rect.origin.x + 1.0,
            dock_bg_rect.origin.y + 1.0,
            dock_bg_rect.size.width - 2.0,
            dock_bg_rect.size.height - 2.0,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: inner_highlight_rect,
            corner_radius: 15.0,
            color: Color::new(1.0, 1.0, 1.0, 0.12),
            transform: Transform::identity(),
            z_index: z_index - 0.05,
        });

        // Top glass highlight - essential for macOS authenticity
        let top_highlight_rect = Rect::new(
            dock_bg_rect.origin.x + 2.0,
            dock_bg_rect.origin.y + 1.0,
            dock_bg_rect.size.width - 4.0,
            8.0, // Height of top highlight
        );

        commands.push(RenderCommand::GradientRectangle {
            rect: top_highlight_rect,
            start_color: Color::new(1.0, 1.0, 1.0, 0.25),
            end_color: Color::new(1.0, 1.0, 1.0, 0.0),
            direction: std::f32::consts::PI / 2.0, // Vertical gradient
            transform: Transform::identity(),
            z_index,
        });

        // Subtle bottom inner shadow
        let bottom_shadow_rect = Rect::new(
            dock_bg_rect.origin.x + 2.0,
            dock_bg_rect.origin.y + dock_bg_rect.size.height - 4.0,
            dock_bg_rect.size.width - 4.0,
            3.0,
        );

        commands.push(RenderCommand::GradientRectangle {
            rect: bottom_shadow_rect,
            start_color: Color::new(0.0, 0.0, 0.0, 0.0),
            end_color: Color::new(0.0, 0.0, 0.0, 0.1),
            direction: std::f32::consts::PI / 2.0, // Vertical gradient
            transform: Transform::identity(),
            z_index: z_index + 0.01,
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