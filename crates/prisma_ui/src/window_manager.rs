/// Core window management system with GPU-backed compositing
use gpui::{
    div, px, size, AnyElement, App, AppContext, Bounds, Context,
    Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels,
    Point, Render, Size, Styled, WeakEntity, Window
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, IconName, StyledExt
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Events emitted by windows
#[derive(Clone, Debug)]
pub enum WindowEvent {
    /// Window was closed
    Closed(WindowId),
    /// Window was focused
    Focused(WindowId),
    /// Window was moved
    Moved { id: WindowId, position: Point<Pixels> },
    /// Window was resized
    Resized { id: WindowId, size: Size<Pixels> },
    /// Window requests to be minimized
    MinimizeRequested(WindowId),
    /// Window requests to be maximized
    MaximizeRequested(WindowId),
}

/// Unique identifier for windows
pub type WindowId = Uuid;

/// Window state for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowState {
    pub id: WindowId,
    pub title: String,
    pub bounds: Bounds<Pixels>,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
}

/// Window snapping zones for tiling
#[derive(Clone, Debug, PartialEq)]
pub enum SnapZone {
    Left,
    Right,
    Top,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

/// Core window manager handling all window operations
pub struct WindowManager {
    /// All managed windows
    windows: HashMap<WindowId, Entity<ManagedWindow>>,
    /// Cached window bounds to avoid reading during events
    window_bounds: HashMap<WindowId, Bounds<Pixels>>,
    /// Window z-index ordering (higher = on top)
    window_z_index: HashMap<WindowId, i32>,
    /// Next z-index to assign
    next_z_index: i32,
    /// Currently focused window
    focused_window: Option<WindowId>,
    /// Desktop bounds for window positioning
    pub desktop_bounds: Bounds<Pixels>,
    /// Window being dragged
    dragging_window: Option<WindowId>,
    /// Window being resized
    resizing_window: Option<WindowId>,
    /// Resize handle being dragged
    resize_handle: Option<ResizeHandle>,
    /// Current snap zone during drag
    snap_zone: Option<SnapZone>,
    /// Focus handle for the manager
    focus_handle: FocusHandle,
    /// Pending drag position to avoid reentrancy
    pending_drag_position: Option<Point<Pixels>>,
    /// Pending window focus to avoid reentrancy
    pending_focus_window: Option<WindowId>,
    /// Pending resize update to avoid reentrancy
    pending_resize_data: Option<(WindowId, Point<Pixels>, ResizeHandle)>,
    /// Drag offset from mouse position to window origin
    drag_offset: Point<Pixels>,
}

#[derive(Clone, Debug)]
enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
}

