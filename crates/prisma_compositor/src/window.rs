/// Window management system for the GPU-accelerated compositor
use std::collections::HashMap;
use crate::{
    geometry::{Rect, Point, Size, Color},
    renderer::{RenderCommand, RenderLayer},
    ui::{UITree, UIElement},
    events::Event,
};

/// Unique identifier for windows
pub type WindowId = u32;

/// Window state and properties
#[derive(Debug, Clone)]
pub struct WindowState {
    pub id: WindowId,
    pub title: String,
    pub bounds: Rect,
    pub min_size: Size,
    pub max_size: Size,
    pub resizable: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
    pub visible: bool,
    pub always_on_top: bool,
    pub decorations: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            id: 0,
            title: "Untitled".to_string(),
            bounds: Rect::new(100.0, 100.0, 800.0, 600.0),
            min_size: Size::new(200.0, 150.0),
            max_size: Size::new(f32::INFINITY, f32::INFINITY),
            resizable: true,
            minimized: false,
            maximized: false,
            focused: false,
            visible: true,
            always_on_top: false,
            decorations: true,
        }
    }
}

/// Window decoration rendering (title bar, borders, etc.)
pub struct WindowDecorations {
    title_bar_height: f32,
    border_width: f32,
    corner_radius: f32,
    title_bar_color: Color,
    border_color: Color,
    focused_color: Color,
    unfocused_color: Color,
}

impl Default for WindowDecorations {
    fn default() -> Self {
        Self {
            title_bar_height: 40.0, // Perfect macOS proportions
            border_width: 0.0, // No visible border for modern look
            corner_radius: 12.0, // Authentic macOS rounded corners
            // Modern macOS dark theme colors with transparency
            title_bar_color: Color::new(0.14, 0.14, 0.16, 0.85), // Dark translucent
            border_color: Color::new(0.0, 0.0, 0.0, 0.1), // Subtle dark border
            focused_color: Color::new(0.0, 0.48, 1.0, 1.0), // macOS accent blue
            unfocused_color: Color::new(0.44, 0.44, 0.44, 1.0), // Dark gray
        }
    }
}

impl WindowDecorations {
    /// Generate render commands for beautiful macOS-style window decorations
    pub fn render(&self, state: &WindowState, z_index: f32) -> Vec<RenderCommand> {
        if !state.decorations {
            return Vec::new();
        }

        let mut commands = Vec::new();

        // Beautiful shadow layers for depth
        self.render_window_shadows(state, z_index - 0.3, &mut commands);

        // Main window background with macOS dark theme
        let window_bg_color = if state.focused {
            Color::new(0.16, 0.16, 0.18, 0.95) // Dark translucent when focused
        } else {
            Color::new(0.12, 0.12, 0.14, 0.92) // Darker when unfocused
        };

        commands.push(RenderCommand::RoundedRectangle {
            rect: state.bounds,
            corner_radius: self.corner_radius,
            color: window_bg_color,
            transform: crate::geometry::Transform::identity(),
            z_index,
        });

        // Dynamic title bar with transparency and blur effect
        let title_bar_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y,
            state.bounds.size.width,
            self.title_bar_height,
        );

        // Title bar background with beautiful gradient
        let title_bar_start = if state.focused {
            Color::new(0.22, 0.22, 0.24, 0.88) // Lighter when focused
        } else {
            Color::new(0.14, 0.14, 0.16, 0.85) // Darker when unfocused
        };

        let title_bar_end = if state.focused {
            Color::new(0.18, 0.18, 0.20, 0.85)
        } else {
            Color::new(0.11, 0.11, 0.13, 0.82)
        };

        commands.push(RenderCommand::GradientRectangle {
            rect: title_bar_rect,
            start_color: title_bar_start,
            end_color: title_bar_end,
            direction: std::f32::consts::PI / 2.0, // Top to bottom
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.2,
        });

