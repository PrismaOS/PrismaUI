/// Authentic macOS menu bar with translucent background and proper styling
use crate::{
    geometry::{Rect, Size, Color, Transform},
    renderer::RenderCommand,
    ui::{UIElement, Layout, LayoutConstraints, EdgeInsets},
    events::{Event, EventHandler, InputEvent, ButtonState, MouseButton},
};

/// macOS menu bar items
#[derive(Debug, Clone)]
pub struct MenuBarItem {
    pub text: String,
    pub highlighted: bool,
    pub bounds: Rect,
}

impl MenuBarItem {
    pub fn new(text: String) -> Self {
        Self {
            text,
            highlighted: false,
            bounds: Rect::ZERO,
        }
    }
}

/// Authentic macOS menu bar
pub struct MenuBar {
    id: String,
    layout: Layout,
    items: Vec<MenuBarItem>,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
    hovered_item: Option<usize>,
}

impl MenuBar {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            items: Vec::new(),
            visible: true,
            needs_layout: true,
            needs_visual: true,
            hovered_item: None,
        }
    }

    /// Add default macOS menu items
    pub fn add_default_items(&mut self) {
        let default_items = [
            "",  // Apple logo (will be rendered as symbol)
            "Finder",
            "File",
            "Edit",
            "View",
            "Go",
            "Window",
            "Help",
        ];

        for item_text in default_items {
            self.items.push(MenuBarItem::new(item_text.to_string()));
        }

        self.needs_layout = true;
        self.needs_visual = true;
    }

    /// Set hover state for menu item
    pub fn set_hovered_item(&mut self, item_index: Option<usize>) {
        if self.hovered_item != item_index {
            self.hovered_item = item_index;
            self.needs_visual = true;
        }
    }

    /// Get current time for clock display
    fn get_current_time() -> String {
        // In a real implementation, would get actual system time
        "Sat 14:30".to_string()
    }

    /// Render the authentic macOS menu bar background
    fn render_background(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let bounds = self.layout.bounds;

        // macOS menu bar has a specific translucent background
        commands.push(RenderCommand::Rectangle {
            rect: bounds,
            color: Color::new(0.94, 0.94, 0.96, 0.85), // Light translucent for macOS
            transform: Transform::identity(),
            z_index,
        });

        // Subtle bottom border like real macOS
        let border_rect = Rect::new(
            bounds.origin.x,
            bounds.origin.y + bounds.size.height - 1.0,
            bounds.size.width,
            1.0,
        );

        commands.push(RenderCommand::Rectangle {
            rect: border_rect,
            color: Color::new(0.8, 0.8, 0.82, 0.6),
            transform: Transform::identity(),
            z_index: z_index + 0.1,
        });

        commands
    }

    /// Render menu items
    fn render_menu_items(&self, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let item_height = self.layout.bounds.size.height;
        let mut x_offset = 16.0; // Left padding like macOS

        for (i, item) in self.items.iter().enumerate() {
            let item_width = if item.text.is_empty() {
                24.0 // Apple logo width
            } else {
                item.text.len() as f32 * 8.0 + 16.0 // Rough width calculation
            };

            let item_bounds = Rect::new(
                self.layout.bounds.origin.x + x_offset,
                self.layout.bounds.origin.y,
                item_width,
                item_height,
            );

            // Highlight background for hovered item
            if Some(i) == self.hovered_item {
                commands.push(RenderCommand::RoundedRectangle {
                    rect: Rect::new(
                        item_bounds.origin.x + 2.0,
                        item_bounds.origin.y + 2.0,
                        item_bounds.size.width - 4.0,
                        item_bounds.size.height - 4.0,
                    ),
                    corner_radius: 4.0,
                    color: Color::new(0.26, 0.59, 0.96, 0.8), // macOS blue highlight
                    transform: Transform::identity(),
                    z_index: z_index + 0.2,
                });
            }

            // Render menu item text (in real implementation would use proper text rendering)
            if item.text.is_empty() {
                // Apple logo as rounded rectangle for now
                let logo_size = 16.0;
                let logo_rect = Rect::new(
                    item_bounds.origin.x + (item_bounds.size.width - logo_size) / 2.0,
                    item_bounds.origin.y + (item_bounds.size.height - logo_size) / 2.0,
                    logo_size,
                    logo_size,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: logo_rect,
                    corner_radius: 4.0,
                    color: Color::new(0.1, 0.1, 0.1, 0.9), // Dark for Apple logo
                    transform: Transform::identity(),
                    z_index: z_index + 0.3,
                });
            } else {
                // Text placeholder (would be actual text in real implementation)
                let text_rect = Rect::new(
                    item_bounds.origin.x + 8.0,
                    item_bounds.origin.y + 8.0,
                    item_bounds.size.width - 16.0,
                    item_bounds.size.height - 16.0,
                );

                commands.push(RenderCommand::Rectangle {
                    rect: text_rect,
                    color: Color::new(0.1, 0.1, 0.1, 0.9), // Dark text
                    transform: Transform::identity(),
                    z_index: z_index + 0.3,
                });
            }

            x_offset += item_width;
        }

        // Right side items (time, battery, etc.)
        let time_text = Self::get_current_time();
        let time_width = time_text.len() as f32 * 8.0 + 16.0;
        let time_x = self.layout.bounds.size.width - time_width - 16.0;

        let time_rect = Rect::new(
            self.layout.bounds.origin.x + time_x,
            self.layout.bounds.origin.y + 8.0,
            time_width,
            item_height - 16.0,
        );

        commands.push(RenderCommand::Rectangle {
            rect: time_rect,
            color: Color::new(0.1, 0.1, 0.1, 0.9),
            transform: Transform::identity(),
            z_index: z_index + 0.3,
        });

        commands
    }
}

impl UIElement for MenuBar {
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
        Size::new(available_size.width, 24.0) // macOS menu bar is exactly 24px high
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

        // Render background
        commands.extend(self.render_background(z_index));

        // Render menu items
        commands.extend(self.render_menu_items(z_index + 0.1));

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

impl EventHandler for MenuBar {
    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Input(InputEvent::MouseMove { position, .. }) => {
                // Check which menu item is being hovered
                let mut hover_index = None;
                let mut x_offset = 16.0;

                for (i, item) in self.items.iter().enumerate() {
                    let item_width = if item.text.is_empty() {
                        24.0
                    } else {
                        item.text.len() as f32 * 8.0 + 16.0
                    };

                    let item_bounds = Rect::new(
                        self.layout.bounds.origin.x + x_offset,
                        self.layout.bounds.origin.y,
                        item_width,
                        self.layout.bounds.size.height,
                    );

                    if item_bounds.contains_point(*position) {
                        hover_index = Some(i);
                        break;
                    }

                    x_offset += item_width;
                }

                self.set_hovered_item(hover_index);
                hover_index.is_some()
            }
            Event::Input(InputEvent::MouseButton {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                position,
                ..
            }) => {
                // Handle menu clicks
                self.layout.bounds.contains_point(*position)
            }
            _ => false,
        }
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}