impl WindowManager {
    pub fn new(desktop_bounds: Bounds<Pixels>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            windows: HashMap::new(),
            window_bounds: HashMap::new(),
            window_z_index: HashMap::new(),
            next_z_index: 1,
            focused_window: None,
            desktop_bounds,
            dragging_window: None,
            resizing_window: None,
            resize_handle: None,
            snap_zone: None,
            focus_handle: cx.focus_handle(),
            pending_drag_position: None,
            pending_focus_window: None,
            pending_resize_data: None,
            drag_offset: Point::default(),
        })
    }

    /// Create a new window with the given content
    pub fn create_window<V: 'static + Render>(
        &mut self,
        title: String,
        content: Entity<V>,
        initial_bounds: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> WindowId {
        let id = Uuid::new_v4();

        // Default window bounds if none provided
        let bounds = initial_bounds.unwrap_or_else(|| {
            let size = size(px(800.0), px(600.0));
            let center = self.desktop_bounds.center();
            Bounds::centered(None, size, cx)
        });

        let weak_self = cx.weak_entity();
        let managed_window = cx.new(|cx| ManagedWindow::new(
            id,
            title,
            content,
            bounds,
            weak_self,
            cx,
        ));

        self.windows.insert(id, managed_window);
        self.window_bounds.insert(id, bounds);
        self.window_z_index.insert(id, self.next_z_index);
        self.next_z_index += 1;
        self.focus_window(id, window, cx);

        cx.emit(WindowEvent::Focused(id));
        id
    }

    /// Focus a specific window
    pub fn focus_window(&mut self, id: WindowId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(old_focused) = self.focused_window {
            if let Some(window) = self.windows.get(&old_focused) {
                window.update(cx, |w, cx| {
                    w.set_focused(false, cx);
                });
            }
        }

        self.focused_window = Some(id);
        if let Some(window) = self.windows.get(&id) {
            window.update(cx, |w, cx| {
                w.set_focused(true, cx);
            });
        }

        // Bring focused window to front by giving it the highest z-index
        self.window_z_index.insert(id, self.next_z_index);
        self.next_z_index += 1;

        cx.emit(WindowEvent::Focused(id));
        cx.notify();
    }

    /// Close a window
    pub fn close_window(&mut self, id: WindowId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(_) = self.windows.remove(&id) {
            self.window_bounds.remove(&id);
            self.window_z_index.remove(&id);
            if self.focused_window == Some(id) {
                self.focused_window = None;
                // Focus another window if available
                if let Some(&next_id) = self.windows.keys().next() {
                    self.focused_window = Some(next_id);
                    if let Some(window) = self.windows.get(&next_id) {
                        window.update(cx, |w, cx| {
                            w.set_focused(true, cx);
                        });
                    }
                }
            }
            cx.emit(WindowEvent::Closed(id));
            cx.notify();
        }
    }

    /// Minimize a window
    pub fn minimize_window(&mut self, id: WindowId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(window) = self.windows.get(&id) {
            window.update(cx, |w, cx| {
                w.set_minimized(true, cx);
            });
            cx.emit(WindowEvent::MinimizeRequested(id));
            cx.notify();
        }
    }

    /// Maximize/restore a window
    pub fn toggle_maximize_window(&mut self, id: WindowId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(window) = self.windows.get(&id) {
            let is_maximized = window.read(cx).maximized;
            let new_bounds = if is_maximized {
                // Restore to previous size
                window.read(cx).restored_bounds
            } else {
                // Maximize to desktop bounds with some padding
                Bounds {
                    origin: Point { x: px(10.0), y: px(40.0) },
                    size: Size {
                        width: self.desktop_bounds.size.width - px(20.0),
                        height: self.desktop_bounds.size.height - px(50.0),
                    },
                }
            };

            window.update(cx, |w, cx| {
                if !is_maximized {
                    w.restored_bounds = w.bounds;
                }
                w.set_maximized(!is_maximized, cx);
                w.set_bounds(new_bounds, cx);
            });
            self.update_window_bounds(id, new_bounds);

            cx.emit(WindowEvent::MaximizeRequested(id));
            cx.notify();
        }
    }

    /// Get window count for taskbar
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Get list of window titles for task switcher
    pub fn window_titles(&self, cx: &App) -> Vec<(WindowId, String)> {
        self.windows
            .iter()
            .map(|(&id, window)| (id, window.read(cx).title.clone()))
            .collect()
    }

    /// Update cached bounds for a window
    fn update_window_bounds(&mut self, window_id: WindowId, new_bounds: Bounds<Pixels>) {
        self.window_bounds.insert(window_id, new_bounds);
    }

    /// Check if a window is the topmost at a given position
    pub fn is_window_topmost(&self, window_id: WindowId, position: Point<Pixels>) -> bool {
        let window_z_index = self.window_z_index.get(&window_id).copied().unwrap_or(0);

        // Find all windows that contain this position
        let mut overlapping_windows: Vec<(WindowId, i32)> = self.window_bounds.iter()
            .filter_map(|(&id, &bounds)| {
                if bounds.contains(&position) {
                    let z_index = self.window_z_index.get(&id).copied().unwrap_or(0);
                    Some((id, z_index))
                } else {
                    None
                }
            })
            .collect();

        // Sort by z-index (highest first)
        overlapping_windows.sort_by_key(|(_, z_index)| -z_index);

        // Check if this window is the topmost
        overlapping_windows.first().map(|(id, _)| *id == window_id).unwrap_or(false)
    }

    /// Start dragging a window
    fn start_drag(&mut self, window_id: WindowId, mouse_pos: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(&window_bounds) = self.window_bounds.get(&window_id) {
            // Calculate offset from mouse position to window origin using cached bounds
            self.drag_offset = Point {
                x: mouse_pos.x - window_bounds.origin.x,
                y: mouse_pos.y - window_bounds.origin.y,
            };
            self.dragging_window = Some(window_id);
            self.pending_focus_window = Some(window_id);
            cx.notify();
        }
    }

    /// Handle window drag movement
    fn handle_drag_move(&mut self, position: Point<Pixels>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(_window_id) = self.dragging_window {
            // Store the position for processing in the next render
            self.pending_drag_position = Some(position);
            // Calculate snap zone
            self.snap_zone = self.calculate_snap_zone(position);
            cx.notify();
        }
    }

    /// Stop dragging and apply snapping if needed
    fn stop_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(window_id) = self.dragging_window {
            if let Some(snap_zone) = &self.snap_zone {
                self.apply_snap_zone(window_id, snap_zone.clone(), window, cx);
            }
            self.dragging_window = None;
            self.snap_zone = None;
            self.pending_drag_position = None;
            cx.notify();
        }

        // Also stop any resize operation
        if self.resizing_window.is_some() {
            self.resizing_window = None;
            self.resize_handle = None;
            self.pending_resize_data = None;
            cx.notify();
        }
    }

    /// Start resizing a window
    fn start_resize(&mut self, window_id: WindowId, handle: ResizeHandle, cx: &mut Context<Self>) {
        self.resizing_window = Some(window_id);
        self.resize_handle = Some(handle);
        cx.notify();
    }

    /// Handle window resize movement
    fn handle_resize_move(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let (Some(window_id), Some(handle)) = (self.resizing_window, &self.resize_handle) {
            self.pending_resize_data = Some((window_id, position, handle.clone()));
            cx.notify();
        }
    }

    /// Calculate snap zone based on cursor position
    fn calculate_snap_zone(&self, position: Point<Pixels>) -> Option<SnapZone> {
        let bounds = &self.desktop_bounds;
        let edge_threshold = px(50.0);

        // Left edge
        if position.x <= bounds.origin.x + edge_threshold {
            if position.y <= bounds.origin.y + edge_threshold {
                Some(SnapZone::TopLeft)
            } else if position.y >= bounds.bottom() - edge_threshold {
                Some(SnapZone::BottomLeft)
            } else {
                Some(SnapZone::Left)
            }
        }
        // Right edge
        else if position.x >= bounds.right() - edge_threshold {
            if position.y <= bounds.origin.y + edge_threshold {
                Some(SnapZone::TopRight)
            } else if position.y >= bounds.bottom() - edge_threshold {
                Some(SnapZone::BottomRight)
            } else {
                Some(SnapZone::Right)
            }
        }
        // Top edge
        else if position.y <= bounds.origin.y + edge_threshold {
            Some(SnapZone::Top)
        }
        else {
            None
        }
    }

    /// Apply snapping to a window
    fn apply_snap_zone(&mut self, window_id: WindowId, zone: SnapZone, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(window) = self.windows.get(&window_id) {
            let bounds = &self.desktop_bounds;
            let new_bounds = match zone {
                SnapZone::Left => Bounds {
                    origin: Point { x: bounds.origin.x, y: bounds.origin.y + px(40.0) },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: bounds.size.height - px(40.0),
                    },
                },
                SnapZone::Right => Bounds {
                    origin: Point {
                        x: bounds.origin.x + bounds.size.width / 2.0,
                        y: bounds.origin.y + px(40.0)
                    },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: bounds.size.height - px(40.0),
                    },
                },
                SnapZone::Top => Bounds {
                    origin: Point { x: px(10.0), y: px(40.0) },
                    size: Size {
                        width: bounds.size.width - px(20.0),
                        height: bounds.size.height - px(50.0),
                    },
                },
                SnapZone::TopLeft => Bounds {
                    origin: Point { x: bounds.origin.x, y: bounds.origin.y + px(40.0) },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: (bounds.size.height - px(40.0)) / 2.0,
                    },
                },
                SnapZone::TopRight => Bounds {
                    origin: Point {
                        x: bounds.origin.x + bounds.size.width / 2.0,
                        y: bounds.origin.y + px(40.0)
                    },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: (bounds.size.height - px(40.0)) / 2.0,
                    },
                },
                SnapZone::BottomLeft => Bounds {
                    origin: Point {
                        x: bounds.origin.x,
                        y: bounds.origin.y + px(40.0) + (bounds.size.height - px(40.0)) / 2.0
                    },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: (bounds.size.height - px(40.0)) / 2.0,
                    },
                },
                SnapZone::BottomRight => Bounds {
                    origin: Point {
                        x: bounds.origin.x + bounds.size.width / 2.0,
                        y: bounds.origin.y + px(40.0) + (bounds.size.height - px(40.0)) / 2.0
                    },
                    size: Size {
                        width: bounds.size.width / 2.0,
                        height: (bounds.size.height - px(40.0)) / 2.0,
                    },
                },
                SnapZone::Center => return, // No snapping
            };

            window.update(cx, |w, cx| {
                w.restored_bounds = w.bounds;
                w.set_bounds(new_bounds, cx);
            });
            self.update_window_bounds(window_id, new_bounds);
        }
    }
}

