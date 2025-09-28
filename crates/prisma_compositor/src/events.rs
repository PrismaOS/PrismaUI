/// Event system for the GPU-accelerated UI
use serde::{Deserialize, Serialize};
use crate::geometry::{Point, Size};

/// Main event types for the UI system
#[derive(Debug, Clone)]
pub enum Event {
    /// Input events (mouse, keyboard, touch)
    Input(InputEvent),
    /// Window events (resize, close, etc.)
    Window(WindowEvent),
    /// Custom UI events
    UI(UIEvent),
}

/// Input events with high-precision coordinates
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse button press/release
    MouseButton {
        button: MouseButton,
        state: ButtonState,
        position: Point,
        modifiers: Modifiers,
    },
    /// Mouse movement
    MouseMove {
        position: Point,
        delta: Point,
    },
    /// Mouse wheel scroll
    MouseWheel {
        delta: Point, // x = horizontal, y = vertical
        position: Point,
        modifiers: Modifiers,
    },
    /// Key press/release
    Keyboard {
        key: Key,
        state: ButtonState,
        modifiers: Modifiers,
    },
    /// Text input (for text fields)
    TextInput {
        text: String,
    },
    /// Touch events (for future touch support)
    Touch {
        id: u64,
        phase: TouchPhase,
        position: Point,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

impl Modifiers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }
}

/// Key representation (simplified for now)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    Character(String),
    Escape,
    Tab,
    Enter,
    Space,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Unknown,
}

impl Key {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Escape" => Key::Escape,
            "Tab" => Key::Tab,
            "Enter" => Key::Enter,
            " " => Key::Space,
            "Backspace" => Key::Backspace,
            "Delete" => Key::Delete,
            "ArrowUp" => Key::ArrowUp,
            "ArrowDown" => Key::ArrowDown,
            "ArrowLeft" => Key::ArrowLeft,
            "ArrowRight" => Key::ArrowRight,
            "Home" => Key::Home,
            "End" => Key::End,
            "PageUp" => Key::PageUp,
            "PageDown" => Key::PageDown,
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "F5" => Key::F5,
            "F6" => Key::F6,
            "F7" => Key::F7,
            "F8" => Key::F8,
            "F9" => Key::F9,
            "F10" => Key::F10,
            "F11" => Key::F11,
            "F12" => Key::F12,
            _ => {
                if s.len() == 1 || s.chars().count() == 1 {
                    Key::Character(s.to_string())
                } else {
                    Key::Unknown
                }
            }
        }
    }
}

/// Window-related events
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window resized
    Resized { size: Size },
    /// Window moved
    Moved { position: Point },
    /// Window closed
    CloseRequested,
    /// Window focused/unfocused
    FocusChanged { focused: bool },
    /// Window scale factor changed (DPI change)
    ScaleFactorChanged { scale_factor: f64 },
    /// Window theme changed (dark/light mode)
    ThemeChanged { dark_mode: bool },
}

/// Custom UI events
#[derive(Debug, Clone)]
pub enum UIEvent {
    /// Button clicked
    ButtonClicked { id: String },
    /// Text field changed
    TextChanged { id: String, text: String },
    /// Slider value changed
    SliderChanged { id: String, value: f32 },
    /// Checkbox toggled
    CheckboxToggled { id: String, checked: bool },
    /// Menu item selected
    MenuItemSelected { id: String },
    /// Custom application event
    Custom { event_type: String, data: serde_json::Value },
}

/// Event handler trait for UI elements
pub trait EventHandler {
    /// Handle an event, return true if consumed
    fn handle_event(&mut self, event: &Event) -> bool;

    /// Get the bounds for hit testing
    fn bounds(&self) -> crate::geometry::Rect;

    /// Check if this handler should receive events at the given point
    fn hit_test(&self, point: Point) -> bool {
        self.bounds().contains_point(point)
    }
}

