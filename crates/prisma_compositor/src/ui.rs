/// GPU-accelerated UI element system with efficient layout and rendering
use std::collections::HashMap;
use crate::{
    geometry::{Rect, Size, Color, Transform},
    renderer::{RenderCommand, RenderLayer},
    events::{Event, EventHandler},
    text::{TextRenderer, FontProperties, TextLayout},
};

/// Layout constraints for flexible UI positioning
#[derive(Debug, Clone)]
pub struct LayoutConstraints {
    pub min_size: Size,
    pub max_size: Size,
    pub preferred_size: Option<Size>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub aspect_ratio: Option<f32>,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            min_size: Size::ZERO,
            max_size: Size::new(f32::INFINITY, f32::INFINITY),
            preferred_size: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            aspect_ratio: None,
        }
    }
}

/// Layout information for positioned elements
#[derive(Debug, Clone)]
pub struct Layout {
    pub bounds: Rect,
    pub constraints: LayoutConstraints,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            bounds: Rect::ZERO,
            constraints: LayoutConstraints::default(),
            margin: EdgeInsets::ZERO,
            padding: EdgeInsets::ZERO,
        }
    }
}

/// Edge insets for margin and padding
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    pub fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn total_size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }
}

/// Base trait for UI elements with GPU-accelerated rendering
pub trait UIElement: EventHandler {
    /// Get unique ID for this element
    fn id(&self) -> &str;

    /// Get layout information
    fn layout(&self) -> &Layout;

    /// Get mutable layout information
    fn layout_mut(&mut self) -> &mut Layout;

    /// Calculate preferred size given constraints
    fn measure(&mut self, available_size: Size) -> Size;

    /// Position and size the element within given bounds
    fn arrange(&mut self, bounds: Rect);

    /// Generate render commands for GPU rendering
    fn render(&self, z_index: f32) -> Vec<RenderCommand>;

    /// Get child elements for layout and event handling
    fn children(&self) -> Vec<&dyn UIElement> {
        Vec::new()
    }

    /// Get mutable child elements
    fn children_mut(&mut self) -> Vec<&mut dyn UIElement> {
        Vec::new()
    }

    /// Set visibility
    fn set_visible(&mut self, visible: bool);

    /// Check if element is visible
    fn is_visible(&self) -> bool;

    /// Mark element as needing layout
    fn invalidate_layout(&mut self);

    /// Mark element as needing visual update
    fn invalidate_visual(&mut self);

    /// Get element bounds for hit testing
    fn bounds(&self) -> Rect {
        self.layout().bounds
    }
}

/// Rectangle UI element for backgrounds, dividers, etc.
pub struct Rectangle {
    id: String,
    layout: Layout,
    color: Color,
    corner_radius: f32,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
}

impl Rectangle {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            color: Color::TRANSPARENT,
            corner_radius: 0.0,
            visible: true,
            needs_layout: true,
            needs_visual: true,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self.needs_visual = true;
        self
    }

    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self.needs_visual = true;
        self
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
        self.needs_visual = true;
    }
}

impl UIElement for Rectangle {
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
        if let Some(preferred) = self.layout.constraints.preferred_size {
            Size::new(
                preferred.width.min(available_size.width),
                preferred.height.min(available_size.height),
            )
        } else {
            available_size
        }
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;
        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        let render_bounds = Rect::new(
            self.layout.bounds.origin.x + self.layout.padding.left,
            self.layout.bounds.origin.y + self.layout.padding.top,
            self.layout.bounds.size.width - self.layout.padding.horizontal(),
            self.layout.bounds.size.height - self.layout.padding.vertical(),
        );

