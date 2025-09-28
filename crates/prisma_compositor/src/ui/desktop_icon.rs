/// Beautiful desktop icons with macOS-style design and animations
use crate::{
    geometry::{Rect, Size, Color, Transform, Point},
    renderer::{RenderCommand, RenderLayer},
    ui::{UIElement, Layout, LayoutConstraints, EdgeInsets},
    events::{Event, EventHandler, InputEvent, ButtonState, MouseButton},
    text::{TextRenderer, FontProperties, TextLayout},
};

/// Icon size variants for different use cases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconSize {
    Small,   // 48x48
    Medium,  // 64x64
    Large,   // 80x80
}

impl IconSize {
    pub fn pixels(&self) -> f32 {
        match self {
            IconSize::Small => 48.0,
            IconSize::Medium => 64.0,
            IconSize::Large => 80.0,
        }
    }
}

/// Desktop icon events
#[derive(Debug, Clone)]
pub enum IconEvent {
    Clicked(String),
    DoubleClicked(String),
    RightClicked(String),
    DragStarted(String, Point),
}

/// Beautiful desktop icon with GPU-accelerated rendering
pub struct DesktopIcon {
    id: String,
    layout: Layout,
    name: String,
    icon_path: Option<String>,
    texture_id: Option<u32>,
    size: IconSize,
    selected: bool,
    hovered: bool,
    pressed: bool,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
    // Animation state
    hover_animation_progress: f32,
    click_animation_progress: f32,
    last_click_time: std::time::Instant,
}

