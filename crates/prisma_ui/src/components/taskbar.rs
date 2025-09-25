/// Taskbar component - window switcher and system tray
use gpui::{
    div, img, px, Context, Entity, FocusHandle, Focusable, AppContext,
    IntoElement, ParentElement, Render, Styled, Window, Bounds, Pixels, Animation, AnimationExt,
    InteractiveElement, MouseButton
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, Icon, IconName, StyledExt,
    slider::{Slider, SliderState}, switch::Switch,
};
use chrono::{DateTime, Local};
use std::collections::HashMap;
use crate::{
    window_manager::WindowManager,
    components::{AppMenu, CommandPalette}
};

/// Icon type for tray icons
#[derive(Clone)]
pub enum TrayIconType {
    /// Use a built-in icon from IconName
    Icon(IconName),
    /// Use an image from file path
    Image(String),
}

/// System tray icon
#[derive(Clone)]
pub struct TrayIcon {
    pub id: String,
    pub icon: TrayIconType,
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

/// Tray popup types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrayPopupType {
    Battery,
    Network,
    Volume,
    Clock,
}

/// Tray popup state
#[derive(Clone, Debug)]
pub struct TrayPopupState {
    pub popup_type: TrayPopupType,
    pub is_open: bool,
    pub position: (Pixels, Pixels), // x, y position relative to taskbar
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
    /// Active tray popup
    active_popup: Option<TrayPopupState>,
    /// Volume slider state
    volume_slider: Entity<SliderState>,
    /// Brightness slider state
    brightness_slider: Entity<SliderState>,
    /// WiFi enabled state
    wifi_enabled: bool,
    /// Bluetooth enabled state
    bluetooth_enabled: bool,
    /// Battery percentage
    battery_percentage: f32,
    /// Battery charging state
    battery_charging: bool,
}

