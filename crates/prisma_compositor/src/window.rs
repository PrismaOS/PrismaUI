/// High-performance window management with GPU acceleration
use std::sync::{Arc, RwLock, Mutex, atomic::{AtomicU64, Ordering}};
use std::collections::HashMap;
use winit::event::{MouseButton, ElementState};

use crate::ui::{UISystem, UILayer, UIElement, UIElementType, UIRect};

/// Unique window identifier
pub type WindowId = u64;

/// Window events for the compositor
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window was created
    Created(WindowId),
    /// Window was destroyed
    Destroyed(WindowId),
    /// Window was moved
    Moved { id: WindowId, x: f32, y: f32 },
    /// Window was resized
    Resized { id: WindowId, width: f32, height: f32 },
    /// Window gained focus
    Focused(WindowId),
    /// Window lost focus
    Unfocused(WindowId),
    /// Window was minimized
    Minimized(WindowId),
    /// Window was maximized
    Maximized(WindowId),
    /// Window was restored
    Restored(WindowId),
    /// Window close requested
    CloseRequested(WindowId),
}

/// Window state for persistence and management
#[derive(Debug, Clone)]
pub struct WindowState {
    pub id: WindowId,
    pub title: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}

/// Window decoration style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowDecorations {
    None,
    Minimal,      // Only title bar
    Standard,     // Title bar with min/max/close
    System,       // Use system decorations
}

/// Window compositor window
pub struct Window {
    pub id: WindowId,
    pub state: WindowState,
    pub ui_layer: u64, // UI layer ID for this window
    pub content_area: UIRect,
    pub decoration_elements: Vec<u64>, // UI element IDs for decorations

    /// Window behavior
    pub draggable: bool,
    pub resizable: bool,
    pub snap_enabled: bool,

    /// Performance optimization
    pub needs_redraw: bool,
    pub last_draw_time: std::time::Instant,
    pub draw_count: u64,

    /// Window decorations
    decorations: WindowDecorations,
    title_bar_height: f32,
    border_width: f32,
}

impl Window {
    /// Create a new window
    pub fn new(
        id: WindowId,
        title: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        ui_system: &UISystem,
    ) -> Self {
        let state = WindowState {
            id,
            title: title.clone(),
            x,
            y,
            width,
            height,
            minimized: false,
            maximized: false,
            focused: false,
            resizable: true,
            decorations: true,
            always_on_top: false,
            transparent: false,
        };

        // Create UI layer for this window
        let ui_layer = ui_system.create_layer(format!("Window-{}", id));

        let title_bar_height = 32.0;
        let border_width = 2.0;

        let content_area = UIRect::new(
            x + border_width,
            y + title_bar_height,
            width - 2.0 * border_width,
            height - title_bar_height - border_width,
        );

        let mut window = Self {
            id,
            state,
            ui_layer,
            content_area,
            decoration_elements: Vec::new(),
            draggable: true,
            resizable: true,
            snap_enabled: true,
            needs_redraw: true,
            last_draw_time: std::time::Instant::now(),
            draw_count: 0,
            decorations: WindowDecorations::Standard,
            title_bar_height,
            border_width,
        };

        // Create window decorations
        window.create_decorations(ui_system);

        window
    }

    /// Create window decoration elements
    fn create_decorations(&mut self, ui_system: &UISystem) {
        if self.decorations == WindowDecorations::None {
            return;
        }

        // Window border
        let border_color = if self.state.focused {
            [0.4, 0.6, 1.0, 1.0] // Blue when focused
        } else {
            [0.5, 0.5, 0.5, 1.0] // Gray when unfocused
        };

        // Top border (title bar background)
        let title_bar = UIElement::rect(
            self.generate_decoration_id(),
            UIRect::new(self.state.x, self.state.y, self.state.width, self.title_bar_height),
            [0.2, 0.2, 0.2, 1.0],
        );
        ui_system.add_element_to_layer(self.ui_layer, title_bar);
        self.decoration_elements.push(title_bar.id);

        // Title text
        let title_text = UIElement::text(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x + 10.0,
                self.state.y + 6.0,
                self.state.width - 120.0, // Leave space for buttons
                20.0,
            ),
            self.state.title.clone(),
            0, // Default font
            [1.0, 1.0, 1.0, 1.0],
        );
        ui_system.add_element_to_layer(self.ui_layer, title_text);
        self.decoration_elements.push(title_text.id);

        // Window control buttons
        let button_size = 24.0;
        let button_y = self.state.y + 4.0;

