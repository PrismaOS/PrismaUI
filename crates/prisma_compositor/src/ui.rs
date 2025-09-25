/// High-performance UI system with GPU-accelerated rendering
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// UI rectangle for layout and rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UIRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UIRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn intersects(&self, other: &UIRect) -> bool {
        !(self.x + self.width < other.x
            || other.x + other.width < self.x
            || self.y + self.height < other.y
            || other.y + other.height < self.y)
    }
}

/// UI element type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UIElementType {
    Rect,
    Text,
    Image,
    Button,
    Panel,
    Window,
}

/// UI element for GPU rendering
#[derive(Debug, Clone)]
pub struct UIElement {
    pub id: u64,
    pub element_type: UIElementType,
    pub rect: UIRect,
    pub color: [f32; 4],
    pub texture_id: Option<u32>,
    pub text_content: Option<String>,
    pub font_id: Option<u32>,
    pub z_index: i32,
    pub visible: bool,
    pub opacity: f32,
    pub clip_rect: Option<UIRect>,
    pub transform: UITransform,
    pub flags: u32,
}

/// UI transformation for advanced effects
#[derive(Debug, Clone, Copy)]
pub struct UITransform {
    pub translation: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub anchor: [f32; 2], // Rotation/scale anchor point
}

impl Default for UITransform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
            anchor: [0.5, 0.5],
        }
    }
}

impl UIElement {
    /// Create a new UI element
    pub fn new(id: u64, element_type: UIElementType, rect: UIRect) -> Self {
        Self {
            id,
            element_type,
            rect,
            color: [1.0, 1.0, 1.0, 1.0],
            texture_id: None,
            text_content: None,
            font_id: None,
            z_index: 0,
            visible: true,
            opacity: 1.0,
            clip_rect: None,
            transform: UITransform::default(),
            flags: 0,
        }
    }

    /// Create a colored rectangle
    pub fn rect(id: u64, rect: UIRect, color: [f32; 4]) -> Self {
        let mut element = Self::new(id, UIElementType::Rect, rect);
        element.color = color;
        element
    }

    /// Create a text element
    pub fn text(id: u64, rect: UIRect, text: String, font_id: u32, color: [f32; 4]) -> Self {
        let mut element = Self::new(id, UIElementType::Text, rect);
        element.text_content = Some(text);
        element.font_id = Some(font_id);
        element.color = color;
        element
    }

    /// Create an image element
    pub fn image(id: u64, rect: UIRect, texture_id: u32, opacity: f32) -> Self {
        let mut element = Self::new(id, UIElementType::Image, rect);
        element.texture_id = Some(texture_id);
        element.opacity = opacity;
        element
    }

    /// Create a button element
    pub fn button(id: u64, rect: UIRect, text: String, font_id: u32) -> Self {
        let mut element = Self::new(id, UIElementType::Button, rect);
        element.text_content = Some(text);
        element.font_id = Some(font_id);
        element.color = [0.3, 0.3, 0.3, 1.0]; // Default button color
        element
    }

    /// Get transformed rect
    pub fn get_transformed_rect(&self) -> UIRect {
        let mut rect = self.rect;

        // Apply translation
        rect.x += self.transform.translation[0];
        rect.y += self.transform.translation[1];

        // Apply scale
        rect.width *= self.transform.scale[0];
        rect.height *= self.transform.scale[1];

        // Note: Rotation would require more complex transformation
        // For simplicity, we'll skip it in this example

        rect
    }

    /// Check if element is interactive
    pub fn is_interactive(&self) -> bool {
        matches!(
            self.element_type,
            UIElementType::Button | UIElementType::Window
        )
    }
}

/// UI text rendering
#[derive(Debug, Clone)]
pub struct UIText {
    pub text: String,
    pub font_id: u32,
    pub size: f32,
    pub color: [f32; 4],
    pub position: [f32; 2],
    pub max_width: Option<f32>,
    pub line_height: f32,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// UI image rendering
#[derive(Debug, Clone)]
pub struct UIImage {
    pub texture_id: u32,
    pub src_rect: Option<UIRect>, // Source rectangle in texture
    pub dst_rect: UIRect,         // Destination rectangle on screen
    pub tint_color: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub opacity: f32,
}

/// UI layer for organizing rendering
#[derive(Debug, Clone)]
pub struct UILayer {
    pub id: u64,
    pub name: String,
    pub elements: Vec<UIElement>,
    pub visible: bool,
    pub opacity: f32,
    pub clip_rect: Option<UIRect>,
    pub z_index: i32,
}

impl UILayer {
    /// Create a new UI layer
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            elements: Vec::new(),
            visible: true,
            opacity: 1.0,
            clip_rect: None,
            z_index: 0,
        }
    }

    /// Add an element to the layer
    pub fn add_element(&mut self, element: UIElement) {
        self.elements.push(element);
        // Sort by z-index for proper rendering order
        self.elements.sort_by_key(|e| e.z_index);
    }

    /// Remove an element by ID
    pub fn remove_element(&mut self, id: u64) {
        self.elements.retain(|e| e.id != id);
    }

    /// Find element by position (for hit testing)
    pub fn find_element_at_position(&self, x: f32, y: f32) -> Option<&UIElement> {
        // Search from highest z-index to lowest (reverse order)
        for element in self.elements.iter().rev() {
            if element.visible && element.get_transformed_rect().contains_point(x, y) {
                if element.is_interactive() {
                    return Some(element);
                }
            }
        }
        None
    }

    /// Get elements that need to be redrawn in a specific region
    pub fn get_dirty_elements(&self, dirty_rect: &UIRect) -> Vec<&UIElement> {
        self.elements
            .iter()
            .filter(|element| {
                element.visible && element.get_transformed_rect().intersects(dirty_rect)
            })
            .collect()
    }
}