impl TrayIcon {
    /// Render the icon based on its type
    fn render_icon(&self) -> impl IntoElement {
        match &self.icon {
            TrayIconType::Icon(icon_name) => Icon::new(icon_name.clone()).size_6().into_any_element(),
            TrayIconType::Image(path) => img(path.clone()).w_6().h_6().into_any_element(),
        }
    }
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
            icon: TrayIconType::Image("icons/wifi/wifi-512.png".to_string()),
            tooltip: "Network: Connected".to_string(),
            badge_count: None,
        });

        tray_icons.insert("battery".to_string(), TrayIcon {
            id: "battery".to_string(),
            icon: TrayIconType::Image("icons/battery/almost-empty-512.png".to_string()),
            tooltip: "Battery: 85%".to_string(),
            badge_count: None,
        });

        tray_icons.insert("sound".to_string(), TrayIcon {
            id: "sound".to_string(),
            icon: TrayIconType::Image("icons/speaker.png".to_string()),
            tooltip: "Volume: 70%".to_string(),
            badge_count: None,
        });

        // Initialize slider states
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(5.)
                .default_value(70.)
        });

        let brightness_slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(5.)
                .default_value(80.)
        });

        tray_icons.insert("notifications".to_string(), TrayIcon {
            id: "notifications".to_string(),
            icon: TrayIconType::Image("icons/inbox.png".to_string()),
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
            active_popup: None,
            volume_slider,
            brightness_slider,
            wifi_enabled: true,
            bluetooth_enabled: true,
            battery_percentage: 85.0,
            battery_charging: false,
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

    /// Toggle tray popup
    pub fn toggle_tray_popup(&mut self, popup_type: TrayPopupType, position: (Pixels, Pixels), cx: &mut Context<Self>) {
        if let Some(ref active) = self.active_popup {
            if active.popup_type == popup_type {
                // Close current popup
                self.active_popup = None;
            } else {
                // Switch to new popup
                self.active_popup = Some(TrayPopupState {
                    popup_type,
                    is_open: true,
                    position,
                });
            }
        } else {
            // Open new popup
            self.active_popup = Some(TrayPopupState {
                popup_type,
                is_open: true,
                position,
            });
        }
        cx.notify();
    }

    /// Close any active popup
    pub fn close_popup(&mut self, cx: &mut Context<Self>) {
        self.active_popup = None;
        cx.notify();
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
            .bg(cx.theme().accent.opacity(0.8))  // Darker semi-transparent background
            //.hover(|this| this.bg(cx.theme().accent))  // Full accent color on hover
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img("icons/menu.png").w_5().h_5())
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
                .children(window_titles.into_iter().enumerate().map(|(idx, (window_id, title))| {
                    Button::new(("window", idx))
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
                                    wm.focus_or_restore_window(window_id, window, cx);
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
            .children(self.tray_icons.values().enumerate().map(|(idx, icon)| {
                let icon_id = icon.id.clone();
                let popup_type = match icon_id.as_str() {
                    "battery" => Some(TrayPopupType::Battery),
                    "network" => Some(TrayPopupType::Network),
                    "sound" => Some(TrayPopupType::Volume),
                    _ => None,
                };

                Button::new(("tray", idx))
                    .ghost()
                    .compact()
                    .relative()
                    .child(icon.render_icon())
                    .when_some(icon.badge_count, |this, count| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .size_4()
                                .bg(cx.theme().danger)
                                .text_color(cx.theme().danger_foreground)
                                .rounded_full()
                                .text_xs()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if count > 9 { "9+".to_string() } else { count.to_string() })
                        )
                    })
                    .tooltip(&icon.tooltip)
                    .when_some(popup_type, |this, popup_type| {
                        this.on_click(cx.listener(move |taskbar, _, _, cx| {
                            // Calculate popup position (above the taskbar icon)
                            let position = (px(300.0 + (idx as f32 * 40.0)), px(60.0));
                            taskbar.toggle_tray_popup(popup_type, position, cx);
                        }))
                    })
            }))
    }

    fn render_battery_tray(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(280.0))
            .bg(cx.theme().background.opacity(0.95))
            .border_1()
            .border_color(cx.theme().border.opacity(0.3))
            .rounded_lg()
            .shadow_xl()
            .p_4()
            .gap_3()
            .child(
                // Header
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Bot).size_5())
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child("Battery")
                            )
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(if self.battery_percentage > 20.0 { cx.theme().foreground } else { cx.theme().danger })
                            .child(format!("{}%", self.battery_percentage as i32))
                    )
            )
            .child(
                // Battery status
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .w_full()
                            .h(px(12.0))
                            .bg(cx.theme().muted.opacity(0.3))
                            .rounded_full()
                            .child(
                                div()
                                    .w(px((self.battery_percentage / 100.0) * 248.0))
                                    .h_full()
                                    .bg(if self.battery_percentage > 20.0 { cx.theme().success } else { cx.theme().danger })
                                    .rounded_full()
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(if self.battery_charging {
                                "⚡ Charging - 2h 30m until full"
                            } else {
                                "🔋 5h 20m remaining"
                            })
                    )
            )
            .child(
                // Brightness control
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::Sun).size_4())
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .text_color(cx.theme().foreground)
                                            .child("Brightness")
                                    )
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("80%")
                            )
                    )
                    .child(
                        Slider::new(&self.brightness_slider)
                            .w_full()
                    )
            )
            .child(
                // Power modes
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child("Power Mode")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("power-saver")
                                    .ghost()
                                    .compact()
                                    .child("💾 Power Saver")
                            )
                            .child(
                                Button::new("balanced")
                                    .primary()
                                    .compact()
                                    .child("⚖️ Balanced")
                            )
                            .child(
                                Button::new("performance")
                                    .ghost()
                                    .compact()
                                    .child("🚀 Performance")
                            )
                    )
            )
    }

    fn render_network_tray(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(320.0))
            .bg(cx.theme().background.opacity(0.95))
            .border_1()
            .border_color(cx.theme().border.opacity(0.3))
            .rounded_lg()
            .shadow_xl()
            .p_4()
            .gap_3()
            .child(
                // Header
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Globe).size_5())
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child("Network & Internet")
                            )
                    )
            )
            .child(
                // WiFi Section
                v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::Globe).size_4())
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .text_color(cx.theme().foreground)
                                            .child("Wi-Fi")
                                    )
                            )
                            .child(
                                Switch::new("wifi")
                                    .checked(self.wifi_enabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        // Toggle would be handled here
                                        cx.notify();
                                    }))
                            )
                    )
                    .when(self.wifi_enabled, |this| {
                        this.child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Connected to: Home Network")
                                )
                                .child(
                                    Button::new("wifi-settings")
                                        .ghost()
                                        .compact()
                                        .w_full()
                                        .child("WiFi Settings")
                                )
                        )
                    })
            )
            .child(
                // Bluetooth Section
                v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::CircleCheck).size_4())
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .text_color(cx.theme().foreground)
                                            .child("Bluetooth")
                                    )
                            )
                            .child(
                                Switch::new("bluetooth")
                                    .checked(self.bluetooth_enabled)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        // Toggle would be handled here
                                        cx.notify();
                                    }))
                            )
                    )
                    .when(self.bluetooth_enabled, |this| {
                        this.child(
                            Button::new("bluetooth-settings")
                                .ghost()
                                .compact()
                                .w_full()
                                .child("Bluetooth Settings")
                        )
                    })
            )
    }

    fn render_volume_tray(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(280.0))
            .bg(cx.theme().background.opacity(0.95))
            .border_1()
            .border_color(cx.theme().border.opacity(0.3))
            .rounded_lg()
            .shadow_xl()
            .p_4()
            .gap_3()
            .child(
                // Header
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::SquareTerminal).size_5())
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child("Volume")
                            )
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child("70%")
                    )
            )
            .child(
                // Volume slider
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(Icon::new(IconName::Search).size_4())
                            .child(
                                Slider::new(&self.volume_slider)
                                    .flex_1()
                            )
                            .child(Icon::new(IconName::SquareTerminal).size_4())
                    )
            )
            .child(
                // Quick audio settings
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child("Audio Devices")
                    )
                    .child(
                        Button::new("speakers")
                            .ghost()
                            .justify_start()
                            .w_full()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::SquareTerminal).size_4())
                                    .child("Speakers (Active)")
                            )
                    )
                    .child(
                        Button::new("headphones")
                            .ghost()
                            .justify_start()
                            .w_full()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::Search).size_4())
                                    .child("Headphones")
                            )
                    )
            )
            .child(
                // Sound settings button
                Button::new("sound-settings")
                    .ghost()
                    .w_full()
                    .child("Sound Settings")
            )
    }

    fn render_tray_popup(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        self.active_popup.as_ref().map(|popup| {
            let content = match popup.popup_type {
                TrayPopupType::Battery => self.render_battery_tray(cx).into_any_element(),
                TrayPopupType::Network => self.render_network_tray(cx).into_any_element(),
                TrayPopupType::Volume => self.render_volume_tray(cx).into_any_element(),
                TrayPopupType::Clock => div().child("Clock settings coming soon...").into_any_element(),
            };

            // Full-screen overlay for click-outside-to-close
            div()
                .absolute()
                .inset_0()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.active_popup = None;
                    cx.notify();
                }))
                .child(
                    div()
                        .absolute()
                        .right(px(16.0))
                        .bottom(px(64.0)) // Above taskbar (48px) + margin (16px)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            // Prevent popup from closing when clicking inside it
                            cx.stop_propagation();
                        })
                        .child(content)
                        .with_animation(
                            "slide-up",
                            Animation::new(std::time::Duration::from_millis(200))
                                .with_easing(gpui_component::animation::cubic_bezier(0.4, 0.0, 0.2, 1.0)),
                            |this, delta| {
                                let y_offset = px(20.) * (1.0 - delta);
                                this.bottom(px(64.0) + y_offset)
                            }
                        )
                )
        })
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
            .child(img("icons/search.png").w_4().h_4())
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

        div()
            .relative()
            .w_full()
            .h_full()
            .child(
                match self.position {
            TaskbarPosition::Bottom => {
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .w_full()
                    .h(taskbar_height)
                    .bg(cx.theme().sidebar.opacity(0.95))
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
            )
            .children(self.render_tray_popup(cx))
    }
}