        // Close button
        let close_button = UIElement::button(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x + self.state.width - button_size - 8.0,
                button_y,
                button_size,
                button_size,
            ),
            "✕".to_string(),
            0,
        );
        ui_system.add_element_to_layer(self.ui_layer, close_button);
        self.decoration_elements.push(close_button.id);

        // Maximize button
        let maximize_button = UIElement::button(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x + self.state.width - 2.0 * button_size - 12.0,
                button_y,
                button_size,
                button_size,
            ),
            if self.state.maximized { "⧉" } else { "□" }.to_string(),
            0,
        );
        ui_system.add_element_to_layer(self.ui_layer, maximize_button);
        self.decoration_elements.push(maximize_button.id);

        // Minimize button
        let minimize_button = UIElement::button(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x + self.state.width - 3.0 * button_size - 16.0,
                button_y,
                button_size,
                button_size,
            ),
            "−".to_string(),
            0,
        );
        ui_system.add_element_to_layer(self.ui_layer, minimize_button);
        self.decoration_elements.push(minimize_button.id);

        // Left border
        let left_border = UIElement::rect(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x,
                self.state.y + self.title_bar_height,
                self.border_width,
                self.state.height - self.title_bar_height,
            ),
            border_color,
        );
        ui_system.add_element_to_layer(self.ui_layer, left_border);
        self.decoration_elements.push(left_border.id);

        // Right border
        let right_border = UIElement::rect(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x + self.state.width - self.border_width,
                self.state.y + self.title_bar_height,
                self.border_width,
                self.state.height - self.title_bar_height,
            ),
            border_color,
        );
        ui_system.add_element_to_layer(self.ui_layer, right_border);
        self.decoration_elements.push(right_border.id);

        // Bottom border
        let bottom_border = UIElement::rect(
            self.generate_decoration_id(),
            UIRect::new(
                self.state.x,
                self.state.y + self.state.height - self.border_width,
                self.state.width,
                self.border_width,
            ),
            border_color,
        );
        ui_system.add_element_to_layer(self.ui_layer, bottom_border);
        self.decoration_elements.push(bottom_border.id);
    }

    /// Update window position
    pub fn set_position(&mut self, x: f32, y: f32, ui_system: &UISystem) {
        self.state.x = x;
        self.state.y = y;
        self.update_content_area();
        self.update_decorations(ui_system);
        self.needs_redraw = true;
    }

    /// Update window size
    pub fn set_size(&mut self, width: f32, height: f32, ui_system: &UISystem) {
        self.state.width = width.max(200.0); // Minimum width
        self.state.height = height.max(150.0); // Minimum height
        self.update_content_area();
        self.update_decorations(ui_system);
        self.needs_redraw = true;
    }

    /// Update window bounds (position and size)
    pub fn set_bounds(&mut self, x: f32, y: f32, width: f32, height: f32, ui_system: &UISystem) {
        self.state.x = x;
        self.state.y = y;
        self.state.width = width.max(200.0);
        self.state.height = height.max(150.0);
        self.update_content_area();
        self.update_decorations(ui_system);
        self.needs_redraw = true;
    }

    /// Set window focus state
    pub fn set_focused(&mut self, focused: bool, ui_system: &UISystem) {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.update_decorations(ui_system);
            self.needs_redraw = true;
        }
    }

    /// Minimize window
    pub fn minimize(&mut self, ui_system: &UISystem) {
        if !self.state.minimized {
            self.state.minimized = true;
            self.hide_decorations(ui_system);
        }
    }

    /// Maximize window
    pub fn maximize(&mut self, ui_system: &UISystem) {
        self.state.maximized = !self.state.maximized;
        self.update_decorations(ui_system);
        self.needs_redraw = true;
    }

    /// Restore window from minimized/maximized state
    pub fn restore(&mut self, ui_system: &UISystem) {
        if self.state.minimized {
            self.state.minimized = false;
            self.show_decorations(ui_system);
        }
        if self.state.maximized {
            self.state.maximized = false;
            self.update_decorations(ui_system);
        }
        self.needs_redraw = true;
    }

    /// Check if point is in title bar (for dragging)
    pub fn is_point_in_title_bar(&self, x: f32, y: f32) -> bool {
        x >= self.state.x
            && x <= self.state.x + self.state.width
            && y >= self.state.y
            && y <= self.state.y + self.title_bar_height
    }

    /// Check if point is in resize area
    pub fn get_resize_cursor(&self, x: f32, y: f32) -> Option<ResizeCursor> {
        let resize_area = 8.0;

        let left_edge = (x - self.state.x).abs() < resize_area;
        let right_edge = (x - (self.state.x + self.state.width)).abs() < resize_area;
        let top_edge = (y - self.state.y).abs() < resize_area;
        let bottom_edge = (y - (self.state.y + self.state.height)).abs() < resize_area;

        match (left_edge, right_edge, top_edge, bottom_edge) {
            (true, false, true, false) => Some(ResizeCursor::NorthWest),
            (false, true, true, false) => Some(ResizeCursor::NorthEast),
            (true, false, false, true) => Some(ResizeCursor::SouthWest),
            (false, true, false, true) => Some(ResizeCursor::SouthEast),
            (true, false, false, false) => Some(ResizeCursor::West),
            (false, true, false, false) => Some(ResizeCursor::East),
            (false, false, true, false) => Some(ResizeCursor::North),
            (false, false, false, true) => Some(ResizeCursor::South),
            _ => None,
        }
    }

    /// Update content area based on window bounds
    fn update_content_area(&mut self) {
        self.content_area = UIRect::new(
            self.state.x + self.border_width,
            self.state.y + self.title_bar_height,
            self.state.width - 2.0 * self.border_width,
            self.state.height - self.title_bar_height - self.border_width,
        );
    }

    /// Update decoration positions
    fn update_decorations(&mut self, ui_system: &UISystem) {
        // Remove old decorations
        for &element_id in &self.decoration_elements {
            ui_system.remove_element(self.ui_layer, element_id);
        }
        self.decoration_elements.clear();

        // Create new decorations
        self.create_decorations(ui_system);
    }

    /// Hide decorations (for minimized windows)
    fn hide_decorations(&self, ui_system: &UISystem) {
        for &element_id in &self.decoration_elements {
            ui_system.remove_element(self.ui_layer, element_id);
        }
    }

    /// Show decorations (when restoring)
    fn show_decorations(&mut self, ui_system: &UISystem) {
        self.create_decorations(ui_system);
    }

    /// Generate unique decoration element ID
    fn generate_decoration_id(&self) -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(1000);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// Resize cursor types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeCursor {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Window manager for the compositor
