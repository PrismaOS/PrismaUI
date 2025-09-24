/// App menu component - Windows Start Menu / macOS Dock hybrid
use gpui::{
    div, img, px, Action, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, AppContext, ElementId, Animation, AnimationExt
};
use std::time::Duration;
use gpui_component::animation::cubic_bezier;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, input::{InputEvent, InputState, TextInput},
    v_flex, ActiveTheme, Icon, IconName, StyledExt, Selectable
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application entry in the menu
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: IconName,
    pub executable_path: String,
    pub category: String,
    pub pinned: bool,
    pub recently_used: bool,
}

/// App menu categories
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppCategory {
    Pinned,
    RecentlyUsed,
    Productivity,
    Development,
    Games,
    Multimedia,
    System,
    Other,
}

impl AppCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Pinned => "Pinned",
            Self::RecentlyUsed => "Recently Used",
            Self::Productivity => "Productivity",
            Self::Development => "Development",
            Self::Games => "Games",
            Self::Multimedia => "Multimedia",
            Self::System => "System",
            Self::Other => "Other",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Pinned => IconName::Star,
            Self::RecentlyUsed => IconName::Calendar,
            Self::Productivity => IconName::Building2,
            Self::Development => IconName::SquareTerminal,
            Self::Games => IconName::Heart,
            Self::Multimedia => IconName::TriangleAlert,
            Self::System => IconName::Settings,
            Self::Other => IconName::Menu,
        }
    }
}

/// Actions for app menu
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = app_menu, no_json)]
pub enum AppMenuAction {
    ToggleMenu,
    LaunchApp(String),
    PinApp(String),
    UnpinApp(String),
    Search(String),
}

