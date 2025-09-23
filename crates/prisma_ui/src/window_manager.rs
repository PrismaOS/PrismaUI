/// Core window management system with GPU-backed compositing
use gpui::{
    div, px, size, AnyElement, App, AppContext, Bounds, Context,
    Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point, Render, Size, Styled, WeakEntity, Window
};
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, Icon, IconName, StyledExt
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
            focused_window: None,
            desktop_bounds,
            dragging_window: None,
            resizing_window: None,
            resize_handle: None,
            snap_zone: None,
            focus_handle: cx.focus_handle(),
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

        let managed_window = cx.new(|cx| ManagedWindow::new(
            id,
            title,
            content.into(),
            bounds,
            cx,
        ));

        self.windows.insert(id, managed_window);
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

        cx.emit(WindowEvent::Focused(id));
        cx.notify();
    }

    /// Close a window
    pub fn close_window(&mut self, id: WindowId, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(_) = self.windows.remove(&id) {
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

    /// Start dragging a window
    fn start_drag(&mut self, window_id: WindowId, window: &mut Window, cx: &mut Context<Self>) {
        self.dragging_window = Some(window_id);
        self.focus_window(window_id, window, cx);
    }

    /// Handle window drag movement
    fn handle_drag_move(&mut self, position: Point<Pixels>, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(window_id) = self.dragging_window {
            if let Some(window) = self.windows.get(&window_id) {
                window.update(cx, |w, cx| {
                    let mut new_bounds = w.bounds;
                    new_bounds.origin = position;
                    w.set_bounds(new_bounds, cx);
                });

                // Calculate snap zone
                self.snap_zone = self.calculate_snap_zone(position);
                cx.emit(WindowEvent::Moved { id: window_id, position });
                cx.notify();
            }
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
        let windows: Vec<_> = self.windows.values().cloned().collect();

        div()
            .absolute()
            .size_full()
            .children(windows)
            .on_mouse_down(MouseDownEvent::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                // Check if clicking on a window
                for (&id, managed_window) in &this.windows {
                    let bounds = managed_window.read(cx).bounds;
                    if bounds.contains(&event.position) {
                        this.focus_window(id, window, cx);
                        break;
                    }
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.handle_drag_move(event.position, window, cx);
            }))
            .on_mouse_up(MouseUpEvent::Left, cx.listener(|this, _: &MouseUpEvent, window, cx| {
                this.stop_drag(window, cx);
            }))
    }
}

/// Individual managed window with frame controls
pub struct ManagedWindow {
    pub id: WindowId,
    pub title: String,
    pub content: AnyElement,
    pub bounds: Bounds<Pixels>,
    pub restored_bounds: Bounds<Pixels>,
    pub minimized: bool,
    pub maximized: bool,
    pub focused: bool,
    focus_handle: FocusHandle,
}

impl ManagedWindow {
    pub fn new<V: IntoElement>(
        id: WindowId,
        title: String,
        content: V,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id,
            title,
            content: content.into_any_element(),
            bounds,
            restored_bounds: bounds,
            minimized: false,
            maximized: false,
            focused: false,
            focus_handle: cx.focus_handle(),
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

    fn render_title_bar(&self, window_manager: WeakEntity<WindowManager>, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_id = self.id;
        let focused = self.focused;

        h_flex()
            .w_full()
            .h(px(30.0))
            .bg(if focused { cx.theme().accent } else { cx.theme().muted })
            .border_b_1()
            .border_color(cx.theme().border)
            .px_3()
            .items_center()
            .justify_between()
            .on_mouse_down(MouseDownEvent::Left, cx.listener(move |_, _, window, cx| {
                if let Some(wm) = window_manager.upgrade() {
                    wm.update(cx, |wm, cx| wm.start_drag(window_id, window, cx));
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
                    .gap_1()
                    .child(
                        Button::new("minimize")
                            .ghost()
                            .compact()
                            .icon(IconName::Minus)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = window_manager.upgrade() {
                                    wm.update(cx, |wm, cx| wm.minimize_window(window_id, window, cx));
                                }
                            }))
                    )
                    .child(
                        Button::new("maximize")
                            .ghost()
                            .compact()
                            .icon(if self.maximized { IconName::Minimize } else { IconName::Maximize })
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = window_manager.upgrade() {
                                    wm.update(cx, |wm, cx| wm.toggle_maximize_window(window_id, window, cx));
                                }
                            }))
                    )
                    .child(
                        Button::new("close")
                            .ghost()
                            .compact()
                            .icon(IconName::Close)
                            .on_click(cx.listener(move |_, _, window, cx| {
                                if let Some(wm) = window_manager.upgrade() {
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

        let window_manager = cx.view::<WindowManager>().downgrade();

        div()
            .absolute()
            .size(self.bounds.size)
            .origin(self.bounds.origin)
            .bg(cx.theme().background)
            .border_1()
            .border_color(if self.focused { cx.theme().primary } else { cx.theme().border })
            .rounded(cx.theme().radius)
            .shadow_xl()
            .when(self.focused, |this| this.shadow_2xl())
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_title_bar(window_manager, window, cx))
                    .child(
                        // Window content
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(self.content.clone())
                    )
            )
    }
}