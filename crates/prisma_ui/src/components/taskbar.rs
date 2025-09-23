/// Taskbar component - window switcher and system tray
use gpui::{
    div, px, Context, Entity, FocusHandle, Focusable, AppContext,
    IntoElement, ParentElement, Render, Styled, Window, Bounds, Pixels
};
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, Icon, IconName, StyledExt
};
use chrono::{DateTime, Local};
use std::collections::HashMap;
use crate::{
    window_manager::WindowManager,
    components::{AppMenu, CommandPalette}
};

/// System tray icon
#[derive(Clone)]
pub struct TrayIcon {
    pub id: String,
    pub icon: IconName,
    pub tooltip: String,
    pub badge_count: Option<u32>,
}

/// Taskbar positioning
#[derive(Clone, Debug, PartialEq)]
pub enum TaskbarPosition {
    Bottom,
    Top,
    Left,
    Right,
}

/// Main taskbar component managing system navigation and window switching
pub struct Taskbar {
    /// Taskbar position on screen
    position: TaskbarPosition,
    /// Desktop bounds for positioning
    bounds: Bounds<Pixels>,
    /// App menu reference
    app_menu: Entity<AppMenu>,
    /// Command palette reference
    command_palette: Entity<CommandPalette>,
    /// Window manager reference for window switching
    window_manager: Option<Entity<WindowManager>>,
    /// System tray icons
    tray_icons: HashMap<String, TrayIcon>,
    /// Current time for clock
    current_time: DateTime<Local>,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl Taskbar {
    pub fn new(
        bounds: Bounds<Pixels>,
        app_menu: Entity<AppMenu>,
        command_palette: Entity<CommandPalette>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut tray_icons = HashMap::new();

        // Add default system tray icons
        tray_icons.insert("network".to_string(), TrayIcon {
            id: "network".to_string(),
            icon: IconName::Globe,
            tooltip: "Network: Connected".to_string(),
            badge_count: None,
        });

        tray_icons.insert("battery".to_string(), TrayIcon {
            id: "battery".to_string(),
            icon: IconName::SquareTerminal,
            tooltip: "Battery: 85%".to_string(),
            badge_count: None,
        });

        tray_icons.insert("sound".to_string(), TrayIcon {
            id: "sound".to_string(),
            icon: IconName::Settings,
            tooltip: "Volume: 70%".to_string(),
            badge_count: None,
        });

        tray_icons.insert("notifications".to_string(), TrayIcon {
            id: "notifications".to_string(),
            icon: IconName::Bell,
            tooltip: "Notifications".to_string(),
            badge_count: Some(3),
        });

        Self {
            position: TaskbarPosition::Bottom,
            bounds,
            app_menu,
            command_palette,
            window_manager: None,
            tray_icons,
            current_time: Local::now(),
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create taskbar entity
    pub fn create(
        bounds: Bounds<Pixels>,
        app_menu: Entity<AppMenu>,
        command_palette: Entity<CommandPalette>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(bounds, app_menu, command_palette, window, cx))
    }

    /// Set window manager reference for window switching
    pub fn set_window_manager(&mut self, window_manager: Entity<WindowManager>, cx: &mut Context<Self>) {
        self.window_manager = Some(window_manager);
        cx.notify();
    }

    /// Update taskbar bounds (on screen resolution change)
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.bounds = bounds;
        cx.notify();
    }

    /// Update system time
    pub fn update_time(&mut self, cx: &mut Context<Self>) {
        self.current_time = Local::now();
        cx.notify();
    }

    /// Add system tray icon
    pub fn add_tray_icon(&mut self, icon: TrayIcon, cx: &mut Context<Self>) {
        self.tray_icons.insert(icon.id.clone(), icon);
        cx.notify();
    }

    /// Remove system tray icon
    pub fn remove_tray_icon(&mut self, id: &str, cx: &mut Context<Self>) {
        self.tray_icons.remove(id);
        cx.notify();
    }

    /// Update tray icon badge count
    pub fn update_tray_badge(&mut self, id: &str, badge_count: Option<u32>, cx: &mut Context<Self>) {
        if let Some(icon) = self.tray_icons.get_mut(id) {
            icon.badge_count = badge_count;
            cx.notify();
        }
    }

    /// Get taskbar height based on position
    pub fn height(&self) -> Pixels {
        match self.position {
            TaskbarPosition::Bottom | TaskbarPosition::Top => px(48.0),
            TaskbarPosition::Left | TaskbarPosition::Right => px(64.0),
        }
    }

    /// Get available desktop area (screen minus taskbar)
    pub fn desktop_area(&self) -> Bounds<Pixels> {
        match self.position {
            TaskbarPosition::Bottom => Bounds {
                origin: self.bounds.origin,
                size: gpui::Size {
                    width: self.bounds.size.width,
                    height: self.bounds.size.height - self.height(),
                },
            },
            TaskbarPosition::Top => Bounds {
                origin: gpui::Point {
                    x: self.bounds.origin.x,
                    y: self.bounds.origin.y + self.height(),
                },
                size: gpui::Size {
                    width: self.bounds.size.width,
                    height: self.bounds.size.height - self.height(),
                },
            },
            TaskbarPosition::Left => Bounds {
                origin: gpui::Point {
                    x: self.bounds.origin.x + self.height(),
                    y: self.bounds.origin.y,
                },
                size: gpui::Size {
                    width: self.bounds.size.width - self.height(),
                    height: self.bounds.size.height,
                },
            },
            TaskbarPosition::Right => Bounds {
                origin: self.bounds.origin,
                size: gpui::Size {
                    width: self.bounds.size.width - self.height(),
                    height: self.bounds.size.height,
                },
            },
        }
    }

    fn render_start_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("start-button")
            .primary()
            .size(px(40.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::Menu).size_5())
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.app_menu.update(cx, |menu, cx| {
                    menu.toggle(window, cx);
                });
            }))
    }

    fn render_window_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(window_manager) = &self.window_manager {
            let window_titles = window_manager.read(cx).window_titles(cx);

            h_flex()
                .gap_1()
                .children(window_titles.into_iter().map(|(window_id, title)| {
                    Button::new(format!("window-{}", window_id))
                        .ghost()
                        .compact()
                        .max_w(px(200.0))
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size_4()
                                        .bg(cx.theme().primary)
                                        .rounded_full()
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .truncate()
                                        .child(title)
                                )
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(wm) = &this.window_manager {
                                wm.update(cx, |wm, cx| {
                                    wm.focus_window(window_id, window, cx);
                                });
                            }
                        }))
                }))
        } else {
            h_flex()
        }
    }

    fn render_system_tray(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .children(self.tray_icons.values().map(|icon| {
                Button::new(format!("tray-{}", icon.id))
                    .ghost()
                    .compact()
                    .relative()
                    .child(Icon::new(icon.icon).size_4())
                    .when_some(icon.badge_count, |this, count| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .size_4()
                                .bg(cx.theme().destructive)
                                .text_color(cx.theme().destructive_foreground)
                                .rounded_full()
                                .text_xs()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if count > 9 { "9+".to_string() } else { count.to_string() })
                        )
                    })
                    .tooltip(&icon.tooltip)
            }))
    }

    fn render_clock(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .justify_center()
            .px_3()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(self.current_time.format("%H:%M").to_string())
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.current_time.format("%m/%d").to_string())
            )
    }

    fn render_search_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("search-button")
            .ghost()
            .size(px(40.0))
            .child(Icon::new(IconName::Search).size_4())
            .tooltip("Search (Ctrl+Space)")
            .on_click(cx.listener(|this, _, window, cx| {
                this.command_palette.update(cx, |palette, cx| {
                    palette.toggle(window, cx);
                });
            }))
    }
}