        // Add glassy highlight on title bar
        let highlight_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y,
            state.bounds.size.width,
            2.0,
        );

        commands.push(RenderCommand::Rectangle {
            rect: highlight_rect,
            color: if state.focused {
                Color::new(1.0, 1.0, 1.0, 0.08) // Subtle white highlight
            } else {
                Color::new(1.0, 1.0, 1.0, 0.04)
            },
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.3,
        });

        // Title bar separator (very subtle)
        let separator_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y + self.title_bar_height - 0.5,
            state.bounds.size.width,
            0.5,
        );

        commands.push(RenderCommand::Rectangle {
            rect: separator_rect,
            color: Color::new(0.0, 0.0, 0.0, 0.15),
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.3,
        });

        // Beautiful traffic light buttons
        self.render_traffic_lights(state, z_index + 0.4, &mut commands);

        // Window content background with dark theme
        let content_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y + self.title_bar_height,
            state.bounds.size.width,
            state.bounds.size.height - self.title_bar_height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: content_rect,
            corner_radius: self.corner_radius,
            color: Color::new(0.11, 0.11, 0.13, 0.98), // Dark content area
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.1,
        });

        commands
    }

    /// Render beautiful layered shadows for window depth
    fn render_window_shadows(&self, state: &WindowState, z_index: f32, commands: &mut Vec<RenderCommand>) {
        if state.focused {
            // Multiple shadow layers for beautiful depth when focused
            let shadow_layers = [
                (2.0, 8.0, Color::new(0.0, 0.0, 0.0, 0.25)),   // Close shadow
                (4.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.18)),  // Medium shadow
                (8.0, 32.0, Color::new(0.0, 0.0, 0.0, 0.12)),  // Far shadow
                (16.0, 48.0, Color::new(0.0, 0.0, 0.0, 0.08)), // Ambient shadow
            ];

            for (i, (offset, blur_radius, color)) in shadow_layers.iter().enumerate() {
                let shadow_rect = Rect::new(
                    state.bounds.origin.x + offset,
                    state.bounds.origin.y + offset,
                    state.bounds.size.width,
                    state.bounds.size.height,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: shadow_rect,
                    corner_radius: self.corner_radius + blur_radius / 4.0,
                    color: *color,
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index - (i as f32 * 0.01),
                });
            }
        } else {
            // Single subtle shadow when unfocused
            let shadow_rect = Rect::new(
                state.bounds.origin.x + 2.0,
                state.bounds.origin.y + 4.0,
                state.bounds.size.width,
                state.bounds.size.height,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: shadow_rect,
                corner_radius: self.corner_radius + 2.0,
                color: Color::new(0.0, 0.0, 0.0, 0.15),
                transform: crate::geometry::Transform::identity(),
                z_index,
            });
        }
    }

    /// Render authentic macOS-style traffic light buttons
    fn render_traffic_lights(&self, state: &WindowState, z_index: f32, commands: &mut Vec<RenderCommand>) {
        let button_size = 12.0; // Slightly smaller for authenticity
        let button_spacing = 8.0;
        let margin_left = 20.0; // More spacing from edge
        let margin_top = (self.title_bar_height - button_size) / 2.0;

        let button_y = state.bounds.origin.y + margin_top;

        if state.focused {
            // Authentic macOS colors when focused
            let buttons = [
                (
                    "close",
                    Color::from_hex("#FF5F56").unwrap_or(Color::RED),
                    Color::from_hex("#E0443E").unwrap_or(Color::RED), // Darker border
                ),
                (
                    "minimize",
                    Color::from_hex("#FFBD2E").unwrap_or(Color::new(1.0, 0.74, 0.18, 1.0)),
                    Color::from_hex("#DEA123").unwrap_or(Color::new(0.87, 0.63, 0.14, 1.0)),
                ),
                (
                    "maximize",
                    Color::from_hex("#27C93F").unwrap_or(Color::GREEN),
                    Color::from_hex("#1AAD34").unwrap_or(Color::GREEN),
                ),
            ];

            for (i, (_name, bg_color, border_color)) in buttons.iter().enumerate() {
                let button_x = state.bounds.origin.x + margin_left + (i as f32 * (button_size + button_spacing));
                let button_rect = Rect::new(button_x, button_y, button_size, button_size);

                // Button shadow for depth
                let shadow_rect = Rect::new(
                    button_x + 0.5,
                    button_y + 1.0,
                    button_size,
                    button_size,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: shadow_rect,
                    corner_radius: button_size / 2.0,
                    color: Color::new(0.0, 0.0, 0.0, 0.15),
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index - 0.1,
                });

                // Button border for depth
                let border_rect = Rect::new(
                    button_x - 0.5,
                    button_y - 0.5,
                    button_size + 1.0,
                    button_size + 1.0,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: border_rect,
                    corner_radius: (button_size + 1.0) / 2.0,
                    color: *border_color,
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index - 0.05,
                });

                // Main button
                commands.push(RenderCommand::RoundedRectangle {
                    rect: button_rect,
                    corner_radius: button_size / 2.0,
                    color: *bg_color,
                    transform: crate::geometry::Transform::identity(),
                    z_index,
                });

                // Glossy highlight for 3D effect
                let highlight_rect = Rect::new(
                    button_x + 1.0,
                    button_y + 1.0,
                    button_size - 2.0,
                    (button_size - 2.0) / 2.0,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: highlight_rect,
                    corner_radius: (button_size - 2.0) / 4.0,
                    color: Color::new(1.0, 1.0, 1.0, 0.25),
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index + 0.1,
                });
            }
        } else {
            // Subtle gray buttons when unfocused
            let unfocused_color = Color::new(0.44, 0.44, 0.44, 0.8);
            let unfocused_border = Color::new(0.35, 0.35, 0.35, 0.9);

            for i in 0..3 {
                let button_x = state.bounds.origin.x + margin_left + (i as f32 * (button_size + button_spacing));
                let button_rect = Rect::new(button_x, button_y, button_size, button_size);

                // Subtle border
                let border_rect = Rect::new(
                    button_x - 0.5,
                    button_y - 0.5,
                    button_size + 1.0,
                    button_size + 1.0,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: border_rect,
                    corner_radius: (button_size + 1.0) / 2.0,
                    color: unfocused_border,
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index - 0.05,
                });

                // Main button
                commands.push(RenderCommand::RoundedRectangle {
                    rect: button_rect,
                    corner_radius: button_size / 2.0,
                    color: unfocused_color,
                    transform: crate::geometry::Transform::identity(),
                    z_index,
                });
            }
        }
    }

    /// Get content area (excluding decorations)
    pub fn content_area(&self, state: &WindowState) -> Rect {
        if !state.decorations || state.maximized {
            return state.bounds;
        }

        Rect::new(
            state.bounds.origin.x + self.border_width,
            state.bounds.origin.y + self.border_width + self.title_bar_height,
            state.bounds.size.width - 2.0 * self.border_width,
            state.bounds.size.height - 2.0 * self.border_width - self.title_bar_height,
        )
    }

    /// Check if point is in title bar (for dragging)
    pub fn hit_test_title_bar(&self, state: &WindowState, point: Point) -> bool {
        if !state.decorations || state.maximized {
            return false;
        }

        let title_bar_rect = Rect::new(
            state.bounds.origin.x + self.border_width,
            state.bounds.origin.y + self.border_width,
            state.bounds.size.width - 2.0 * self.border_width,
            self.title_bar_height,
        );

        title_bar_rect.contains_point(point)
    }

    /// Check if point is on resize border
    pub fn hit_test_resize_border(&self, state: &WindowState, point: Point) -> Option<ResizeDirection> {
        if !state.decorations || !state.resizable || state.maximized {
            return None;
        }

        let border = self.border_width * 2.0; // Expand hit area
        let bounds = state.bounds;

        // Check corners first (higher priority)
        if point.x <= bounds.origin.x + border && point.y <= bounds.origin.y + border {
            return Some(ResizeDirection::TopLeft);
        }
        if point.x >= bounds.origin.x + bounds.size.width - border && point.y <= bounds.origin.y + border {
            return Some(ResizeDirection::TopRight);
        }
        if point.x <= bounds.origin.x + border && point.y >= bounds.origin.y + bounds.size.height - border {
            return Some(ResizeDirection::BottomLeft);
        }
        if point.x >= bounds.origin.x + bounds.size.width - border && point.y >= bounds.origin.y + bounds.size.height - border {
            return Some(ResizeDirection::BottomRight);
        }

        // Check edges
        if point.x <= bounds.origin.x + border {
            return Some(ResizeDirection::Left);
        }
        if point.x >= bounds.origin.x + bounds.size.width - border {
            return Some(ResizeDirection::Right);
        }
        if point.y <= bounds.origin.y + border {
            return Some(ResizeDirection::Top);
        }
        if point.y >= bounds.origin.y + bounds.size.height - border {
            return Some(ResizeDirection::Bottom);
        }

        None
    }
}