        if self.corner_radius > 0.0 {
            vec![RenderCommand::RoundedRectangle {
                rect: render_bounds,
                corner_radius: self.corner_radius,
                color: self.color,
                transform: Transform::identity(),
                z_index,
            }]
        } else {
            vec![RenderCommand::Rectangle {
                rect: render_bounds,
                color: self.color,
                transform: Transform::identity(),
                z_index,
            }]
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

impl EventHandler for Rectangle {
    fn handle_event(&mut self, _event: &Event) -> bool {
        false // Rectangles don't handle events by default
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}

/// Text UI element with GPU-accelerated rendering
pub struct Text {
    id: String,
    layout: Layout,
    content: String,
    font_properties: FontProperties,
    font_size: f32,
    color: Color,
    text_layout: Option<TextLayout>,
    visible: bool,
    needs_layout: bool,
    needs_visual: bool,
}

impl Text {
    pub fn new(id: String, content: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            content,
            font_properties: FontProperties::default(),
            font_size: 14.0,
            color: Color::BLACK,
            text_layout: None,
            visible: true,
            needs_layout: true,
            needs_visual: true,
        }
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self.needs_layout = true;
        self.needs_visual = true;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self.needs_visual = true;
        self
    }

    pub fn with_font_family(mut self, family: String) -> Self {
        self.font_properties.family = family;
        self.needs_layout = true;
        self.needs_visual = true;
        self
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.text_layout = None;
        self.needs_layout = true;
        self.needs_visual = true;
    }

    pub fn update_text_layout(&mut self, text_renderer: &mut TextRenderer) {
        self.text_layout = Some(text_renderer.render_text(
            &self.content,
            &self.font_properties,
            self.font_size,
            self.color,
            self.layout.bounds.origin,
        ));
        self.needs_visual = false;
    }
}

impl UIElement for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn measure(&mut self, _available_size: Size) -> Size {
        // TODO: Proper text measurement
        // For now, estimate based on font size
        let char_width = self.font_size * 0.6; // Rough estimate
        let line_height = self.font_size * 1.2;

        Size::new(
            self.content.len() as f32 * char_width,
            line_height,
        )
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;
        self.text_layout = None; // Invalidate text layout
        self.needs_layout = false;
        self.needs_visual = true;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible || self.text_layout.is_none() {
            return Vec::new();
        }

        if let Some(text_layout) = &self.text_layout {
            text_layout.to_render_commands(z_index)
        } else {
            Vec::new()
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
        self.text_layout = None;
    }

    fn invalidate_visual(&mut self) {
        self.needs_visual = true;
    }
}

impl EventHandler for Text {
    fn handle_event(&mut self, _event: &Event) -> bool {
        false // Text doesn't handle events by default
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}

/// Button UI element with hover and click states
pub struct Button {
    id: String,
    layout: Layout,
    background: Rectangle,
    text: Text,
    normal_color: Color,
    hover_color: Color,
    pressed_color: Color,
    is_hovered: bool,
    is_pressed: bool,
    visible: bool,
    needs_layout: bool,
    click_handler: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Button {
    pub fn new(id: String, text: String) -> Self {
        let bg_id = format!("{}_bg", id);
        let text_id = format!("{}_text", id);

        Self {
            id: id.clone(),
            layout: Layout::default(),
            background: Rectangle::new(bg_id).with_corner_radius(4.0),
            text: Text::new(text_id, text).with_color(Color::WHITE),
            normal_color: Color::from_hex("#3b82f6").unwrap_or(Color::BLUE),
            hover_color: Color::from_hex("#2563eb").unwrap_or(Color::BLUE),
            pressed_color: Color::from_hex("#1d4ed8").unwrap_or(Color::BLUE),
            is_hovered: false,
            is_pressed: false,
            visible: true,
            needs_layout: true,
            click_handler: None,
        }
    }

    pub fn with_colors(mut self, normal: Color, hover: Color, pressed: Color) -> Self {
        self.normal_color = normal;
        self.hover_color = hover;
        self.pressed_color = pressed;
        self.update_background_color();
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.click_handler = Some(Box::new(handler));
        self
    }

    fn update_background_color(&mut self) {
        let color = if self.is_pressed {
            self.pressed_color
        } else if self.is_hovered {
            self.hover_color
        } else {
            self.normal_color
        };

        self.background.set_color(color);
    }
}

impl UIElement for Button {
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
        let text_size = self.text.measure(available_size);
        let padding = self.layout.padding.total_size();

        Size::new(
            text_size.width + padding.width + 16.0, // Extra padding for button
            text_size.height + padding.height + 8.0,
        )
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;

        // Arrange background to fill entire bounds
        self.background.arrange(bounds);

        // Center text within button
        let text_size = self.text.measure(bounds.size);
        let text_x = bounds.origin.x + (bounds.size.width - text_size.width) / 2.0;
        let text_y = bounds.origin.y + (bounds.size.height - text_size.height) / 2.0;

        self.text.arrange(Rect::new(text_x, text_y, text_size.width, text_size.height));

        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        let mut commands = Vec::new();
        commands.extend(self.background.render(z_index));
        commands.extend(self.text.render(z_index + 0.1));
        commands
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.background.set_visible(visible);
        self.text.set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate_layout(&mut self) {
        self.needs_layout = true;
        self.background.invalidate_layout();
        self.text.invalidate_layout();
    }

    fn invalidate_visual(&mut self) {
        self.background.invalidate_visual();
        self.text.invalidate_visual();
    }
}

impl EventHandler for Button {
    fn handle_event(&mut self, event: &Event) -> bool {
        use crate::events::{InputEvent, ButtonState, MouseButton};

        match event {
            Event::Input(InputEvent::MouseMove { position, .. }) => {
                let was_hovered = self.is_hovered;
                self.is_hovered = UIElement::bounds(self).contains_point(*position);

                if was_hovered != self.is_hovered {
                    self.update_background_color();
                    return true;
                }
            }
            Event::Input(InputEvent::MouseButton { button: MouseButton::Left, state, position, .. }) => {
                if UIElement::bounds(self).contains_point(*position) {
                    match state {
                        ButtonState::Pressed => {
                            self.is_pressed = true;
                            self.update_background_color();
                            return true;
                        }
                        ButtonState::Released => {
                            if self.is_pressed {
                                self.is_pressed = false;
                                self.update_background_color();

                                // Fire click event
                                if let Some(handler) = &self.click_handler {
                                    handler();
                                }
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        false
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}

/// Container for holding multiple UI elements with layout
pub struct Container {
    id: String,
    layout: Layout,
    children: Vec<Box<dyn UIElement>>,
    layout_direction: LayoutDirection,
    visible: bool,
    needs_layout: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
    Stack, // Children overlap
}

impl Container {
    pub fn new(id: String) -> Self {
        Self {
            id,
            layout: Layout::default(),
            children: Vec::new(),
            layout_direction: LayoutDirection::Vertical,
            visible: true,
            needs_layout: true,
        }
    }

    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.layout_direction = direction;
        self.needs_layout = true;
        self
    }

    pub fn add_child(&mut self, child: Box<dyn UIElement>) {
        self.children.push(child);
        self.needs_layout = true;
    }

    pub fn remove_child(&mut self, id: &str) -> Option<Box<dyn UIElement>> {
        if let Some(index) = self.children.iter().position(|child| child.id() == id) {
            self.needs_layout = true;
            Some(self.children.remove(index))
        } else {
            None
        }
    }
}

impl UIElement for Container {
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
        match self.layout_direction {
            LayoutDirection::Horizontal => {
                let mut total_width = 0.0;
                let mut max_height = 0.0;

                for child in &mut self.children {
                    let child_size = child.measure(available_size);
                    total_width += child_size.width;
                    max_height = f32::max(max_height, child_size.height);
                }

                Size::new(total_width, max_height)
            }
            LayoutDirection::Vertical => {
                let mut max_width = 0.0;
                let mut total_height = 0.0;

                for child in &mut self.children {
                    let child_size = child.measure(available_size);
                    max_width = f32::max(max_width, child_size.width);
                    total_height += child_size.height;
                }

                Size::new(max_width, total_height)
            }
            LayoutDirection::Stack => {
                let mut max_width = 0.0;
                let mut max_height = 0.0;

                for child in &mut self.children {
                    let child_size = child.measure(available_size);
                    max_width = f32::max(max_width, child_size.width);
                    max_height = f32::max(max_height, child_size.height);
                }

                Size::new(max_width, max_height)
            }
        }
    }

    fn arrange(&mut self, bounds: Rect) {
        self.layout.bounds = bounds;

        let content_bounds = Rect::new(
            bounds.origin.x + self.layout.padding.left,
            bounds.origin.y + self.layout.padding.top,
            bounds.size.width - self.layout.padding.horizontal(),
            bounds.size.height - self.layout.padding.vertical(),
        );

        match self.layout_direction {
            LayoutDirection::Horizontal => {
                let child_width = content_bounds.size.width / self.children.len() as f32;
                for (i, child) in self.children.iter_mut().enumerate() {
                    let child_bounds = Rect::new(
                        content_bounds.origin.x + i as f32 * child_width,
                        content_bounds.origin.y,
                        child_width,
                        content_bounds.size.height,
                    );
                    child.arrange(child_bounds);
                }
            }
            LayoutDirection::Vertical => {
                let child_height = content_bounds.size.height / self.children.len() as f32;
                for (i, child) in self.children.iter_mut().enumerate() {
                    let child_bounds = Rect::new(
                        content_bounds.origin.x,
                        content_bounds.origin.y + i as f32 * child_height,
                        content_bounds.size.width,
                        child_height,
                    );
                    child.arrange(child_bounds);
                }
            }
            LayoutDirection::Stack => {
                for child in &mut self.children {
                    child.arrange(content_bounds);
                }
            }
        }

        self.needs_layout = false;
    }

    fn render(&self, z_index: f32) -> Vec<RenderCommand> {
        if !self.visible {
            return Vec::new();
        }

        let mut commands = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            commands.extend(child.render(z_index + i as f32 * 0.1));
        }
        commands
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        for child in &mut self.children {
            child.set_visible(visible);
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn invalidate_layout(&mut self) {
        self.needs_layout = true;
        for child in &mut self.children {
            child.invalidate_layout();
        }
    }

    fn invalidate_visual(&mut self) {
        for child in &mut self.children {
            child.invalidate_visual();
        }
    }
}

impl EventHandler for Container {
    fn handle_event(&mut self, event: &Event) -> bool {
        // Forward events to children (in reverse order for correct hit testing)
        for child in self.children.iter_mut().rev() {
            if child.handle_event(event) {
                return true;
            }
        }
        false
    }

    fn bounds(&self) -> Rect {
        self.layout.bounds
    }
}

/// UI tree for managing the entire UI hierarchy
pub struct UITree {
    root: Option<Box<dyn UIElement>>,
    text_renderer: TextRenderer,
    dirty_elements: HashMap<String, bool>,
}

impl UITree {
    pub fn new() -> Self {
        Self {
            root: None,
            text_renderer: TextRenderer::new(),
            dirty_elements: HashMap::new(),
        }
    }

    pub fn set_root(&mut self, root: Box<dyn UIElement>) {
        self.root = Some(root);
        self.mark_dirty();
    }

    pub fn layout(&mut self, available_size: Size) {
        if let Some(root) = &mut self.root {
            let measured_size = root.measure(available_size);
            let bounds = Rect::new(0.0, 0.0, measured_size.width, measured_size.height);
            root.arrange(bounds);
        }
    }

    pub fn render(&self) -> Vec<RenderLayer> {
        if let Some(root) = &self.root {
            let commands = root.render(0.0);
            vec![RenderLayer {
                z_index: 0.0,
                commands,
                clip_rect: None,
            }]
        } else {
            Vec::new()
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if let Some(root) = &mut self.root {
            root.handle_event(event)
        } else {
            false
        }
    }

    pub fn text_renderer(&mut self) -> &mut TextRenderer {
        &mut self.text_renderer
    }

    fn mark_dirty(&mut self) {
        // Mark entire tree as dirty
        self.dirty_elements.clear();
    }
}

impl Default for UITree {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Advanced UI features to be implemented:
//
// 1. Layout animations with GPU acceleration
//    - Smooth transitions between layout states
//    - Spring physics for natural motion
//    - Keyframe animations
//
// 2. Advanced layout systems
//    - Grid layout with spanning
//    - Flexbox-style layout
//    - Constraint-based layout solver
//
// 3. Rich text and typography
//    - Multi-style text runs
//    - Text selection and editing
//    - Rich text formatting
//
// 4. Interactive elements
//    - Scroll views with momentum
//    - Drag and drop system
//    - Gesture recognition
//
// 5. Theming and styling
//    - CSS-like styling system
//    - Dynamic theme switching
//    - Style inheritance
//
// 6. Performance optimizations
//    - Virtual scrolling for large lists
//    - Render layer caching
//    - Incremental layout updates
//    - GPU-based clipping and masking