impl Focusable for Taskbar {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Taskbar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let taskbar_height = self.height();

        match self.position {
            TaskbarPosition::Bottom => {
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .w_full()
                    .h(taskbar_height)
                    .bg(cx.theme().sidebar.opacity(0.95))
                    .backdrop_blur_sm()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .size_full()
                            .items_center()
                            .px_2()
                            .gap_2()
                            // Left section - Start button and window buttons
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(self.render_start_button(cx))
                                    .child(self.render_search_button(cx))
                                    .child(
                                        div()
                                            .w_px()
                                            .h_6()
                                            .bg(cx.theme().border)
                                            .mx_2()
                                    )
                                    .child(self.render_window_buttons(cx))
                            )
                            // Right section - System tray and clock
                            .child(
                                h_flex()
                                    .ml_auto()
                                    .items_center()
                                    .gap_2()
                                    .child(self.render_system_tray(cx))
                                    .child(
                                        div()
                                            .w_px()
                                            .h_6()
                                            .bg(cx.theme().border)
                                            .mx_2()
                                    )
                                    .child(self.render_clock(cx))
                            )
                    )
            }
            // TODO: Implement other positions
            _ => {
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .w_full()
                    .h(taskbar_height)
                    .bg(cx.theme().sidebar)
                    .child("Taskbar (other positions not implemented yet)")
            }
        }
    }
}