impl EventEmitter<WindowEvent> for WindowManager {}

impl Focusable for WindowManager {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WindowManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Process pending window focus to avoid reentrancy
        if let Some(window_id) = self.pending_focus_window.take() {
            self.focus_window(window_id, window, cx);
        }

        // Process pending drag position update to avoid reentrancy
        if let (Some(mouse_position), Some(window_id)) = (self.pending_drag_position.take(), self.dragging_window) {
            if let Some(managed_window) = self.windows.get(&window_id) {
                let new_bounds = {
                    let old_bounds = self.window_bounds.get(&window_id).copied().unwrap_or_default();
                    let mut new_bounds = old_bounds;
                    // Apply the drag offset to maintain relative position
                    new_bounds.origin = Point {
                        x: mouse_position.x - self.drag_offset.x,
                        y: mouse_position.y - self.drag_offset.y,
                    };
                    new_bounds
                };

                // Update both the actual window and cached bounds
                managed_window.update(cx, |w, cx| {
                    w.set_bounds(new_bounds, cx);
                });
                self.update_window_bounds(window_id, new_bounds);

                cx.emit(WindowEvent::Moved { id: window_id, position: new_bounds.origin });
            }
        }

        // Process pending resize update to avoid reentrancy
        if let Some((window_id, mouse_pos, handle)) = self.pending_resize_data.take() {
            if let Some(managed_window) = self.windows.get(&window_id) {
                let new_bounds = {
                    let old_bounds = self.window_bounds.get(&window_id).copied().unwrap_or_default();
                    let mut new_bounds = old_bounds;

                    match handle {
                        ResizeHandle::Right => {
                            new_bounds.size.width = (mouse_pos.x - old_bounds.origin.x).max(px(200.0));
                        },
                        ResizeHandle::Bottom => {
                            new_bounds.size.height = (mouse_pos.y - old_bounds.origin.y).max(px(150.0));
                        },
                        ResizeHandle::BottomRight => {
                            new_bounds.size.width = (mouse_pos.x - old_bounds.origin.x).max(px(200.0));
                            new_bounds.size.height = (mouse_pos.y - old_bounds.origin.y).max(px(150.0));
                        },
                        ResizeHandle::Left => {
                            let new_width = (old_bounds.origin.x + old_bounds.size.width - mouse_pos.x).max(px(200.0));
                            new_bounds.origin.x = old_bounds.origin.x + old_bounds.size.width - new_width;
                            new_bounds.size.width = new_width;
                        },
                        ResizeHandle::Top => {
                            let new_height = (old_bounds.origin.y + old_bounds.size.height - mouse_pos.y).max(px(150.0));
                            new_bounds.origin.y = old_bounds.origin.y + old_bounds.size.height - new_height;
                            new_bounds.size.height = new_height;
                        },
                        ResizeHandle::TopLeft => {
                            let new_width = (old_bounds.origin.x + old_bounds.size.width - mouse_pos.x).max(px(200.0));
                            let new_height = (old_bounds.origin.y + old_bounds.size.height - mouse_pos.y).max(px(150.0));
                            new_bounds.origin.x = old_bounds.origin.x + old_bounds.size.width - new_width;
                            new_bounds.origin.y = old_bounds.origin.y + old_bounds.size.height - new_height;
                            new_bounds.size.width = new_width;
                            new_bounds.size.height = new_height;
                        },
                        ResizeHandle::TopRight => {
                            let new_height = (old_bounds.origin.y + old_bounds.size.height - mouse_pos.y).max(px(150.0));
                            new_bounds.origin.y = old_bounds.origin.y + old_bounds.size.height - new_height;
                            new_bounds.size.width = (mouse_pos.x - old_bounds.origin.x).max(px(200.0));
                            new_bounds.size.height = new_height;
                        },
                        ResizeHandle::BottomLeft => {
                            let new_width = (old_bounds.origin.x + old_bounds.size.width - mouse_pos.x).max(px(200.0));
                            new_bounds.origin.x = old_bounds.origin.x + old_bounds.size.width - new_width;
                            new_bounds.size.width = new_width;
                            new_bounds.size.height = (mouse_pos.y - old_bounds.origin.y).max(px(150.0));
                        },
                    }

                    new_bounds
                };

                // Update both the actual window and cached bounds
                managed_window.update(cx, |w, cx| {
                    w.set_bounds(new_bounds, cx);
                });
                self.update_window_bounds(window_id, new_bounds);

                cx.emit(WindowEvent::Resized { id: window_id, size: new_bounds.size });
            }
        }

