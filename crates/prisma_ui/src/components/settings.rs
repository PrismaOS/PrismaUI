/// Settings application - comprehensive system preferences
use gpui::{
    div, img, px, Action, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, AppContext
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, input::{InputEvent, InputState, TextInput},
    ActiveTheme, Icon, IconName, StyledExt, Selectable
};
use serde::Deserialize;
use std::collections::HashMap;

/// Settings categories
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SettingsCategory {
    General,
    Appearance,
    Desktop,
    Sound,
    Network,
    Bluetooth,
    Display,
    Keyboard,
    Mouse,
    Privacy,
    Security,
    Users,
    DateTime,
    Language,
    Accessibility,
    Updates,
    About,
}

impl SettingsCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Desktop => "Desktop & Dock",
            Self::Sound => "Sound",
            Self::Network => "Network",
            Self::Bluetooth => "Bluetooth",
            Self::Display => "Display",
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse & Trackpad",
            Self::Privacy => "Privacy & Security",
            Self::Security => "Security",
            Self::Users => "Users & Groups",
            Self::DateTime => "Date & Time",
            Self::Language => "Language & Region",
            Self::Accessibility => "Accessibility",
            Self::Updates => "Software Update",
            Self::About => "About This System",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::General => IconName::Settings,
            Self::Appearance => IconName::Palette,
            Self::Desktop => IconName::LayoutDashboard,
            Self::Sound => IconName::Heart, // Using available icon as placeholder
            Self::Network => IconName::Globe,
            Self::Bluetooth => IconName::Globe,
            Self::Display => IconName::LayoutDashboard,
            Self::Keyboard => IconName::SquareTerminal,
            Self::Mouse => IconName::CircleUser,
            Self::Privacy => IconName::Eye,
            Self::Security => IconName::EyeOff,
            Self::Users => IconName::User,
            Self::DateTime => IconName::Calendar,
            Self::Language => IconName::Globe,
            Self::Accessibility => IconName::Eye,
            Self::Updates => IconName::ArrowDown,
            Self::About => IconName::Info,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::General => "Basic system preferences and startup options",
            Self::Appearance => "Theme, colors, and visual preferences",
            Self::Desktop => "Wallpaper, dock position, and desktop settings",
            Self::Sound => "Audio output, input, and sound effects",
            Self::Network => "Wi-Fi, Ethernet, and network connections",
            Self::Bluetooth => "Bluetooth devices and connections",
            Self::Display => "Resolution, scaling, and multiple displays",
            Self::Keyboard => "Key repeat, shortcuts, and input methods",
            Self::Mouse => "Cursor speed, scrolling, and gestures",
            Self::Privacy => "App permissions and data protection",
            Self::Security => "Passwords, encryption, and security policies",
            Self::Users => "User accounts and permissions",
            Self::DateTime => "Date, time, and timezone settings",
            Self::Language => "Language, region, and localization",
            Self::Accessibility => "Visual, hearing, and motor accessibility",
            Self::Updates => "System updates and automatic installation",
            Self::About => "System information and specifications",
        }
    }

    pub fn icon_image(&self) -> Option<&str> {
        match self {
            Self::General => Some("icons/settings.png"),
            Self::Appearance => Some("icons/palette.png"),
            Self::Desktop => Some("icons/monitor.png"),
            Self::Sound => Some("icons/speaker.png"),
            Self::Network => Some("icons/wifi/wifi-512.png"),
            Self::Bluetooth => Some("icons/bluetooth.png"),
            Self::Display => Some("icons/display.png"),
            Self::Keyboard => Some("icons/keyboard.png"),
            Self::Mouse => Some("icons/mouse.png"),
            Self::Privacy => Some("icons/shield.png"),
            Self::Security => Some("icons/lock-512.png"),
            Self::Users => Some("icons/users.png"),
            Self::DateTime => Some("icons/clock.png"),
            Self::Language => Some("icons/globe.png"),
            Self::Accessibility => Some("icons/accessibility.png"),
            Self::Updates => Some("icons/download.png"),
            Self::About => Some("icons/info.png"),
        }
    }
}

