/// Wallpaper component with image loading and GPU-accelerated effects
use gpui::{
    div, img, rgb, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Pixels, Render, Styled, StyledImage, Window
};
use gpui_component::ActiveTheme;
// use gpui_component::StyledExt;

/// Wallpaper display modes
#[derive(Clone, Debug, PartialEq)]
pub enum WallpaperMode {
    /// Stretch to fill entire screen
    Stretch,
    /// Scale maintaining aspect ratio, may have letterboxing
    Fit,
    /// Scale to fill screen, may crop parts of image
    Fill,
    /// Center image at original size
    Center,
    /// Tile image across screen
    Tile,
}

/// Wallpaper component supporting various image formats and display modes
pub struct Wallpaper {
    /// Image path or URL
    pub image_path: Option<String>,
    /// Display mode for the wallpaper
    pub mode: WallpaperMode,
    /// Fallback solid color
    pub fallback_color: gpui::Rgba,
    /// Desktop bounds for proper sizing
    pub bounds: Bounds<Pixels>,
}

impl Wallpaper {
    pub fn new(bounds: Bounds<Pixels>) -> Self {
        Self {
            image_path: None,
            mode: WallpaperMode::Fill,
            fallback_color: gpui::Rgba { r: 0.176, g: 0.216, b: 0.282, a: 1.0 }, // Nice dark blue-gray
            bounds,
        }
    }

    /// Set wallpaper image from path
    pub fn image(mut self, path: impl Into<String>) -> Self {
        self.image_path = Some(path.into());
        self
    }

    /// Set wallpaper display mode
    pub fn mode(mut self, mode: WallpaperMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set fallback color
    pub fn fallback_color(mut self, color: gpui::Rgba) -> Self {
        self.fallback_color = color;
        self
    }

    /// Create wallpaper entity
    pub fn create(bounds: Bounds<Pixels>, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|_| Self::new(bounds))
    }

    /// Update wallpaper image
    pub fn set_image(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        self.image_path = path;
        cx.notify();
    }

    /// Update display mode
    pub fn set_mode(&mut self, mode: WallpaperMode, cx: &mut Context<Self>) {
        self.mode = mode;
        cx.notify();
    }

    /// Update bounds (on screen resolution change)
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.bounds = bounds;
        cx.notify();
    }

    fn render_image(&self, path: &str) -> impl IntoElement {
        // Debug: print the path being loaded
        eprintln!("Wallpaper: Attempting to load image: {}", path);

        let image = img(path);

        // For simplicity, always wrap in a div container
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.fallback_color)  // Add fallback background color
            .child(
                match self.mode {
                    WallpaperMode::Fill => image.size_full().object_fit(gpui::ObjectFit::Cover),
                    WallpaperMode::Stretch => image.size_full(),
                    WallpaperMode::Fit => image.max_w_full().max_h_full().object_fit(gpui::ObjectFit::Contain),
                    WallpaperMode::Center => image,
                    WallpaperMode::Tile => image,
                }
            )
    }

    fn render_fallback(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(self.fallback_color)
            // Add subtle gradient for visual interest
            .child(
                div()
                    .absolute()
                    .size_full()
                    .bg(
                        // Create a subtle radial gradient effect
                        gpui::Rgba {
                            r: self.fallback_color.r * 1.1,
                            g: self.fallback_color.g * 1.1,
                            b: self.fallback_color.b * 1.1,
                            a: 0.3,
                        }
                    )
                    .opacity(0.3)
            )
    }
}

impl Render for Wallpaper {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .size_full()
            .overflow_hidden()
            .child(
                if let Some(ref path) = self.image_path {
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(self.render_image(path))
                        .into_any_element()
                } else {
                    self.render_fallback(cx).into_any_element()
                }
            )
            // Add subtle overlay for better contrast with desktop icons
            .child(
                div()
                    .absolute()
                    .size_full()
                    .bg(cx.theme().transparent)
                    .opacity(0.05)
            )
    }
}