        // Sort windows by z-index (higher z-index = on top)
        let mut window_data: Vec<_> = self.windows.iter().map(|(&id, window)| {
            let z_index = self.window_z_index.get(&id).copied().unwrap_or(0);
            (window.clone(), z_index)
        }).collect();
        window_data.sort_by_key(|(_, z_index)| *z_index);
        let windows: Vec<_> = window_data.into_iter().map(|(window, _)| window).collect();

        div()
            .absolute()
            .size_full()
            .children(windows)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                // Check windows in z-index order (highest first) to prevent passthrough
                let mut window_hits: Vec<(WindowId, i32)> = this.window_bounds.iter()
                    .filter_map(|(&id, &bounds)| {
                        if bounds.contains(&event.position) {
                            let z_index = this.window_z_index.get(&id).copied().unwrap_or(0);
                            Some((id, z_index))
                        } else {
                            None
                        }
                    })
                    .collect();

                // Sort by z-index (highest first) and only focus the topmost window
                window_hits.sort_by_key(|(_, z_index)| -z_index);
                if let Some((top_window_id, _)) = window_hits.first() {
                    this.pending_focus_window = Some(*top_window_id);
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if this.dragging_window.is_some() {
                    this.handle_drag_move(event.position, window, cx);
                } else if this.resizing_window.is_some() {
                    this.handle_resize_move(event.position, cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.stop_drag(window, cx);
            }))
    }
}