/// Event dispatcher for efficient event routing
pub struct EventDispatcher {
    handlers: Vec<Box<dyn EventHandler>>,
    captured_handler: Option<usize>, // For mouse capture scenarios
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            captured_handler: None,
        }
    }

    /// Add an event handler
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// Remove all handlers
    pub fn clear_handlers(&mut self) {
        self.handlers.clear();
        self.captured_handler = None;
    }

    /// Dispatch an event to appropriate handlers
    pub fn dispatch(&mut self, event: &Event) -> bool {
        match event {
            Event::Input(input_event) => self.dispatch_input_event(input_event),
            Event::Window(window_event) => self.dispatch_window_event(window_event),
            Event::UI(ui_event) => self.dispatch_ui_event(ui_event),
        }
    }

    fn dispatch_input_event(&mut self, event: &InputEvent) -> bool {
        // If we have a captured handler, send all events to it
        if let Some(captured_idx) = self.captured_handler {
            if captured_idx < self.handlers.len() {
                return self.handlers[captured_idx].handle_event(&Event::Input(event.clone()));
            }
        }

        // For mouse events, do hit testing
        match event {
            InputEvent::MouseButton { position, .. } |
            InputEvent::MouseMove { position, .. } |
            InputEvent::MouseWheel { position, .. } => {
                // Iterate handlers in reverse order (top to bottom)
                for (idx, handler) in self.handlers.iter_mut().enumerate().rev() {
                    if handler.hit_test(*position) {
                        if handler.handle_event(&Event::Input(event.clone())) {
                            // Handler consumed the event
                            if matches!(event, InputEvent::MouseButton { state: ButtonState::Pressed, .. }) {
                                // Capture mouse for this handler
                                self.captured_handler = Some(idx);
                            }
                            return true;
                        }
                    }
                }
            }
            InputEvent::Keyboard { .. } | InputEvent::TextInput { .. } => {
                // Keyboard events go to focused handler (TODO: implement focus system)
                // For now, just send to all handlers
                for handler in &mut self.handlers {
                    if handler.handle_event(&Event::Input(event.clone())) {
                        return true;
                    }
                }
            }
            InputEvent::Touch { position, .. } => {
                // Similar to mouse events
                for handler in self.handlers.iter_mut().rev() {
                    if handler.hit_test(*position) {
                        if handler.handle_event(&Event::Input(event.clone())) {
                            return true;
                        }
                    }
                }
            }
        }

        // Release capture on mouse button release
        if matches!(event, InputEvent::MouseButton { state: ButtonState::Released, .. }) {
            self.captured_handler = None;
        }

        false
    }

    fn dispatch_window_event(&mut self, event: &WindowEvent) -> bool {
        // Window events are broadcast to all handlers
        let mut consumed = false;
        for handler in &mut self.handlers {
            if handler.handle_event(&Event::Window(event.clone())) {
                consumed = true;
            }
        }
        consumed
    }

    fn dispatch_ui_event(&mut self, event: &UIEvent) -> bool {
        // UI events are broadcast to all handlers
        let mut consumed = false;
        for handler in &mut self.handlers {
            if handler.handle_event(&Event::UI(event.clone())) {
                consumed = true;
            }
        }
        consumed
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert winit events to our event system
impl From<winit::event::MouseButton> for MouseButton {
    fn from(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Other(4),
            winit::event::MouseButton::Forward => MouseButton::Other(5),
            winit::event::MouseButton::Other(id) => MouseButton::Other(id as u8),
        }
    }
}

impl From<winit::event::ElementState> for ButtonState {
    fn from(state: winit::event::ElementState) -> Self {
        match state {
            winit::event::ElementState::Pressed => ButtonState::Pressed,
            winit::event::ElementState::Released => ButtonState::Released,
        }
    }
}

// TODO: Animation event system will be added here:
//
// #[derive(Debug, Clone)]
// pub enum AnimationEvent {
//     Started { id: String },
//     Updated { id: String, progress: f32 },
//     Completed { id: String },
// }
//
// This will integrate with the animation system for smooth UI transitions.