/// System shell component managing desktop interactions and effects
use gpui::{
    div, px, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window, Bounds, Pixels, AppContext
};
use gpui_component::{ActiveTheme, StyledExt};

use crate::{
    window_manager::WindowManager,
    components::{AppMenu, CommandPalette},
};

/// System shell managing desktop interactions, effects, and overlays
pub struct SystemShell {
    /// Desktop bounds
    bounds: Bounds<Pixels>,
    /// Window manager reference
    window_manager: Entity<WindowManager>,
    /// App menu reference
    app_menu: Entity<AppMenu>,
    /// Command palette reference
    command_palette: Entity<CommandPalette>,
    /// Whether desktop icons are shown
    show_desktop_icons: bool,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl SystemShell {
    pub fn new(
        bounds: Bounds<Pixels>,
        window_manager: Entity<WindowManager>,
        app_menu: Entity<AppMenu>,
        command_palette: Entity<CommandPalette>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            bounds,
            window_manager,
            app_menu,
            command_palette,
            show_desktop_icons: true,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create system shell entity
    pub fn create(
        bounds: Bounds<Pixels>,
        window_manager: Entity<WindowManager>,
        app_menu: Entity<AppMenu>,
        command_palette: Entity<CommandPalette>,
        cx: &mut gpui::App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(bounds, window_manager, app_menu, command_palette, cx))
    }

    /// Update bounds on screen resolution change
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.bounds = bounds;
        cx.notify();
    }

    /// Toggle desktop icons visibility
    pub fn toggle_desktop_icons(&mut self, cx: &mut Context<Self>) {
        self.show_desktop_icons = !self.show_desktop_icons;
        cx.notify();
    }

    fn render_desktop_icons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.show_desktop_icons {
            return div();
        }

        use gpui_component::{v_flex, Icon, IconName};

        // Sample desktop icons
        let desktop_icons = vec![
            ("Trash", IconName::Trash2, "Open trash"),
            ("Computer", IconName::HardDrive, "This computer"),
            ("Network", IconName::Globe, "Network locations"),
            ("Documents", IconName::FileText, "Documents folder"),
        ];

        div()
            .absolute()
            .top_4()
            .right_4()
            .child(
                v_flex()
                    .gap_6()
                    .children(desktop_icons.into_iter().map(|(name, icon, tooltip)| {
                        div()
                            .w(px(80.0))
                            .cursor_pointer()
                            .p_2()
                            .rounded(cx.theme().radius)
                            .hover(|this| this.bg(cx.theme().accent.opacity(0.1)))
                            .on_double_click(cx.listener(move |_, _, _, _| {
                                tracing::info!("Double-clicked desktop icon: {}", name);
                            }))
                            .child(
                                v_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .size_12()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(cx.theme().background.opacity(0.8))
                                            .text_color(cx.theme().foreground)
                                            .rounded(cx.theme().radius)
                                            .shadow_sm()
                                            .child(Icon::new(icon).size_6())
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_center()
                                            .text_color(cx.theme().background)
                                            .font_semibold()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(gpui::rgba(0x000000_80))
                                            .child(name)
                                    )
                            )
                            .tooltip(tooltip)
                    }))
            )
    }

    fn render_context_menu_overlay(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        // TODO: Implement right-click context menu system
        div()
    }

    fn render_drag_drop_overlay(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        // TODO: Implement drag and drop visual feedback
        div()
    }

    fn render_snap_zones(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        // TODO: Implement window snapping visual zones
        div()
    }
}

impl Focusable for SystemShell {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SystemShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .size_full()
            .pointer_events_none() // Let events pass through to underlying components
            // Desktop icons
            .child(
                div()
                    .pointer_events_auto() // Re-enable events for icons
                    .child(self.render_desktop_icons(cx))
            )
            // Context menu overlay
            .child(self.render_context_menu_overlay(cx))
            // Drag and drop overlay
            .child(self.render_drag_drop_overlay(cx))
            // Window snap zones
            .child(self.render_snap_zones(cx))
    }
}