pub struct WindowManager {
    windows: Arc<RwLock<HashMap<WindowId, Window>>>,
    window_order: Arc<RwLock<Vec<WindowId>>>, // Z-order for rendering
    focused_window: Arc<RwLock<Option<WindowId>>>,
    ui_system: Arc<UISystem>,

    /// Window interaction state
    dragging_window: Arc<Mutex<Option<WindowId>>>,
    resizing_window: Arc<Mutex<Option<(WindowId, ResizeCursor)>>>,
    drag_offset: Arc<Mutex<(f32, f32)>>,

    /// Window ID generation
    next_window_id: AtomicU64,

    /// Performance tracking
    total_windows: AtomicU64,
    visible_windows: AtomicU64,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new() -> Self {
        let ui_system = Arc::new(UISystem::new(1920.0, 1080.0));

        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            window_order: Arc::new(RwLock::new(Vec::new())),
            focused_window: Arc::new(RwLock::new(None)),
            ui_system,
            dragging_window: Arc::new(Mutex::new(None)),
            resizing_window: Arc::new(Mutex::new(None)),
            drag_offset: Arc::new(Mutex::new((0.0, 0.0))),
            next_window_id: AtomicU64::new(1),
            total_windows: AtomicU64::new(0),
            visible_windows: AtomicU64::new(0),
        }
    }

    /// Create a new window
    pub fn create_window(
        &self,
        title: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> WindowId {
        let window_id = self.next_window_id.fetch_add(1, Ordering::Relaxed);

        let window = Window::new(window_id, title, x, y, width, height, &self.ui_system);

        {
            let mut windows = self.windows.write().unwrap();
            windows.insert(window_id, window);
        }

        {
            let mut window_order = self.window_order.write().unwrap();
            window_order.push(window_id);
        }

        self.total_windows.fetch_add(1, Ordering::Relaxed);
        self.visible_windows.fetch_add(1, Ordering::Relaxed);

        // Focus the new window
        self.focus_window(window_id);

        window_id
    }

    /// Destroy a window
    pub fn destroy_window(&self, window_id: WindowId) {
        {
            let mut windows = self.windows.write().unwrap();
            windows.remove(&window_id);
        }

        {
            let mut window_order = self.window_order.write().unwrap();
            window_order.retain(|&id| id != window_id);
        }

        // Focus another window if this was focused
        let was_focused = {
            let focused = self.focused_window.read().unwrap();
            *focused == Some(window_id)
        };

        if was_focused {
            let next_window = {
                let window_order = self.window_order.read().unwrap();
                window_order.last().copied()
            };

            if let Some(next_id) = next_window {
                self.focus_window(next_id);
            } else {
                *self.focused_window.write().unwrap() = None;
            }
        }

        self.visible_windows.fetch_sub(1, Ordering::Relaxed);
    }

    /// Focus a window
    pub fn focus_window(&self, window_id: WindowId) {
        // Unfocus current window
        if let Ok(focused) = self.focused_window.read() {
            if let Some(current_focused) = *focused {
                if let Ok(mut windows) = self.windows.write() {
                    if let Some(window) = windows.get_mut(&current_focused) {
                        window.set_focused(false, &self.ui_system);
                    }
                }
            }
        }

        // Focus new window
        {
            let mut focused = self.focused_window.write().unwrap();
            *focused = Some(window_id);
        }

        if let Ok(mut windows) = self.windows.write() {
            if let Some(window) = windows.get_mut(&window_id) {
                window.set_focused(true, &self.ui_system);
            }
        }

        // Move to front of Z-order
        {
            let mut window_order = self.window_order.write().unwrap();
            window_order.retain(|&id| id != window_id);
            window_order.push(window_id);
        }
    }

    /// Handle cursor movement for window interactions
    pub fn handle_cursor_move(&self, x: f32, y: f32) {
        // Handle window dragging
        if let Ok(dragging) = self.dragging_window.lock() {
            if let Some(window_id) = *dragging {
                let drag_offset = *self.drag_offset.lock().unwrap();
                let new_x = x - drag_offset.0;
                let new_y = y - drag_offset.1;

                if let Ok(mut windows) = self.windows.write() {
                    if let Some(window) = windows.get_mut(&window_id) {
                        window.set_position(new_x, new_y, &self.ui_system);
                    }
                }
            }
        }

        // Handle window resizing
        if let Ok(resizing) = self.resizing_window.lock() {
            if let Some((window_id, cursor)) = *resizing {
                if let Ok(mut windows) = self.windows.write() {
                    if let Some(window) = windows.get_mut(&window_id) {
                        self.resize_window_with_cursor(window, x, y, cursor);
                    }
                }
            }
        }
    }

    /// Handle mouse input
    pub fn handle_mouse_input(&self, button: MouseButton, state: ElementState) {
        if button != MouseButton::Left {
            return;
        }

        match state {
            ElementState::Pressed => {
                // Start drag or resize operation
                // This would be implemented based on cursor position
            }
            ElementState::Released => {
                // End drag or resize operation
                *self.dragging_window.lock().unwrap() = None;
                *self.resizing_window.lock().unwrap() = None;
            }
        }
    }

    /// Handle high-precision mouse movement
    pub fn handle_mouse_delta(&self, dx: f32, dy: f32) {
        // Use for smooth window dragging with sub-pixel precision
        if let Ok(dragging) = self.dragging_window.lock() {
            if let Some(window_id) = *dragging {
                if let Ok(mut windows) = self.windows.write() {
                    if let Some(window) = windows.get_mut(&window_id) {
                        let new_x = window.state.x + dx;
                        let new_y = window.state.y + dy;
                        window.set_position(new_x, new_y, &self.ui_system);
                    }
                }
            }
        }
    }

    /// Resize window based on cursor and position
    fn resize_window_with_cursor(&self, window: &mut Window, x: f32, y: f32, cursor: ResizeCursor) {
        let current_x = window.state.x;
        let current_y = window.state.y;
        let current_width = window.state.width;
        let current_height = window.state.height;

        let (new_x, new_y, new_width, new_height) = match cursor {
            ResizeCursor::East => (current_x, current_y, x - current_x, current_height),
            ResizeCursor::West => (x, current_y, current_x + current_width - x, current_height),
            ResizeCursor::North => (current_x, y, current_width, current_y + current_height - y),
            ResizeCursor::South => (current_x, current_y, current_width, y - current_y),
            ResizeCursor::NorthEast => (
                current_x,
                y,
                x - current_x,
                current_y + current_height - y,
            ),
            ResizeCursor::NorthWest => (x, y, current_x + current_width - x, current_y + current_height - y),
            ResizeCursor::SouthEast => (current_x, current_y, x - current_x, y - current_y),
            ResizeCursor::SouthWest => (x, current_y, current_x + current_width - x, y - current_y),
        };

        window.set_bounds(new_x, new_y, new_width, new_height, &self.ui_system);
    }

    /// Get UI system reference
    pub fn get_ui_system(&self) -> Arc<UISystem> {
        Arc::clone(&self.ui_system)
    }

    /// Get window count
    pub fn get_window_count(&self) -> (u64, u64) {
        (
            self.total_windows.load(Ordering::Relaxed),
            self.visible_windows.load(Ordering::Relaxed),
        )
    }
}