/// UI system managing layers and rendering
pub struct UISystem {
    layers: Arc<RwLock<HashMap<u64, UILayer>>>,
    layer_order: Arc<RwLock<Vec<u64>>>, // Ordered layer IDs for rendering
    dirty_regions: Arc<RwLock<Vec<UIRect>>>,
    screen_size: [f32; 2],
    element_id_counter: std::sync::atomic::AtomicU64,
}

impl UISystem {
    /// Create a new UI system
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            layers: Arc::new(RwLock::new(HashMap::new())),
            layer_order: Arc::new(RwLock::new(Vec::new())),
            dirty_regions: Arc::new(RwLock::new(Vec::new())),
            screen_size: [screen_width, screen_height],
            element_id_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Create a new layer
    pub fn create_layer(&self, name: String) -> u64 {
        let layer_id = self.generate_id();
        let layer = UILayer::new(layer_id, name);

        {
            let mut layers = self.layers.write().unwrap();
            layers.insert(layer_id, layer);
        }

        {
            let mut layer_order = self.layer_order.write().unwrap();
            layer_order.push(layer_id);
        }

        layer_id
    }

    /// Add element to a layer
    pub fn add_element_to_layer(&self, layer_id: u64, element: UIElement) {
        if let Ok(mut layers) = self.layers.write() {
            if let Some(layer) = layers.get_mut(&layer_id) {
                let element_rect = element.get_transformed_rect();
                layer.add_element(element);

                // Mark region as dirty
                self.mark_dirty_region(element_rect);
            }
        }
    }

    /// Remove element from layer
    pub fn remove_element(&self, layer_id: u64, element_id: u64) {
        if let Ok(mut layers) = self.layers.write() {
            if let Some(layer) = layers.get_mut(&layer_id) {
                // Find element to get its rect before removal
                if let Some(element) = layer.elements.iter().find(|e| e.id == element_id) {
                    let element_rect = element.get_transformed_rect();
                    layer.remove_element(element_id);

                    // Mark region as dirty
                    self.mark_dirty_region(element_rect);
                }
            }
        }
    }

    /// Update element in layer
    pub fn update_element(&self, layer_id: u64, element: UIElement) {
        if let Ok(mut layers) = self.layers.write() {
            if let Some(layer) = layers.get_mut(&layer_id) {
                // Mark old and new positions as dirty
                if let Some(old_element) = layer.elements.iter().find(|e| e.id == element.id) {
                    let old_rect = old_element.get_transformed_rect();
                    self.mark_dirty_region(old_rect);
                }

                let new_rect = element.get_transformed_rect();
                self.mark_dirty_region(new_rect);

                // Update element
                layer.remove_element(element.id);
                layer.add_element(element);
            }
        }
    }

    /// Handle mouse input (for UI interaction)
    pub fn handle_mouse_input(&self, x: f32, y: f32, button: u32, pressed: bool) -> Option<u64> {
        let layers = self.layers.read().unwrap();
        let layer_order = self.layer_order.read().unwrap();

        // Check layers from top to bottom
        for &layer_id in layer_order.iter().rev() {
            if let Some(layer) = layers.get(&layer_id) {
                if let Some(element) = layer.find_element_at_position(x, y) {
                    // Handle button interaction
                    if element.is_interactive() && pressed {
                        return Some(element.id);
                    }
                }
            }
        }

        None
    }

    /// Mark a region as needing redraw
    pub fn mark_dirty_region(&self, rect: UIRect) {
        if let Ok(mut dirty_regions) = self.dirty_regions.write() {
            dirty_regions.push(rect);
        }
    }

    /// Get and clear dirty regions
    pub fn get_dirty_regions(&self) -> Vec<UIRect> {
        if let Ok(mut dirty_regions) = self.dirty_regions.write() {
            let regions = dirty_regions.clone();
            dirty_regions.clear();
            regions
        } else {
            Vec::new()
        }
    }

    /// Get layers in render order
    pub fn get_layers_for_rendering(&self) -> Vec<UILayer> {
        let layers = self.layers.read().unwrap();
        let layer_order = self.layer_order.read().unwrap();

        layer_order
            .iter()
            .filter_map(|&id| layers.get(&id))
            .filter(|layer| layer.visible)
            .cloned()
            .collect()
    }

    /// Update screen size
    pub fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = [width, height];

        // Mark entire screen as dirty
        self.mark_dirty_region(UIRect::new(0.0, 0.0, width, height));
    }

    /// Generate unique element ID
    fn generate_id(&self) -> u64 {
        self.element_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}