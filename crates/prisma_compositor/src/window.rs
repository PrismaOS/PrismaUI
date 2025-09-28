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
            title_bar_height: 44.0, // Taller for macOS-style proportions
            border_width: 0.5, // Thinner border for elegance
            corner_radius: 16.0, // Much more rounded for modern macOS look
            title_bar_color: Color::from_hex("#F8F8F8").unwrap_or(Color::new(0.97, 0.97, 0.97, 0.95)), // Light translucent
            border_color: Color::from_hex("#E0E0E0").unwrap_or(Color::new(0.88, 0.88, 0.88, 0.8)), // Light border
            focused_color: Color::from_hex("#007AFF").unwrap_or(Color::BLUE), // macOS blue
            unfocused_color: Color::from_hex("#D1D1D6").unwrap_or(Color::new(0.82, 0.82, 0.84, 1.0)), // System gray
        }
    }
}

impl WindowDecorations {
    /// Generate render commands for window decorations
    pub fn render(&self, state: &WindowState, z_index: f32) -> Vec<RenderCommand> {
        if !state.decorations {
            return Vec::new();
        }

        let mut commands = Vec::new();

        // Main window background with beautiful glass effect
        let window_bg_color = if state.focused {
            Color::new(1.0, 1.0, 1.0, 0.95) // Translucent white when focused
        } else {
            Color::new(0.95, 0.95, 0.97, 0.9) // Slightly gray when unfocused
        };

        commands.push(RenderCommand::RoundedRectangle {
            rect: state.bounds,
            corner_radius: self.corner_radius,
            color: window_bg_color,
            transform: crate::geometry::Transform::identity(),
            z_index,
        });

        // Window border (subtle)
        let border_color = if state.focused {
            Color::new(0.0, 0.48, 1.0, 0.2) // Subtle blue border when focused
        } else {
            Color::new(0.0, 0.0, 0.0, 0.08) // Very subtle dark border when unfocused
        };

        // Border as a slightly larger rounded rectangle behind
        let border_rect = Rect::new(
            state.bounds.origin.x - self.border_width,
            state.bounds.origin.y - self.border_width,
            state.bounds.size.width + 2.0 * self.border_width,
            state.bounds.size.height + 2.0 * self.border_width,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: border_rect,
            corner_radius: self.corner_radius + self.border_width,
            color: border_color,
            transform: crate::geometry::Transform::identity(),
            z_index: z_index - 0.1,
        });

        // Title bar with gradient background
        let title_bar_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y,
            state.bounds.size.width,
            self.title_bar_height,
        );

        let title_bar_start_color = if state.focused {
            Color::new(0.98, 0.98, 0.98, 0.95)
        } else {
            Color::new(0.94, 0.94, 0.96, 0.9)
        };

        let title_bar_end_color = if state.focused {
            Color::new(0.96, 0.96, 0.96, 0.95)
        } else {
            Color::new(0.92, 0.92, 0.94, 0.9)
        };

        commands.push(RenderCommand::GradientRectangle {
            rect: title_bar_rect,
            start_color: title_bar_start_color,
            end_color: title_bar_end_color,
            direction: std::f32::consts::PI / 2.0, // Top to bottom
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.2,
        });

        // Title bar separator line
        let separator_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y + self.title_bar_height - 0.5,
            state.bounds.size.width,
            0.5,
        );

        commands.push(RenderCommand::Rectangle {
            rect: separator_rect,
            color: Color::new(0.0, 0.0, 0.0, 0.05),
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.3,
        });

        // Traffic light buttons (macOS style)
        self.render_traffic_lights(state, z_index + 0.4, &mut commands);

        // Window content background
        let content_rect = Rect::new(
            state.bounds.origin.x,
            state.bounds.origin.y + self.title_bar_height,
            state.bounds.size.width,
            state.bounds.size.height - self.title_bar_height,
        );

        commands.push(RenderCommand::RoundedRectangle {
            rect: content_rect,
            corner_radius: self.corner_radius, // Only round bottom corners
            color: Color::new(1.0, 1.0, 1.0, 0.98),
            transform: crate::geometry::Transform::identity(),
            z_index: z_index + 0.1,
        });

        commands
    }

    /// Render macOS-style traffic light buttons
    fn render_traffic_lights(&self, state: &WindowState, z_index: f32, commands: &mut Vec<RenderCommand>) {
        let button_size = 14.0;
        let button_spacing = 8.0;
        let margin_left = 16.0;
        let margin_top = (self.title_bar_height - button_size) / 2.0;

        let button_y = state.bounds.origin.y + margin_top;

        // Close button (red)
        let close_x = state.bounds.origin.x + margin_left;
        let close_rect = Rect::new(close_x, button_y, button_size, button_size);

        let close_color = if state.focused {
            Color::from_hex("#FF5F57").unwrap_or(Color::RED)
        } else {
            Color::new(0.82, 0.82, 0.84, 1.0) // Gray when unfocused
        };

        commands.push(RenderCommand::RoundedRectangle {
            rect: close_rect,
            corner_radius: button_size / 2.0, // Perfect circle
            color: close_color,
            transform: crate::geometry::Transform::identity(),
            z_index,
        });

        // Minimize button (yellow)
        let minimize_x = close_x + button_size + button_spacing;
        let minimize_rect = Rect::new(minimize_x, button_y, button_size, button_size);

        let minimize_color = if state.focused {
            Color::from_hex("#FFBD2E").unwrap_or(Color::new(1.0, 0.74, 0.18, 1.0))
        } else {
            Color::new(0.82, 0.82, 0.84, 1.0)
        };

        commands.push(RenderCommand::RoundedRectangle {
            rect: minimize_rect,
            corner_radius: button_size / 2.0,
            color: minimize_color,
            transform: crate::geometry::Transform::identity(),
            z_index,
        });

        // Maximize button (green)
        let maximize_x = minimize_x + button_size + button_spacing;
        let maximize_rect = Rect::new(maximize_x, button_y, button_size, button_size);

        let maximize_color = if state.focused {
            Color::from_hex("#28CA42").unwrap_or(Color::GREEN)
        } else {
            Color::new(0.82, 0.82, 0.84, 1.0)
        };

        commands.push(RenderCommand::RoundedRectangle {
            rect: maximize_rect,
            corner_radius: button_size / 2.0,
            color: maximize_color,
            transform: crate::geometry::Transform::identity(),
            z_index,
        });

        // Add subtle button borders for depth
        if state.focused {
            for (rect, base_color) in [
                (close_rect, close_color),
                (minimize_rect, minimize_color),
                (maximize_rect, maximize_color),
            ] {
                let border_color = Color::new(
                    base_color.r * 0.8,
                    base_color.g * 0.8,
                    base_color.b * 0.8,
                    0.8,
                );

                let border_rect = Rect::new(
                    rect.origin.x - 0.5,
                    rect.origin.y - 0.5,
                    rect.size.width + 1.0,
                    rect.size.height + 1.0,
                );

                commands.push(RenderCommand::RoundedRectangle {
                    rect: border_rect,
                    corner_radius: (button_size + 1.0) / 2.0,
                    color: border_color,
                    transform: crate::geometry::Transform::identity(),
                    z_index: z_index - 0.1,
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