/// Window resize directions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeDirection {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Individual window with content and state
pub struct Window {
    state: WindowState,
    content: UITree,
    decorations: WindowDecorations,
    drag_state: Option<DragState>,
    resize_state: Option<ResizeState>,
    needs_layout: bool,
    needs_render: bool,
}

#[derive(Debug, Clone)]
struct DragState {
    start_position: Point,
    start_window_position: Point,
}

#[derive(Debug, Clone)]
struct ResizeState {
    direction: ResizeDirection,
    start_position: Point,
    start_bounds: Rect,
}

impl Window {
    /// Create a new window
    pub fn new(id: WindowId, title: String) -> Self {
        let mut state = WindowState::default();
        state.id = id;
        state.title = title;

        Self {
            state,
            content: UITree::new(),
            decorations: WindowDecorations::default(),
            drag_state: None,
            resize_state: None,
            needs_layout: true,
            needs_render: true,
        }
    }

    /// Set window content
    pub fn set_content(&mut self, content: Box<dyn UIElement>) {
        self.content.set_root(content);
        self.needs_layout = true;
        self.needs_render = true;
    }

    /// Get window state
    pub fn state(&self) -> &WindowState {
        &self.state
    }

    /// Get mutable window state
    pub fn state_mut(&mut self) -> &mut WindowState {
        &mut self.state
    }