impl DesktopIcon {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            name,
            icon_path: None,
            texture_id: None,
            size: IconSize::Medium,
            selected: false,
            hovered: false,
            pressed: false,
            visible: true,
            needs_layout: true,
            needs_visual: true,
            hover_animation_progress: 0.0,
            click_animation_progress: 0.0,
            last_click_time: std::time::Instant::now(),
        }
    }

    /// Set icon image
    pub fn set_icon(&mut self, path: Option<String>, texture_id: Option<u32>) {
        self.icon_path = path;
        self.texture_id = texture_id;
        self.needs_visual = true;
    }

    /// Set icon size
    pub fn set_size(&mut self, size: IconSize) {
        self.size = size;
        self.needs_layout = true;
        self.needs_visual = true;
    }

    /// Set selected state
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.needs_visual = true;
    }

    /// Set hover state
    pub fn set_hovered(&mut self, hovered: bool) {
        if self.hovered != hovered {
            self.hovered = hovered;
            self.needs_visual = true;
        }
    }

    /// Get the app-specific color for built-in icons
    fn get_app_color(&self) -> Color {
        match self.id.as_str() {
            "terminal" => Color::from_hex("#2D3748").unwrap_or(Color::new(0.18, 0.22, 0.28, 1.0)),
            "code_editor" => Color::from_hex("#0066CC").unwrap_or(Color::BLUE),
            "file_manager" => Color::from_hex("#4A90E2").unwrap_or(Color::BLUE),
            "web_browser" => Color::from_hex("#FF6B6B").unwrap_or(Color::RED),
            "calculator" => Color::from_hex("#48BB78").unwrap_or(Color::GREEN),
            "settings" => Color::from_hex("#718096").unwrap_or(Color::new(0.44, 0.5, 0.59, 1.0)),
            _ => Color::from_hex("#667EEA").unwrap_or(Color::BLUE),
        }
    }

    /// Get symbol for built-in app icons
    fn get_icon_symbol(&self) -> &str {
        match self.id.as_str() {
            "terminal" => "⌘",
            "code_editor" => "⌨",
            "file_manager" => "📁",
            "web_browser" => "🌐",
            "calculator" => "🧮",
            "settings" => "⚙",
            _ => "📱",
        }
    }

    /// Render the icon image/symbol
    fn render_icon_image(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let icon_size = self.size.pixels();

        // Apply hover and click animations
        let scale = 1.0 + (self.hover_animation_progress * 0.05) + (self.click_animation_progress * 0.02);
        let scaled_size = icon_size * scale;
        let offset = (icon_size - scaled_size) / 2.0;

        let icon_rect = Rect::new(
            self.layout.bounds.origin.x + offset,
            self.layout.bounds.origin.y + offset,
            scaled_size,
            scaled_size,
        );

        // Icon background with beautiful rounded corners
        let corner_radius = scaled_size * 0.2; // 20% of size for modern look

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
            // Render built-in icon with app color
            let app_color = self.get_app_color();

            // Background
            commands.push(RenderCommand::RoundedRectangle {
                rect: icon_rect,
                corner_radius,
                color: app_color,
                transform: Transform::identity(),
                z_index,
            });

            // Symbol would be rendered as text in a real implementation
            // For now, we'll use a lighter rectangle to represent the symbol
            let symbol_rect = Rect::new(
                icon_rect.origin.x + scaled_size * 0.25,
                icon_rect.origin.y + scaled_size * 0.25,
                scaled_size * 0.5,
                scaled_size * 0.5,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: symbol_rect,
                corner_radius: corner_radius * 0.5,
                color: Color::new(1.0, 1.0, 1.0, 0.8),
                transform: Transform::identity(),
                z_index: z_index + 0.1,
            });
        }

        // Beautiful shadow for depth
        let shadow_offset = 2.0 * scale;
        let shadow_rect = Rect::new(
            icon_rect.origin.x + shadow_offset,
            icon_rect.origin.y + shadow_offset,
            icon_rect.size.width,
            icon_rect.size.height,
        );

        commands.insert(0, RenderCommand::RoundedRectangle {
            rect: shadow_rect,
            corner_radius,
            color: Color::new(0.0, 0.0, 0.0, 0.15 * scale),
            transform: Transform::identity(),
            z_index: z_index - 0.1,
        });

        // Selection background
        if self.selected {
            let selection_rect = Rect::new(
                icon_rect.origin.x - 4.0,
                icon_rect.origin.y - 4.0,
                icon_rect.size.width + 8.0,
                icon_rect.size.height + 8.0,
            );

            commands.insert(0, RenderCommand::RoundedRectangle {
                rect: selection_rect,
                corner_radius: corner_radius + 4.0,
                color: Color::from_hex("#007AFF").unwrap_or(Color::BLUE).with_alpha(0.3),
                transform: Transform::identity(),
                z_index: z_index - 0.2,
            });
        }

        commands
    }

    /// Render the icon label
    fn render_label(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let icon_size = self.size.pixels();

        // Label background for better readability
        let label_rect = Rect::new(
            self.layout.bounds.origin.x - 10.0,
            self.layout.bounds.origin.y + icon_size + 4.0,
            icon_size + 20.0,
            20.0,
        );

        // Semi-transparent background
        commands.push(RenderCommand::RoundedRectangle {
            rect: label_rect,
            corner_radius: 10.0,
            color: if self.selected {
                Color::new(0.0, 0.48, 1.0, 0.8) // Blue when selected
            } else {
                Color::new(0.0, 0.0, 0.0, 0.4) // Dark semi-transparent
            },
            transform: Transform::identity(),
            z_index,
        });

        // Text would be rendered here in a real implementation
        // For now, we'll use a white rectangle to represent text
        let text_rect = Rect::new(
            label_rect.origin.x + 4.0,
            label_rect.origin.y + 4.0,
            label_rect.size.width - 8.0,
            label_rect.size.height - 8.0,
        );

        commands.push(RenderCommand::Rectangle {
            rect: text_rect,
            color: Color::WHITE,
            transform: Transform::identity(),
            z_index: z_index + 0.1,
        });

        commands
    }

    /// Update animations
    pub fn update_animations(&mut self, delta_time: f32) {
        // Hover animation
        if self.hovered {
            self.hover_animation_progress = (self.hover_animation_progress + delta_time * 4.0).min(1.0);
        } else {
            self.hover_animation_progress = (self.hover_animation_progress - delta_time * 4.0).max(0.0);
        }

        // Click animation
        if self.pressed {
            self.click_animation_progress = (self.click_animation_progress + delta_time * 8.0).min(1.0);
        } else {
            self.click_animation_progress = (self.click_animation_progress - delta_time * 8.0).max(0.0);
        }

        if self.hover_animation_progress > 0.0 || self.click_animation_progress > 0.0 {
            self.needs_visual = true;
        }
    }
}