/// Individual managed window with frame controls
pub struct ManagedWindow {
    pub id: WindowId,
    pub title: String,
    pub content: Option<AnyElement>,
    pub bounds: Bounds<Pixels>,
    pub restored_bounds: Bounds<Pixels>,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
    focus_handle: FocusHandle,
    window_manager: WeakEntity<WindowManager>,
}

impl ManagedWindow {
    pub fn new<V: IntoElement>(
        id: WindowId,
        title: String,
        content: V,
        bounds: Bounds<Pixels>,
        window_manager: WeakEntity<WindowManager>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id,
            title,
            content: Some(content.into_any_element()),
            bounds,
            restored_bounds: bounds,
            minimized: false,
            maximized: false,
            focused: false,
            focus_handle: cx.focus_handle(),
            window_manager,
        }
    }

    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.bounds = bounds;
        cx.notify();
    }

    pub fn set_focused(&mut self, focused: bool, cx: &mut Context<Self>) {
        self.focused = focused;
        cx.notify();
    }

    pub fn set_minimized(&mut self, minimized: bool, cx: &mut Context<Self>) {
        self.minimized = minimized;
        cx.notify();
    }

    pub fn set_maximized(&mut self, maximized: bool, cx: &mut Context<Self>) {
        self.maximized = maximized;
        cx.notify();
    }

    fn render_resize_handles(&self, window_manager: WeakEntity<WindowManager>, cx: &mut Context<Self>) -> impl IntoElement {
        let window_id = self.id;
        let handle_size = px(8.0);
        let corner_size = px(16.0);

        div()
            .absolute()
            .inset_0()
            .children([
                // Corner handles
                div() // Top-left
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(corner_size)
                    .h(corner_size)
                    .cursor_nwse_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                // Check if topmost and start resize in single update call to avoid reentrancy
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::TopLeft, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Top-right
                    .absolute()
                    .right_0()
                    .top_0()
                    .w(corner_size)
                    .h(corner_size)
                    .cursor_nesw_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::TopRight, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Bottom-left
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .w(corner_size)
                    .h(corner_size)
                    .cursor_nesw_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::BottomLeft, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Bottom-right
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .w(corner_size)
                    .h(corner_size)
                    .cursor_nwse_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::BottomRight, cx);
                                    }
                                });
                            }
                        })
                    }),
                // Edge handles
                div() // Top edge
                    .absolute()
                    .left(corner_size)
                    .right(corner_size)
                    .top_0()
                    .h(handle_size)
                    .cursor_ns_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::Top, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Bottom edge
                    .absolute()
                    .left(corner_size)
                    .right(corner_size)
                    .bottom_0()
                    .h(handle_size)
                    .cursor_ns_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::Bottom, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Left edge
                    .absolute()
                    .left_0()
                    .top(corner_size)
                    .bottom(corner_size)
                    .w(handle_size)
                    .cursor_ew_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::Left, cx);
                                    }
                                });
                            }
                        })
                    }),
                div() // Right edge
                    .absolute()
                    .right_0()
                    .top(corner_size)
                    .bottom(corner_size)
                    .w(handle_size)
                    .cursor_ew_resize()
                    .on_mouse_down(MouseButton::Left, {
                        let wm = window_manager.clone();
                        cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                            if let Some(wm) = wm.upgrade() {
                                wm.update(cx, |wm, cx| {
                                    if wm.is_window_topmost(window_id, event.position) {
                                        wm.start_resize(window_id, ResizeHandle::Right, cx);
                                    }
                                });
                            }
                        })
                    }),
            ])
    }

    fn render_title_bar(&self, window_manager: WeakEntity<WindowManager>, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_id = self.id;
        let focused = self.focused;
        let wm1 = window_manager.clone();
        let wm2 = window_manager.clone();
        let wm3 = window_manager.clone();
        let wm4 = window_manager.clone();

        h_flex()
            .w_full()
            .h(px(30.0))
            .bg(if focused { cx.theme().accent } else { cx.theme().muted })
            .border_b_1()
            .border_color(cx.theme().border)
            .px_3()
            .items_center()
            .justify_between()
            .on_mouse_down(MouseButton::Left, cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                if let Some(wm) = wm1.upgrade() {
                    // Check if topmost and start drag in single update call to avoid reentrancy
                    wm.update(cx, |wm, cx| {
                        if wm.is_window_topmost(window_id, event.position) {
                            wm.start_drag(window_id, event.position, cx);
                        }
                    });
                }
            }))
            .child(
                // Window title
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(if focused { cx.theme().accent_foreground } else { cx.theme().muted_foreground })
                    .child(self.title.clone())
            )
            .child(
                // Window controls
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("minimize")
                            .outline()
                            .size(px(28.0))
                            .icon(IconName::Menu)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = wm2.upgrade() {
                                    wm.update(cx, |wm, cx| wm.minimize_window(window_id, window, cx));
                                }
                            }))
                    )
                    .child(
                        Button::new("maximize")
                            .outline()
                            .size(px(28.0))
                            .icon(if self.maximized { IconName::Folder } else { IconName::Settings })
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = wm3.upgrade() {
                                    wm.update(cx, |wm, cx| wm.toggle_maximize_window(window_id, window, cx));
                                }
                            }))
                    )
                    .child(
                        Button::new("close")
                            .outline()
                            .size(px(28.0))
                            .icon(IconName::User)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = wm4.upgrade() {
                                    wm.update(cx, |wm, cx| wm.close_window(window_id, window, cx));
                                }
                            }))
                    )
            )
    }
}

impl Focusable for ManagedWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ManagedWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.minimized {
            return div(); // Hidden when minimized
        }

        let window_manager = self.window_manager.clone();

        div()
            .absolute()
            .w(self.bounds.size.width)
            .h(self.bounds.size.height)
            .left(self.bounds.origin.x)
            .top(self.bounds.origin.y)
            .bg(cx.theme().background)
            .border_1()
            .border_color(if self.focused { cx.theme().primary } else { cx.theme().border })
            .rounded(cx.theme().radius)
            .shadow_xl()
            .when(self.focused, |this| this.shadow_2xl())
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_title_bar(window_manager.clone(), window, cx))
                    .child(
                        // Window content
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.content.take().unwrap_or_else(|| div().into_any_element()))
                    )
            )
            .child(self.render_resize_handles(window_manager, cx))
    }
}