    /// Set window bounds
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
        self.needs_layout = true;
        self.needs_render = true;
    }

    /// Set window title
    pub fn set_title(&mut self, title: String) {
        self.state.title = title;
        self.needs_render = true;
    }

    /// Focus the window
    pub fn focus(&mut self) {
        self.state.focused = true;
        self.needs_render = true;
    }

    /// Unfocus the window
    pub fn unfocus(&mut self) {
        self.state.focused = false;
        self.needs_render = true;
    }

    /// Minimize the window
    pub fn minimize(&mut self) {
        self.state.minimized = true;
        self.state.visible = false;
        self.needs_render = true;
    }

    /// Restore the window from minimized state
    pub fn restore(&mut self) {
        self.state.minimized = false;
        self.state.maximized = false;
        self.state.visible = true;
        self.needs_layout = true;
        self.needs_render = true;
    }

    /// Maximize the window
    pub fn maximize(&mut self, desktop_bounds: Rect) {
        if !self.state.maximized {
            self.state.maximized = true;
            self.state.bounds = desktop_bounds;
            self.needs_layout = true;
            self.needs_render = true;
        }
    }

    /// Toggle maximize state
    pub fn toggle_maximize(&mut self, desktop_bounds: Rect) {
        if self.state.maximized {
            self.restore();
        } else {
            self.maximize(desktop_bounds);
        }
    }

    /// Update layout if needed
    pub fn update_layout(&mut self) {
        if self.needs_layout {
            let content_area = self.decorations.content_area(&self.state);
            self.content.layout(content_area.size);
            self.needs_layout = false;
        }
    }

    /// Generate render commands
    pub fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.state.visible {
            return Vec::new();
        }

        let mut commands = Vec::new();

        // Render decorations
        commands.extend(self.decorations.render(&self.state, z_index));

        // Render content
        let content_layers = self.content.render();
        for layer in content_layers {
            for command in layer.commands {
                // Offset content commands to content area
                let _content_area = self.decorations.content_area(&self.state);
                let adjusted_command = command;

                // TODO: Apply content area offset to render commands
                // This would require modifying the RenderCommand enum to support offset transforms

                commands.push(adjusted_command);
            }
        }

        commands
    }

    /// Handle window events
    pub fn handle_event(&mut self, event: &Event) -> bool {
        use crate::events::{InputEvent, ButtonState, MouseButton};

        match event {
            Event::Input(InputEvent::MouseButton { button: MouseButton::Left, state: ButtonState::Pressed, position, .. }) => {
                // Check for title bar drag
                if self.decorations.hit_test_title_bar(&self.state, *position) {
                    self.drag_state = Some(DragState {
                        start_position: *position,
                        start_window_position: self.state.bounds.origin,
                    });
                    return true;
                }

                // Check for resize border
                if let Some(direction) = self.decorations.hit_test_resize_border(&self.state, *position) {
                    self.resize_state = Some(ResizeState {
                        direction,
                        start_position: *position,
                        start_bounds: self.state.bounds,
                    });
                    return true;
                }

                // Forward to content if in content area
                let content_area = self.decorations.content_area(&self.state);
                if content_area.contains_point(*position) {
                    return self.content.handle_event(event);
                }
            }

            Event::Input(InputEvent::MouseMove { position, .. }) => {
                // Handle window dragging
                if let Some(drag) = &self.drag_state {
                    let delta = *position - drag.start_position;
                    self.state.bounds.origin = drag.start_window_position + delta;
                    self.needs_render = true;
                    return true;
                }

                // Handle window resizing
                if let Some(resize) = &self.resize_state {
                    let delta = *position - resize.start_position;
                    self.apply_resize(resize.direction, delta, resize.start_bounds);
                    self.needs_layout = true;
                    self.needs_render = true;
                    return true;
                }

                // Forward to content
                return self.content.handle_event(event);
            }

            Event::Input(InputEvent::MouseButton { button: MouseButton::Left, state: ButtonState::Released, .. }) => {
                // End dragging/resizing
                self.drag_state = None;
                self.resize_state = None;

                // Forward to content
                return self.content.handle_event(event);
            }

            _ => {
                // Forward other events to content
                return self.content.handle_event(event);
            }
        }

        false
    }

    fn apply_resize(&mut self, direction: ResizeDirection, delta: Point, start_bounds: Rect) {
        let mut new_bounds = start_bounds;

        match direction {
            ResizeDirection::Left => {
                let new_width = (start_bounds.size.width - delta.x).max(self.state.min_size.width);
                let width_change = start_bounds.size.width - new_width;
                new_bounds.origin.x = start_bounds.origin.x + width_change;
                new_bounds.size.width = new_width;
            }
            ResizeDirection::Right => {
                new_bounds.size.width = (start_bounds.size.width + delta.x).max(self.state.min_size.width);
            }
            ResizeDirection::Top => {
                let new_height = (start_bounds.size.height - delta.y).max(self.state.min_size.height);
                let height_change = start_bounds.size.height - new_height;
                new_bounds.origin.y = start_bounds.origin.y + height_change;
                new_bounds.size.height = new_height;
            }
            ResizeDirection::Bottom => {
                new_bounds.size.height = (start_bounds.size.height + delta.y).max(self.state.min_size.height);
            }
            ResizeDirection::TopLeft => {
                self.apply_resize(ResizeDirection::Top, delta, start_bounds);
                self.apply_resize(ResizeDirection::Left, delta, self.state.bounds);
                return;
            }
            ResizeDirection::TopRight => {
                self.apply_resize(ResizeDirection::Top, delta, start_bounds);
                self.apply_resize(ResizeDirection::Right, delta, self.state.bounds);
                return;
            }
            ResizeDirection::BottomLeft => {
                self.apply_resize(ResizeDirection::Bottom, delta, start_bounds);
                self.apply_resize(ResizeDirection::Left, delta, self.state.bounds);
                return;
            }
            ResizeDirection::BottomRight => {
                self.apply_resize(ResizeDirection::Bottom, delta, start_bounds);
                self.apply_resize(ResizeDirection::Right, delta, self.state.bounds);
                return;
            }
        }

        // Apply size constraints
        new_bounds.size.width = new_bounds.size.width
            .max(self.state.min_size.width)
            .min(self.state.max_size.width);
        new_bounds.size.height = new_bounds.size.height
            .max(self.state.min_size.height)
            .min(self.state.max_size.height);

        self.state.bounds = new_bounds;
    }
}