impl UIElement for DesktopIcon {
    fn id(&self) -> &str {
        &self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn measure(&mut self, _available_size: Size) -> Size {
        let icon_size = self.size.pixels();
        Size::new(icon_size, icon_size + 30.0) // Icon + label space
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

        // Render icon
        commands.extend(self.render_icon_image(z_index));

        // Render label
        commands.extend(self.render_label(z_index + 0.5));

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

impl EventHandler for DesktopIcon {
    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Input(InputEvent::MouseMove { position, .. }) => {
                let was_hovered = self.hovered;
                self.hovered = self.layout.bounds.contains_point(*position);

                if was_hovered != self.hovered {
                    self.needs_visual = true;
                    return true;
                }
            }
            Event::Input(InputEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                position,
                ..
            }) => {
                if self.layout.bounds.contains_point(*position) {
                    self.pressed = true;

                    // Check for double click
                    let now = std::time::Instant::now();
                    let double_click = now.duration_since(self.last_click_time).as_millis() < 500;
                    self.last_click_time = now;

                    self.needs_visual = true;
                    return true;
                }
            }
            Event::Input(InputEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Released,
                position,
                ..
            }) => {
                if self.pressed && self.layout.bounds.contains_point(*position) {
                    self.pressed = false;
                    self.needs_visual = true;
                    return true;
                }
                self.pressed = false;
            }
            Event::Input(InputEvent::MouseButton {
                button: MouseButton::Right,
                state: ButtonState::Pressed,
                position,
                ..
            }) => {
                if self.layout.bounds.contains_point(*position) {
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}

/// Desktop icon grid for organizing icons
pub struct DesktopIconGrid {
    id: String,
    layout: Layout,
    icons: Vec<Box<dyn UIElement>>,
    grid_cell_size: Size,
    visible: bool,
    needs_layout: bool,
}

impl DesktopIconGrid {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            icons: Vec::new(),
            grid_cell_size: Size::new(100.0, 120.0), // Icon + label space
            visible: true,
            needs_layout: true,
        }
    }

    /// Add an icon to the grid
    pub fn add_icon(&mut self, icon: Box<dyn UIElement>) {
        self.icons.push(icon);
        self.needs_layout = true;
    }

    /// Add default desktop icons
    pub fn add_default_icons(&mut self) {
        let default_apps = [
            ("terminal", "Terminal"),
            ("code_editor", "Code Editor"),
            ("file_manager", "Files"),
            ("web_browser", "Browser"),
            ("calculator", "Calculator"),
            ("settings", "Settings"),
        ];

        for (id, name) in default_apps {
            let mut icon = DesktopIcon::new(id.to_string(), name.to_string());
            icon.set_size(IconSize::Medium);
            self.add_icon(Box::new(icon));
        }
    }
}

impl UIElement for DesktopIconGrid {
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
        available_size // Grid fills available space
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;

        // Arrange icons in a grid
        let cols = (bounds.size.width / self.grid_cell_size.width) as usize;
        let margin_x = 40.0; // Left margin
        let margin_y = 60.0; // Top margin (below menu bar)

        for (i, icon) in self.icons.iter_mut().enumerate() {
            let col = i % cols;
            let row = i / cols;

            let x = bounds.origin.x + margin_x + (col as f32 * self.grid_cell_size.width);
            let y = bounds.origin.y + margin_y + (row as f32 * self.grid_cell_size.height);

            let icon_bounds = Rect::new(x, y, self.grid_cell_size.width, self.grid_cell_size.height);
            icon.arrange(icon_bounds);
        }

        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        let mut commands = Vec::new();

        for (i, icon) in self.icons.iter().enumerate() {
            commands.extend(icon.render(z_index + (i as f32 * 0.1)));
        }

        commands
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        for icon in &mut self.icons {
            icon.set_visible(visible);
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate_layout(&mut self) {
        self.needs_layout = true;
        for icon in &mut self.icons {
            icon.invalidate_layout();
        }
    }

    fn invalidate_visual(&mut self) {
        for icon in &mut self.icons {
            icon.invalidate_visual();
        }
    }
}

impl EventHandler for DesktopIconGrid {
    fn handle_event(&mut self, event: &Event) -> bool {
        // Forward events to icons (in reverse order for proper hit testing)
        for icon in self.icons.iter_mut().rev() {
            if icon.handle_event(event) {
                return true;
            }
        }
        false
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}