/// Settings control types
#[derive(Clone, Debug)]
pub enum SettingControl {
    Toggle {
        label: String,
        description: String,
        value: bool,
        key: String,
    },
    Slider {
        label: String,
        description: String,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        key: String,
    },
    Dropdown {
        label: String,
        description: String,
        value: String,
        options: Vec<(String, String)>, // (key, display)
        key: String,
    },
    TextInput {
        label: String,
        description: String,
        value: String,
        placeholder: String,
        key: String,
    },
    Button {
        label: String,
        description: String,
        button_text: String,
        action: String,
    },
    Info {
        label: String,
        value: String,
    },
    Section {
        title: String,
    },
}

/// Settings actions
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = settings, no_json)]
pub enum SettingsAction {
    SetCategory(String),
    UpdateSetting(String, String), // key, value
    PerformAction(String),
    ResetToDefaults,
    ImportSettings,
    ExportSettings,
}

/// Settings application
pub struct Settings {
    /// Current active category
    active_category: SettingsCategory,
    /// Available categories
    categories: Vec<SettingsCategory>,
    /// Settings data
    settings_data: HashMap<String, String>,
    /// Search input state
    search_input: Entity<InputState>,
    /// Search query
    search_query: String,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search settings...")
        });

        // Subscribe to search input changes
        cx.subscribe(&search_input, |this, _, event, cx| {
            match event {
                InputEvent::Change => {
                    this.search_query = this.search_input.read(cx).value().to_string();
                    cx.notify();
                }
                _ => {}
            }
        }).detach();

        let categories = vec![
            SettingsCategory::General,
            SettingsCategory::Appearance,
            SettingsCategory::Desktop,
            SettingsCategory::Sound,
            SettingsCategory::Network,
            SettingsCategory::Bluetooth,
            SettingsCategory::Display,
            SettingsCategory::Keyboard,
            SettingsCategory::Mouse,
            SettingsCategory::Privacy,
            SettingsCategory::Security,
            SettingsCategory::Users,
            SettingsCategory::DateTime,
            SettingsCategory::Language,
            SettingsCategory::Accessibility,
            SettingsCategory::Updates,
            SettingsCategory::About,
        ];

        // Initialize default settings
        let mut settings_data = HashMap::new();
        settings_data.insert("startup_sound".to_string(), "true".to_string());
        settings_data.insert("dark_mode".to_string(), "false".to_string());
        settings_data.insert("wallpaper".to_string(), "default.jpg".to_string());
        settings_data.insert("dock_position".to_string(), "bottom".to_string());
        settings_data.insert("dock_autohide".to_string(), "false".to_string());
        settings_data.insert("volume_master".to_string(), "0.7".to_string());
        settings_data.insert("volume_alerts".to_string(), "0.5".to_string());
        settings_data.insert("wifi_enabled".to_string(), "true".to_string());
        settings_data.insert("bluetooth_enabled".to_string(), "true".to_string());
        settings_data.insert("display_resolution".to_string(), "1920x1080".to_string());
        settings_data.insert("display_scaling".to_string(), "100".to_string());
        settings_data.insert("key_repeat_delay".to_string(), "500".to_string());
        settings_data.insert("key_repeat_rate".to_string(), "30".to_string());
        settings_data.insert("mouse_speed".to_string(), "0.5".to_string());
        settings_data.insert("scroll_direction".to_string(), "natural".to_string());
        settings_data.insert("auto_updates".to_string(), "true".to_string());
        settings_data.insert("system_name".to_string(), "PrismaUI System".to_string());
        settings_data.insert("user_name".to_string(), "User".to_string());

        Self {
            active_category: SettingsCategory::General,
            categories,
            settings_data,
            search_input,
            search_query: String::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create settings entity
    pub fn create(window: &mut Window, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// Set active category
    pub fn set_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.active_category = category;
        cx.notify();
    }

    /// Update a setting value
    pub fn update_setting(&mut self, key: &str, value: &str, cx: &mut Context<Self>) {
        self.settings_data.insert(key.to_string(), value.to_string());
        tracing::info!("Updated setting: {} = {}", key, value);
        cx.notify();
    }

    /// Get setting value
    pub fn get_setting(&self, key: &str) -> Option<&String> {
        self.settings_data.get(key)
    }

    /// Get settings for current category
    fn get_category_settings(&self) -> Vec<SettingControl> {
        match self.active_category {
            SettingsCategory::General => vec![
                SettingControl::Section {
                    title: "Startup".to_string(),
                },
                SettingControl::Toggle {
                    label: "Play sound on startup".to_string(),
                    description: "Play the startup sound when the system boots".to_string(),
                    value: self.get_setting("startup_sound").unwrap_or(&"true".to_string()) == "true",
                    key: "startup_sound".to_string(),
                },
                SettingControl::Toggle {
                    label: "Show login items".to_string(),
                    description: "Show applications that start automatically at login".to_string(),
                    value: self.get_setting("show_login_items").unwrap_or(&"true".to_string()) == "true",
                    key: "show_login_items".to_string(),
                },
                SettingControl::Section {
                    title: "System Information".to_string(),
                },
                SettingControl::Info {
                    label: "System Name".to_string(),
                    value: self.get_setting("system_name").unwrap_or(&"PrismaUI System".to_string()).clone(),
                },
                SettingControl::TextInput {
                    label: "Computer Name".to_string(),
                    description: "This name is used to identify your computer on the network".to_string(),
                    value: self.get_setting("computer_name").unwrap_or(&"PrismaUI".to_string()).clone(),
                    placeholder: "Enter computer name".to_string(),
                    key: "computer_name".to_string(),
                },
            ],
            SettingsCategory::Appearance => vec![
                SettingControl::Section {
                    title: "Theme".to_string(),
                },
                SettingControl::Toggle {
                    label: "Dark Mode".to_string(),
                    description: "Use dark colors for windows, menus, and controls".to_string(),
                    value: self.get_setting("dark_mode").unwrap_or(&"false".to_string()) == "true",
                    key: "dark_mode".to_string(),
                },
                SettingControl::Dropdown {
                    label: "Accent Color".to_string(),
                    description: "Choose an accent color for buttons and controls".to_string(),
                    value: self.get_setting("accent_color").unwrap_or(&"blue".to_string()).clone(),
                    options: vec![
                        ("blue".to_string(), "Blue".to_string()),
                        ("purple".to_string(), "Purple".to_string()),
                        ("pink".to_string(), "Pink".to_string()),
                        ("red".to_string(), "Red".to_string()),
                        ("orange".to_string(), "Orange".to_string()),
                        ("yellow".to_string(), "Yellow".to_string()),
                        ("green".to_string(), "Green".to_string()),
                        ("graphite".to_string(), "Graphite".to_string()),
                    ],
                    key: "accent_color".to_string(),
                },
                SettingControl::Section {
                    title: "Window Appearance".to_string(),
                },
                SettingControl::Toggle {
                    label: "Reduce transparency".to_string(),
                    description: "Reduce the transparency of windows and menus".to_string(),
                    value: self.get_setting("reduce_transparency").unwrap_or(&"false".to_string()) == "true",
                    key: "reduce_transparency".to_string(),
                },
                SettingControl::Toggle {
                    label: "Show scroll bars".to_string(),
                    description: "Always show scroll bars in windows".to_string(),
                    value: self.get_setting("show_scroll_bars").unwrap_or(&"true".to_string()) == "true",
                    key: "show_scroll_bars".to_string(),
                },
            ],
            SettingsCategory::Desktop => vec![
                SettingControl::Section {
                    title: "Wallpaper".to_string(),
                },
                SettingControl::Dropdown {
                    label: "Wallpaper".to_string(),
                    description: "Choose a wallpaper for your desktop".to_string(),
                    value: self.get_setting("wallpaper").unwrap_or(&"default.jpg".to_string()).clone(),
                    options: vec![
                        ("default.jpg".to_string(), "Default".to_string()),
                        ("nature1.jpg".to_string(), "Nature 1".to_string()),
                        ("nature2.jpg".to_string(), "Nature 2".to_string()),
                        ("abstract1.jpg".to_string(), "Abstract 1".to_string()),
                        ("solid_color".to_string(), "Solid Color".to_string()),
                    ],
                    key: "wallpaper".to_string(),
                },
                SettingControl::Button {
                    label: "Choose Custom Wallpaper".to_string(),
                    description: "Select a custom image file for your wallpaper".to_string(),
                    button_text: "Choose File...".to_string(),
                    action: "choose_wallpaper".to_string(),
                },
                SettingControl::Section {
                    title: "Dock".to_string(),
                },
                SettingControl::Dropdown {
                    label: "Position on screen".to_string(),
                    description: "Choose where the dock appears on your screen".to_string(),
                    value: self.get_setting("dock_position").unwrap_or(&"bottom".to_string()).clone(),
                    options: vec![
                        ("bottom".to_string(), "Bottom".to_string()),
                        ("left".to_string(), "Left".to_string()),
                        ("right".to_string(), "Right".to_string()),
                    ],
                    key: "dock_position".to_string(),
                },
                SettingControl::Toggle {
                    label: "Automatically hide and show the Dock".to_string(),
                    description: "The dock will hide when not in use to give you more screen space".to_string(),
                    value: self.get_setting("dock_autohide").unwrap_or(&"false".to_string()) == "true",
                    key: "dock_autohide".to_string(),
                },
                SettingControl::Slider {
                    label: "Size".to_string(),
                    description: "Adjust the size of the dock".to_string(),
                    value: self.get_setting("dock_size").unwrap_or(&"0.5".to_string()).parse().unwrap_or(0.5),
                    min: 0.1,
                    max: 1.0,
                    step: 0.1,
                    key: "dock_size".to_string(),
                },
            ],
            SettingsCategory::Sound => vec![
                SettingControl::Section {
                    title: "Output".to_string(),
                },
                SettingControl::Slider {
                    label: "Master Volume".to_string(),
                    description: "Adjust the overall system volume".to_string(),
                    value: self.get_setting("volume_master").unwrap_or(&"0.7".to_string()).parse().unwrap_or(0.7),
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    key: "volume_master".to_string(),
                },
                SettingControl::Toggle {
                    label: "Mute".to_string(),
                    description: "Mute all sound output".to_string(),
                    value: self.get_setting("volume_muted").unwrap_or(&"false".to_string()) == "true",
                    key: "volume_muted".to_string(),
                },
                SettingControl::Section {
                    title: "Sound Effects".to_string(),
                },
                SettingControl::Slider {
                    label: "Alert Volume".to_string(),
                    description: "Volume for system alerts and notifications".to_string(),
                    value: self.get_setting("volume_alerts").unwrap_or(&"0.5".to_string()).parse().unwrap_or(0.5),
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                    key: "volume_alerts".to_string(),
                },
                SettingControl::Toggle {
                    label: "Play user interface sound effects".to_string(),
                    description: "Play sounds for buttons, menus, and other interface elements".to_string(),
                    value: self.get_setting("ui_sounds").unwrap_or(&"true".to_string()) == "true",
                    key: "ui_sounds".to_string(),
                },
            ],
            SettingsCategory::Network => vec![
                SettingControl::Section {
                    title: "Wi-Fi".to_string(),
                },
                SettingControl::Toggle {
                    label: "Wi-Fi".to_string(),
                    description: "Enable Wi-Fi networking".to_string(),
                    value: self.get_setting("wifi_enabled").unwrap_or(&"true".to_string()) == "true",
                    key: "wifi_enabled".to_string(),
                },
                SettingControl::Button {
                    label: "Advanced".to_string(),
                    description: "Configure advanced network settings".to_string(),
                    button_text: "Advanced...".to_string(),
                    action: "network_advanced".to_string(),
                },
                SettingControl::Section {
                    title: "Ethernet".to_string(),
                },
                SettingControl::Info {
                    label: "Status".to_string(),
                    value: "Connected".to_string(),
                },
                SettingControl::Info {
                    label: "IP Address".to_string(),
                    value: "192.168.1.100".to_string(),
                },
            ],
            SettingsCategory::Display => vec![
                SettingControl::Section {
                    title: "Display".to_string(),
                },
                SettingControl::Dropdown {
                    label: "Resolution".to_string(),
                    description: "Choose a resolution for this display".to_string(),
                    value: self.get_setting("display_resolution").unwrap_or(&"1920x1080".to_string()).clone(),
                    options: vec![
                        ("1920x1080".to_string(), "1920 × 1080".to_string()),
                        ("2560x1440".to_string(), "2560 × 1440".to_string()),
                        ("3840x2160".to_string(), "3840 × 2160 (4K)".to_string()),
                    ],
                    key: "display_resolution".to_string(),
                },
                SettingControl::Dropdown {
                    label: "Scaling".to_string(),
                    description: "Adjust the size of text and interface elements".to_string(),
                    value: self.get_setting("display_scaling").unwrap_or(&"100".to_string()).clone(),
                    options: vec![
                        ("100".to_string(), "100% (Default)".to_string()),
                        ("125".to_string(), "125% (Larger)".to_string()),
                        ("150".to_string(), "150% (Largest)".to_string()),
                    ],
                    key: "display_scaling".to_string(),
                },
                SettingControl::Toggle {
                    label: "Automatically adjust brightness".to_string(),
                    description: "Adjust the display brightness based on ambient light".to_string(),
                    value: self.get_setting("auto_brightness").unwrap_or(&"true".to_string()) == "true",
                    key: "auto_brightness".to_string(),
                },
            ],
            SettingsCategory::Updates => vec![
                SettingControl::Section {
                    title: "Automatic Updates".to_string(),
                },
                SettingControl::Toggle {
                    label: "Automatically keep my system up to date".to_string(),
                    description: "Download and install updates automatically".to_string(),
                    value: self.get_setting("auto_updates").unwrap_or(&"true".to_string()) == "true",
                    key: "auto_updates".to_string(),
                },
                SettingControl::Toggle {
                    label: "Download updates over metered connections".to_string(),
                    description: "Allow updates to download when using a metered internet connection".to_string(),
                    value: self.get_setting("updates_metered").unwrap_or(&"false".to_string()) == "true",
                    key: "updates_metered".to_string(),
                },
                SettingControl::Section {
                    title: "Update Status".to_string(),
                },
                SettingControl::Info {
                    label: "Last Check".to_string(),
                    value: "Today at 2:30 PM".to_string(),
                },
                SettingControl::Button {
                    label: "Check for Updates".to_string(),
                    description: "Check for available system updates now".to_string(),
                    button_text: "Check Now".to_string(),
                    action: "check_updates".to_string(),
                },
            ],
            SettingsCategory::About => vec![
                SettingControl::Section {
                    title: "System Information".to_string(),
                },
                SettingControl::Info {
                    label: "System Version".to_string(),
                    value: "PrismaUI 1.0.0".to_string(),
                },
                SettingControl::Info {
                    label: "Build".to_string(),
                    value: "24A348".to_string(),
                },
                SettingControl::Info {
                    label: "Processor".to_string(),
                    value: "Intel Core i7-9750H".to_string(),
                },
                SettingControl::Info {
                    label: "Memory".to_string(),
                    value: "16 GB DDR4".to_string(),
                },
                SettingControl::Info {
                    label: "Storage".to_string(),
                    value: "512 GB SSD".to_string(),
                },
                SettingControl::Section {
                    title: "Legal".to_string(),
                },
                SettingControl::Button {
                    label: "Software License".to_string(),
                    description: "View the software license agreement".to_string(),
                    button_text: "View License".to_string(),
                    action: "view_license".to_string(),
                },
            ],
            _ => vec![
                SettingControl::Section {
                    title: format!("{} Settings", self.active_category.display_name()),
                },
                SettingControl::Info {
                    label: "Status".to_string(),
                    value: "Settings for this category are coming soon...".to_string(),
                },
            ],
        }
    }

    /// Render the settings sidebar
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(280.0))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                // Search bar
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        TextInput::new(&self.search_input)
                            .w_full()
                            .h(px(36.0))
                    )
            )
            .child(
                // Categories list
                div()
                    .flex_1()
                    .scrollable(gpui::Axis::Vertical)
                    .p_2()
                    .children(self.categories.iter().enumerate().map(|(idx, category)| {
                        let is_active = category == &self.active_category;
                        let matches_search = self.search_query.is_empty() ||
                            category.display_name().to_lowercase().contains(&self.search_query.to_lowercase()) ||
                            category.description().to_lowercase().contains(&self.search_query.to_lowercase());

                        if !matches_search {
                            return div().into_any_element();
                        }

                        Button::new(("category", idx))
                            .w_full()
                            .ghost()
                            .justify_start()
                            .p_3()
                            .rounded(cx.theme().radius)
                            .when(is_active, |btn| btn.selected(true))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size(px(32.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(if is_active { cx.theme().primary } else { cx.theme().muted })
                                            .text_color(if is_active { cx.theme().primary_foreground } else { cx.theme().muted_foreground })
                                            .rounded(px(6.0))
                                            .child(if let Some(icon_path) = category.icon_image() {
                                                img(icon_path).w_4().h_4().into_any_element()
                                            } else {
                                                Icon::new(category.icon()).size_4().into_any_element()
                                            })
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .items_start()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .text_color(if is_active { cx.theme().foreground } else { cx.theme().foreground })
                                                    .child(category.display_name())
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .line_clamp(2)
                                                    .child(category.description())
                                            )
                                    )
                            )
                            .on_click({
                                let category = category.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.set_category(category.clone(), cx);
                                })
                            })
                            .into_any_element()
                    }))
            )
    }

    /// Render settings content area
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.get_category_settings();

        v_flex()
            .flex_1()
            .h_full()
            .bg(cx.theme().background)
            .child(
                // Header
                div()
                    .h(px(60.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px_6()
                    .bg(cx.theme().background)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child(self.active_category.display_name())
                    )
            )
            .child(
                // Settings content
                div()
                    .flex_1()
                    .scrollable(gpui::Axis::Vertical)
                    .p_6()
                    .child(
                        v_flex()
                            .gap_6()
                            .max_w(px(600.0))
                            .children(settings.into_iter().enumerate().map(|(idx, setting)| {
                                self.render_setting_control(idx, setting, cx)
                            }))
                    )
            )
    }

    /// Render individual setting control
    fn render_setting_control(&self, idx: usize, control: SettingControl, cx: &mut Context<Self>) -> impl IntoElement {
        match control {
            SettingControl::Section { title } => {
                div()
                    .mt_6()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .mb_4()
                            .child(title)
                    )
            }
            SettingControl::Toggle { label, description, value, key } => {
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(label)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            )
                    )
                    .child(
                        Button::new(("toggle", idx))
                            .ghost()
                            .compact()
                            .when(value, |btn| btn.selected(true))
                            .child(if value { "ON" } else { "OFF" })
                            .on_click({
                                let key = key.clone();
                                cx.listener(move |this, _, _, cx| {
                                    let new_value = !value;
                                    this.update_setting(&key, &new_value.to_string(), cx);
                                })
                            })
                    )
            }
            SettingControl::Slider { label, description, value, min, max, step: _, key } => {
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(label)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            )
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{:.0}%", min * 100.0))
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(6.0))
                                    .bg(cx.theme().muted)
                                    .rounded_full()
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .h_full()
                                            .bg(cx.theme().primary)
                                            .rounded_full()
                                            .w(px((value - min) / (max - min) * 200.0)) // Assuming 200px width
                                    )
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{:.0}%", max * 100.0))
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(format!("{:.0}%", value * 100.0))
                            )
                    )
            }
            SettingControl::Dropdown { label, description, value: _, options, key: _ } => {
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(label)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            )
                    )
                    .child(
                        Button::new(("dropdown", idx))
                            .ghost()
                            .w_full()
                            .justify_between()
                            .px_3()
                            .py_2()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(options.first().map(|(_, display)| display.clone()).unwrap_or_default())
                                    .child(Icon::new(IconName::ChevronDown).size_4())
                            )
                    )
            }
            SettingControl::Button { label, description, button_text, action: _ } => {
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(label)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            )
                    )
                    .child(
                        Button::new(("button", idx))
                            .primary()
                            .child(button_text)
                    )
            }
            SettingControl::Info { label, value } => {
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        div()
                            .text_base()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(label)
                    )
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().muted_foreground)
                            .child(value)
                    )
            }
            SettingControl::TextInput { label, description, value: _, placeholder: _, key: _ } => {
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_4()
                    .bg(cx.theme().sidebar.opacity(0.3))
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(label)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description)
                            )
                    )
                    .child(
                        div()
                            .w_full()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().input)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .text_sm()
                            .child("Text input placeholder")
                    )
            }
        }
    }
}

impl EventEmitter<SettingsAction> for Settings {}

impl Focusable for Settings {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}