/// Window manager for handling multiple windows
pub struct WindowManager {
    windows: HashMap<WindowId, Window>,
    window_order: Vec<WindowId>, // Z-order, back to front
    focused_window: Option<WindowId>,
    next_window_id: WindowId,
    desktop_bounds: Rect,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new(desktop_bounds: Rect) -> Self {
        Self {
            windows: HashMap::new(),
            window_order: Vec::new(),
            focused_window: None,
            next_window_id: 1,
            desktop_bounds,
        }
    }

    /// Create a new window
    pub fn create_window(&mut self, title: String, content: Box<dyn UIElement>) -> WindowId {
        let window_id = self.next_window_id;
        self.next_window_id += 1;

        let mut window = Window::new(window_id, title);
        window.set_content(content);

        // Position new window with slight offset from others
        let offset = (self.windows.len() as f32 * 30.0) % 200.0;
        let bounds = Rect::new(
            100.0 + offset,
            100.0 + offset,
            800.0,
            600.0,
        );
        window.set_bounds(bounds);

        self.windows.insert(window_id, window);
        self.window_order.push(window_id);
        self.focus_window(window_id);

        window_id
    }

    /// Close a window
    pub fn close_window(&mut self, window_id: WindowId) -> bool {
        if self.windows.remove(&window_id).is_some() {
            self.window_order.retain(|&id| id != window_id);

            // Focus next window if this was focused
            if self.focused_window == Some(window_id) {
                self.focused_window = self.window_order.last().copied();
                if let Some(new_focus) = self.focused_window {
                    if let Some(window) = self.windows.get_mut(&new_focus) {
                        window.focus();
                    }
                }
            }

            true
        } else {
            false
        }
    }