/// App menu component - hybrid of Windows Start and macOS Dock
pub struct AppMenu {
    /// Whether the menu is currently open
    open: bool,
    /// All available applications
    apps: HashMap<String, AppEntry>,
    /// Apps organized by category
    categories: HashMap<AppCategory, Vec<String>>,
    /// Search input state
    search_input: Entity<InputState>,
    /// Filtered apps based on search
    filtered_apps: Vec<AppEntry>,
    /// Currently selected category
    active_category: AppCategory,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl AppMenu {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search apps...")
        });

        // Subscribe to search input changes
        cx.subscribe(&search_input, |this, _, event, cx| {
            match event {
                InputEvent::Change => {
                    let query = this.search_input.read(cx).value();
                    this.search(&query, cx);
                }
                _ => {}
            }
        }).detach();

        // Create sample applications
        let mut apps = HashMap::new();
        let sample_apps = vec![
            AppEntry {
                id: "terminal".to_string(),
                name: "Terminal".to_string(),
                description: "Command line interface".to_string(),
                icon: IconName::SquareTerminal,
                executable_path: "/usr/bin/terminal".to_string(),
                category: "Development".to_string(),
                pinned: true,
                recently_used: true,
            },
            AppEntry {
                id: "code_editor".to_string(),
                name: "Code Editor".to_string(),
                description: "Advanced text editor for development".to_string(),
                icon: IconName::SquareTerminal,
                executable_path: "/usr/bin/code".to_string(),
                category: "Development".to_string(),
                pinned: true,
                recently_used: true,
            },
            AppEntry {
                id: "file_manager".to_string(),
                name: "File Manager".to_string(),
                description: "Browse and manage files".to_string(),
                icon: IconName::Folder,
                executable_path: "/usr/bin/files".to_string(),
                category: "System".to_string(),
                pinned: true,
                recently_used: false,
            },
            AppEntry {
                id: "web_browser".to_string(),
                name: "Web Browser".to_string(),
                description: "Browse the internet".to_string(),
                icon: IconName::Globe,
                executable_path: "/usr/bin/browser".to_string(),
                category: "Productivity".to_string(),
                pinned: false,
                recently_used: true,
            },
            AppEntry {
                id: "calculator".to_string(),
                name: "Calculator".to_string(),
                description: "Perform calculations".to_string(),
                icon: IconName::Plus,
                executable_path: "/usr/bin/calc".to_string(),
                category: "Productivity".to_string(),
                pinned: false,
                recently_used: false,
            },
            AppEntry {
                id: "settings".to_string(),
                name: "System Settings".to_string(),
                description: "Configure system preferences".to_string(),
                icon: IconName::Settings,
                executable_path: "/usr/bin/settings".to_string(),
                category: "System".to_string(),
                pinned: false,
                recently_used: false,
            },
        ];

        for app in sample_apps {
            apps.insert(app.id.clone(), app);
        }

        let mut this = Self {
            open: false,
            apps,
            categories: HashMap::new(),
            search_input,
            filtered_apps: Vec::new(),
            active_category: AppCategory::Pinned,
            focus_handle: cx.focus_handle(),
        };

        this.organize_categories();
        this.update_filtered_apps("", AppCategory::Pinned);
        this
    }

    /// Create app menu entity
    pub fn create(window: &mut Window, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// Toggle menu open/closed state
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            cx.focus_view(&self.search_input, window);
        }
        cx.notify();
    }

    /// Close the menu
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    /// Launch an application
    pub fn launch_app(&mut self, app_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(app) = self.apps.get_mut(app_id) {
            // Mark as recently used
            app.recently_used = true;

            // Emit event to desktop to create the application window
            cx.emit(AppMenuAction::LaunchApp(app_id.to_string()));

            tracing::info!("Launching app: {} ({})", app.name, app.executable_path);

            // Close menu after launching
            self.close(cx);
            self.organize_categories();
        }
    }

    /// Pin/unpin an application
    pub fn toggle_pin(&mut self, app_id: &str, cx: &mut Context<Self>) {
        if let Some(app) = self.apps.get_mut(app_id) {
            app.pinned = !app.pinned;
            self.organize_categories();
            self.update_filtered_apps("", self.active_category.clone());
            cx.notify();
        }
    }

    /// Update search filter
    pub fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.update_filtered_apps(query, self.active_category.clone());
        cx.notify();
    }

    /// Switch active category
    pub fn set_category(&mut self, category: AppCategory, cx: &mut Context<Self>) {
        self.active_category = category;
        self.update_filtered_apps("", self.active_category.clone());
        cx.notify();
    }

    /// Organize apps into categories
    fn organize_categories(&mut self) {
        self.categories.clear();

        for app in self.apps.values() {
            // Add to pinned if pinned
            if app.pinned {
                self.categories
                    .entry(AppCategory::Pinned)
                    .or_insert_with(Vec::new)
                    .push(app.id.clone());
            }

            // Add to recently used if recently used
            if app.recently_used {
                self.categories
                    .entry(AppCategory::RecentlyUsed)
                    .or_insert_with(Vec::new)
                    .push(app.id.clone());
            }

            // Add to appropriate category
            let category = match app.category.as_str() {
                "Development" => AppCategory::Development,
                "Productivity" => AppCategory::Productivity,
                "Games" => AppCategory::Games,
                "Multimedia" => AppCategory::Multimedia,
                "System" => AppCategory::System,
                _ => AppCategory::Other,
            };

            self.categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(app.id.clone());
        }
    }

    /// Update filtered apps based on search query and category
    fn update_filtered_apps(&mut self, query: &str, category: AppCategory) {
        let query = query.to_lowercase();

        let app_ids = self.categories.get(&category).cloned().unwrap_or_default();

        self.filtered_apps = app_ids
            .into_iter()
            .filter_map(|id| self.apps.get(&id))
            .filter(|app| {
                query.is_empty() ||
                app.name.to_lowercase().contains(&query) ||
                app.description.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
    }

    fn render_category_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let categories = [
            AppCategory::Pinned,
            AppCategory::RecentlyUsed,
            AppCategory::Development,
            AppCategory::Productivity,
            AppCategory::Games,
            AppCategory::Multimedia,
            AppCategory::System,
            AppCategory::Other,
        ];

        v_flex()
            .w(px(200.0))
            .h_full()
            .bg(cx.theme().sidebar.opacity(0.8))
            .border_r_1()
            .border_color(cx.theme().border.opacity(0.5))
            .p_4()
            .gap_3()
            .scrollable(gpui::Axis::Vertical)
            .children(categories.iter().cloned().enumerate().map(|(idx, category)| {
                let is_active = category == self.active_category;
                let count = self.categories.get(&category).map_or(0, |apps| apps.len());

                Button::new(("category", idx))
                    .w_full()
                    .ghost()
                    .justify_start()
                    .px_3()
                    .py_2()
                    .when(is_active, |btn| btn.selected(true))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .child(Icon::new(category.icon()).size_4().text_color(
                                        if is_active { cx.theme().primary } else { cx.theme().muted_foreground }
                                    ))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .text_color(
                                                if is_active { cx.theme().foreground } else { cx.theme().muted_foreground }
                                            )
                                            .child(category.display_name().to_string())
                                    )
                            )
                            .when(count > 0, |this| {
                                this.child(
                                    div()
                                        .bg(if is_active { cx.theme().primary } else { cx.theme().muted })
                                        .text_color(if is_active { cx.theme().primary_foreground } else { cx.theme().muted_foreground })
                                        .px_2()
                                        .py_1()
                                        .rounded_full()
                                        .text_xs()
                                        .font_medium()
                                        .min_w(px(20.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(count.to_string())
                                )
                            })
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_category(category, cx);
                    }))
            }))
    }

    fn render_app_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .pt_4()
            .pb_4()
            .pl_0()
            .pr_4()
            .gap_4()
            .child(
                // Search bar
                div()
                    .w_full()
                    .pl_4()
                    .child(
                        TextInput::new(&self.search_input)
                            .w_full()
                            .max_w(px(400.0))
                            .h(px(42.0))
                            .px_4()
                            .rounded_lg()
                    )
            )
            .child(
                // App grid container with proper spacing
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .scrollable(gpui::Axis::Vertical)
                    .pl_4()
                    .child(
                        // Grid layout - explicitly left aligned
                        div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap_4()
                            .justify_start()
                            .items_start()
                            .pl_0()
                            .ml_0()
                            .pr_0()
                            .mr_0()
                            .children(self.filtered_apps.iter().enumerate().map(|(idx, app)| {
                                div()
                                    .w(px(100.0))
                                    .h(px(120.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Button::new(("app", idx))
                                            .ghost()
                                            .p_3()
                                            .rounded_xl()
                                            .w(px(80.0))
                                            .h(px(80.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .on_click({
                                                let app_id = app.id.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    this.launch_app(&app_id, window, cx);
                                                })
                                            })
                                            .child(
                                                div()
                                                    .w(px(56.0))
                                                    .h(px(56.0))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .bg(cx.theme().primary.opacity(0.1))
                                                    .text_color(cx.theme().primary)
                                                    .rounded_xl()
                                                    .child(Icon::new(app.icon.clone()).size_6())
                                            )
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .text_xs()
                                            .font_medium()
                                            .text_center()
                                            .text_color(cx.theme().foreground)
                                            .line_clamp(2)
                                            .px_1()
                                            .child(app.name.clone())
                                    )
                            }))
                    )
            )
    }

    fn render_user_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h(px(70.0))
            .bg(cx.theme().sidebar.opacity(0.3))
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.5))
            .px_4()
            .py_3()
            .items_center()
            .gap_3()
            .child(
                // User avatar placeholder
                div()
                    .size(px(48.0))
                    .bg(cx.theme().primary)
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(IconName::User)
                            .size_6()
                            .text_color(cx.theme().primary_foreground)
                    )
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("PrismaUI User")
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("user@prismaui.dev")
                    )
            )
            .child(
                Button::new("user-settings")
                    .ghost()
                    .size(px(32.0))
                    .icon(IconName::Settings)
                    .tooltip("User Settings")
            )
    }

    fn render_power_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h(px(60.0))
            .bg(cx.theme().sidebar.opacity(0.3))
            .border_t_1()
            .border_color(cx.theme().border.opacity(0.5))
            .px_4()
            .items_center()
            .justify_end()
            .gap_2()
            .child(
                Button::new("lock")
                    .ghost()
                    .size(px(40.0))
                    .child(img("icons/lock-512.png").w_5().h_5())
                    .tooltip("Lock")
                    .on_click(cx.listener(|this, _, _, cx| {
                        // TODO: Implement lock functionality
                        this.close(cx);
                    }))
            )
            .child(
                Button::new("sleep")
                    .ghost()
                    .size(px(40.0))
                    .child(img("icons/moon-4-512.png").w_5().h_5())
                    .tooltip("Sleep")
                    .on_click(cx.listener(|this, _, _, cx| {
                        // TODO: Implement sleep functionality
                        this.close(cx);
                    }))
            )
            .child(
                Button::new("restart")
                    .ghost()
                    .size(px(40.0))
                    .child(img("icons/refresh-2-512.png").w_5().h_5())
                    .tooltip("Restart")
                    .on_click(cx.listener(|this, _, _, cx| {
                        // TODO: Implement restart functionality
                        this.close(cx);
                    }))
            )
            .child(
                Button::new("shutdown")
                    .ghost()
                    .size(px(40.0))
                    .child(img("icons/power-2-512.png").w_5().h_5())
                    .tooltip("Shutdown")
                    .on_click(cx.listener(|this, _, _, cx| {
                        // TODO: Implement shutdown functionality
                        this.close(cx);
                    }))
            )
    }
}

