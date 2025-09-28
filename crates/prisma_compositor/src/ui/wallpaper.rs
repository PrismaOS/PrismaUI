/// Beautiful desktop wallpaper system with GPU-accelerated effects
use crate::{
    geometry::{Rect, Size, Color, Transform, Point},
    renderer::{RenderCommand, RenderLayer},
    ui::{UIElement, Layout, LayoutConstraints, EdgeInsets},
    events::Event,
};

/// Wallpaper display modes for different aesthetic preferences
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WallpaperMode {
    /// Fill the entire screen, may crop image
    Fill,
    /// Fit image within screen bounds, may show letterboxing
    Fit,
    /// Stretch image to exact screen dimensions
    Stretch,
    /// Center image at original size
    Center,
    /// Tile image across screen
    Tile,
}

/// Beautiful desktop wallpaper with GPU-accelerated effects
pub struct Wallpaper {
    id: String,
    layout: Layout,
    image_path: Option<String>,
    mode: WallpaperMode,
    texture_id: Option<u32>,
    fallback_color: Color,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
}

impl Wallpaper {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            image_path: None,
            mode: WallpaperMode::Fill,
            texture_id: None,
            fallback_color: Self::create_beautiful_gradient_base(),
            visible: true,
            needs_layout: true,
            needs_visual: true,
        }
    }

    /// Create a beautiful gradient base color for fallback
    fn create_beautiful_gradient_base() -> Color {
        // Deep purple-blue base similar to macOS Monterey
        Color::from_hex("#1E1E2E").unwrap_or(Color::new(0.12, 0.12, 0.18, 1.0))
    }

    /// Set wallpaper image
    pub fn set_image(&mut self, path: Option<String>, texture_id: Option<u32>) {
        self.image_path = path;
        self.texture_id = texture_id;
        self.needs_visual = true;
    }

    /// Set wallpaper display mode
    pub fn set_mode(&mut self, mode: WallpaperMode) {
        self.mode = mode;
        self.needs_visual = true;
    }

    /// Set fallback color
    pub fn set_fallback_color(&mut self, color: Color) {
        self.fallback_color = color;
        self.needs_visual = true;
    }

    /// Generate beautiful gradient background commands
    fn render_gradient_background(&self, bounds: Rect, z_index: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();

        // Main gradient background
        commands.push(RenderCommand::GradientRectangle {
            rect: bounds,
            start_color: Color::from_hex("#1E1E2E").unwrap_or(Color::new(0.12, 0.12, 0.18, 1.0)),
            end_color: Color::from_hex("#2A2A4A").unwrap_or(Color::new(0.16, 0.16, 0.29, 1.0)),
            direction: std::f32::consts::PI * 0.75, // Diagonal gradient
            transform: Transform::identity(),
            z_index,
        });

        // Add floating orbs for depth and beauty
        self.render_floating_orbs(bounds, z_index + 0.1, &mut commands);

        // Subtle texture overlay
        commands.push(RenderCommand::Rectangle {
            rect: bounds,
            color: Color::new(1.0, 1.0, 1.0, 0.02), // Very subtle white overlay
            transform: Transform::identity(),
            z_index: z_index + 0.2,
        });

        commands
    }

    /// Render floating orbs for visual depth
    fn render_floating_orbs(&self, bounds: Rect, z_index: f32, commands: &mut Vec<RenderCommand>) {
        let orbs = [
            // Large purple orb
            (
                Point::new(bounds.size.width * 0.15, bounds.size.height * 0.25),
                400.0,
                Color::from_hex("#8A2BE2").unwrap_or(Color::new(0.54, 0.17, 0.89, 0.08)),
            ),
            // Medium blue orb
            (
                Point::new(bounds.size.width * 0.75, bounds.size.height * 0.15),
                300.0,
                Color::from_hex("#4A90E2").unwrap_or(Color::new(0.29, 0.56, 0.89, 0.1)),
            ),
            // Small pink orb
            (
                Point::new(bounds.size.width * 0.85, bounds.size.height * 0.7),
                250.0,
                Color::from_hex("#FF7AB8").unwrap_or(Color::new(1.0, 0.48, 0.72, 0.06)),
            ),
            // Subtle cyan orb
            (
                Point::new(bounds.size.width * 0.3, bounds.size.height * 0.8),
                200.0,
                Color::from_hex("#00CED1").unwrap_or(Color::new(0.0, 0.81, 0.82, 0.04)),
            ),
        ];

        for (position, size, color) in orbs {
            let orb_rect = Rect::new(
                bounds.origin.x + position.x - size / 2.0,
                bounds.origin.y + position.y - size / 2.0,
                size,
                size,
            );

            commands.push(RenderCommand::RoundedRectangle {
                rect: orb_rect,
                corner_radius: size / 2.0, // Perfect circle
                color,
                transform: Transform::identity(),
                z_index,
            });
        }
    }

    /// Render image wallpaper with proper scaling
    fn render_image_wallpaper(&self, bounds: Rect, z_index: f32) -> Vec<RenderCommand> {
        if let (Some(_path), Some(texture_id)) = (&self.image_path, self.texture_id) {
            let mut commands = Vec::new();

            // Calculate UV coordinates based on mode
            let (image_rect, uv_rect) = self.calculate_image_transform(bounds);

            commands.push(RenderCommand::TexturedRectangle {
                rect: image_rect,
                texture_id,
                uv_rect,
                color: Color::WHITE, // No tint
                transform: Transform::identity(),
                z_index,
            });

            // Add subtle overlay for better desktop icon readability
            commands.push(RenderCommand::Rectangle {
                rect: bounds,
                color: Color::new(0.0, 0.0, 0.0, 0.08),
                transform: Transform::identity(),
                z_index: z_index + 0.1,
            });

            commands
        } else {
            // Fallback to gradient if no image
            self.render_gradient_background(bounds, z_index)
        }
    }

    /// Calculate image transform based on wallpaper mode
    fn calculate_image_transform(&self, bounds: Rect) -> (Rect, Rect) {
        // For now, assume 1:1 image aspect ratio
        // In a real implementation, you'd get this from the loaded texture
        let image_aspect = 16.0 / 9.0; // Common wallpaper aspect ratio
        let screen_aspect = bounds.size.width / bounds.size.height;

        match self.mode {
            WallpaperMode::Fill => {
                // Scale to fill, maintaining aspect ratio
                if image_aspect > screen_aspect {
                    // Image is wider, fit to height
                    let scaled_width = bounds.size.height * image_aspect;
                    let x_offset = (bounds.size.width - scaled_width) / 2.0;
                    (
                        Rect::new(bounds.origin.x + x_offset, bounds.origin.y, scaled_width, bounds.size.height),
                        Rect::new(0.0, 0.0, 1.0, 1.0), // Full UV
                    )
                } else {
                    // Image is taller, fit to width
                    let scaled_height = bounds.size.width / image_aspect;
                    let y_offset = (bounds.size.height - scaled_height) / 2.0;
                    (
                        Rect::new(bounds.origin.x, bounds.origin.y + y_offset, bounds.size.width, scaled_height),
                        Rect::new(0.0, 0.0, 1.0, 1.0), // Full UV
                    )
                }
            }
            WallpaperMode::Fit => {
                // Scale to fit within bounds
                if image_aspect > screen_aspect {
                    // Image is wider, fit to width
                    let scaled_height = bounds.size.width / image_aspect;
                    let y_offset = (bounds.size.height - scaled_height) / 2.0;
                    (
                        Rect::new(bounds.origin.x, bounds.origin.y + y_offset, bounds.size.width, scaled_height),
                        Rect::new(0.0, 0.0, 1.0, 1.0),
                    )
                } else {
                    // Image is taller, fit to height
                    let scaled_width = bounds.size.height * image_aspect;
                    let x_offset = (bounds.size.width - scaled_width) / 2.0;
                    (
                        Rect::new(bounds.origin.x + x_offset, bounds.origin.y, scaled_width, bounds.size.height),
                        Rect::new(0.0, 0.0, 1.0, 1.0),
                    )
                }
            }
            WallpaperMode::Stretch => {
                // Stretch to exact bounds
                (bounds, Rect::new(0.0, 0.0, 1.0, 1.0))
            }
            WallpaperMode::Center => {
                // Center at original size (assuming some default size)
                let display_size = Size::new(800.0, 600.0); // Default size
                let x_offset = (bounds.size.width - display_size.width) / 2.0;
                let y_offset = (bounds.size.height - display_size.height) / 2.0;
                (
                    Rect::new(
                        bounds.origin.x + x_offset,
                        bounds.origin.y + y_offset,
                        display_size.width,
                        display_size.height,
                    ),
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                )
            }
            WallpaperMode::Tile => {
                // Tiling would require multiple render commands
                // For now, just use fill mode
                self.calculate_image_transform(bounds)
            }
        }
    }
}

impl UIElement for Wallpaper {
    fn id(&self) -> &str {
        &self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn measure(&mut self, available_size: Size) -> Size {
        // Wallpaper always wants to fill available space
        available_size
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;
        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        if self.image_path.is_some() && self.texture_id.is_some() {
            self.render_image_wallpaper(self.layout.bounds, z_index)
        } else {
            self.render_gradient_background(self.layout.bounds, z_index)
        }
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.needs_visual = true;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate_layout(&mut self) {
        self.needs_layout = true;
    }

    fn invalidate_visual(&mut self) {
        self.needs_visual = true;
    }
}

impl crate::events::EventHandler for Wallpaper {
    fn handle_event(&mut self, _event: &Event) -> bool {
        false // Wallpaper doesn't handle events
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}