    /// Focus a window (brings to front)
    pub fn focus_window(&mut self, window_id: WindowId) {
        if self.windows.contains_key(&window_id) {
            // Unfocus current window
            if let Some(old_focus) = self.focused_window {
                if let Some(window) = self.windows.get_mut(&old_focus) {
                    window.unfocus();
                }
            }

            // Move to front of Z-order
            self.window_order.retain(|&id| id != window_id);
            self.window_order.push(window_id);

            // Focus new window
            self.focused_window = Some(window_id);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.focus();
            }
        }
    }

    /// Get window by ID
    pub fn get_window(&self, window_id: WindowId) -> Option<&Window> {
        self.windows.get(&window_id)
    }

    /// Get mutable window by ID
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&window_id)
    }

    /// Update all windows (layout, etc.)
    pub fn update(&mut self) {
        for window in self.windows.values_mut() {
            window.update_layout();
        }
    }

    /// Render all windows in Z-order
    pub fn render(&self) -> Vec<RenderLayer> {
        let mut layers = Vec::new();

        for (i, &window_id) in self.window_order.iter().enumerate() {
            if let Some(window) = self.windows.get(&window_id) {
                let commands = window.render(i as f32 * 10.0); // Space out Z-indices
                if !commands.is_empty() {
                    layers.push(RenderLayer {
                        z_index: i as f32 * 10.0,
                        commands,
                        clip_rect: None,
                    });
                }
            }
        }

        layers
    }

    /// Handle events for all windows
    pub fn handle_event(&mut self, event: &Event) -> bool {
        use crate::events::{InputEvent, ButtonState, MouseButton};

        // Handle window focusing on mouse clicks
        if let Event::Input(InputEvent::MouseButton { button: MouseButton::Left, state: ButtonState::Pressed, position, .. }) = event {
            // Check windows in reverse Z-order (front to back)
            for &window_id in self.window_order.iter().rev() {
                if let Some(window) = self.windows.get(&window_id) {
                    if window.state().bounds.contains_point(*position) && window.state().visible {
                        self.focus_window(window_id);
                        break;
                    }
                }
            }
        }

        // Forward event to focused window first
        if let Some(focused_id) = self.focused_window {
            if let Some(window) = self.windows.get_mut(&focused_id) {
                if window.handle_event(event) {
                    return true;
                }
            }
        }

        // If not handled by focused window, try all windows in reverse Z-order
        for &window_id in self.window_order.iter().rev() {
            if Some(window_id) != self.focused_window {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    if window.handle_event(event) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get list of all window IDs in Z-order
    pub fn window_list(&self) -> &[WindowId] {
        &self.window_order
    }

    /// Get focused window ID
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }

    /// Set desktop bounds (for window maximizing)
    pub fn set_desktop_bounds(&mut self, bounds: Rect) {
        self.desktop_bounds = bounds;
    }
}

// TODO: Advanced window management features to be implemented:
//
// 1. Window animations
//    - Smooth open/close animations
//    - Minimize/maximize transitions
//    - Window movement animations
//
// 2. Advanced window features
//    - Modal windows and dialogs
//    - Window groups and tabs
//    - Virtual desktops/workspaces
//    - Window tiling and snapping
//
// 3. Performance optimizations
//    - Occlusion culling (don't render hidden windows)
//    - Dirty region tracking
//    - GPU-accelerated window compositing
//    - Multi-threaded window processing
//
// 4. Platform integration
//    - Native window decorations option
//    - System window list integration
//    - Alt+Tab window switching
//    - Taskbar integration