impl EventEmitter<AppMenuAction> for AppMenu {}

impl Focusable for AppMenu {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element(); // Hidden when closed
        }

        // Modern start menu with user and power sections
        div()
            .absolute()
            .bottom(px(56.0)) // Above taskbar
            .left(px(12.0)) // Left aligned
            .w(px(750.0)) // Wider for better app grid layout
            .h(px(720.0)) // Taller for additional sections
            .bg(cx.theme().background.opacity(0.95))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_2xl()
            .overflow_hidden()
            .child(
                v_flex()
                    .size_full()
                    .child(self.render_user_section(cx))
                    .child(
                        h_flex()
                            .flex_1()
                            .items_start()
                            .child(self.render_category_sidebar(cx))
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .child(self.render_app_grid(cx))
                            )
                    )
                    .child(self.render_power_menu(cx))
            )
            .with_animation(
                ElementId::Name("start-menu-open".into()),
                Animation::new(Duration::from_secs_f64(0.25))
                    .with_easing(cubic_bezier(0.32, 0.72, 0., 1.)),
                move |this, delta| {
                    // Slide up from bottom and fade in
                    let y_offset = px(30.) * (1. - delta);
                    let opacity = 0.5 + (0.5 * delta);
                    this.bottom(px(56.0) - y_offset)
                        .opacity(opacity)
                }
            )
            .into_any_element()
    }
}