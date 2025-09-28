/// Geometry primitives for the GPU-accelerated UI system
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// 2D Point with f32 coordinates - GPU optimized
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// 2D Size with f32 dimensions - GPU optimized
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self { width: 0.0, height: 0.0 };

    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height != 0.0 {
            self.width / self.height
        } else {
            0.0
        }
    }
}

/// Rectangle defined by origin point and size - GPU optimized
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        let min_x = p1.x.min(p2.x);
        let min_y = p1.y.min(p2.y);
        let max_x = p1.x.max(p2.x);
        let max_y = p1.y.max(p2.y);

        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn contains_point(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.size.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.origin.x + self.size.width < other.origin.x
            || other.origin.x + other.size.width < self.origin.x
            || self.origin.y + self.size.height < other.origin.y
            || other.origin.y + other.size.height < self.origin.y)
    }

    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width / 2.0,
            self.origin.y + self.size.height / 2.0,
        )
    }

    pub fn min_corner(&self) -> Point {
        self.origin
    }

    pub fn max_corner(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width,
            self.origin.y + self.size.height,
        )
    }
}

/// 2D Transform matrix for GPU-accelerated transformations
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct Transform {
    /// 3x3 matrix stored in column-major order for GPU efficiency
    pub matrix: [f32; 9],
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0,  // column 1
                0.0, 1.0, 0.0,  // column 2
                0.0, 0.0, 1.0,  // column 3
            ],
        }
    }

    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                x,   y,   1.0,
            ],
        }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            matrix: [
                sx,  0.0, 0.0,
                0.0, sy,  0.0,
                0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotate(angle_radians: f32) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();

        Self {
            matrix: [
                cos_a, sin_a, 0.0,
                -sin_a, cos_a, 0.0,
                0.0,   0.0,   1.0,
            ],
        }
    }

    pub fn multiply(&self, other: &Transform) -> Transform {
        let a = &self.matrix;
        let b = &other.matrix;

        Transform {
            matrix: [
                a[0] * b[0] + a[3] * b[1] + a[6] * b[2],
                a[1] * b[0] + a[4] * b[1] + a[7] * b[2],
                a[2] * b[0] + a[5] * b[1] + a[8] * b[2],

                a[0] * b[3] + a[3] * b[4] + a[6] * b[5],
                a[1] * b[3] + a[4] * b[4] + a[7] * b[5],
                a[2] * b[3] + a[5] * b[4] + a[8] * b[5],

                a[0] * b[6] + a[3] * b[7] + a[6] * b[8],
                a[1] * b[6] + a[4] * b[7] + a[7] * b[8],
                a[2] * b[6] + a[5] * b[7] + a[8] * b[8],
            ],
        }
    }

    pub fn transform_point(&self, point: Point) -> Point {
        let m = &self.matrix;
        Point::new(
            m[0] * point.x + m[3] * point.y + m[6],
            m[1] * point.x + m[4] * point.y + m[7],
        )
    }

    pub fn transform_rect(&self, rect: Rect) -> Rect {
        let corners = [
            self.transform_point(rect.min_corner()),
            self.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y)),
            self.transform_point(rect.max_corner()),
            self.transform_point(Point::new(rect.origin.x, rect.origin.y + rect.size.height)),
        ];

        let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_x = corners.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let max_y = corners.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// RGBA Color with f32 components for GPU efficiency
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgba(rgba: u32) -> Self {
        Self {
            r: ((rgba >> 24) & 0xFF) as f32 / 255.0,
            g: ((rgba >> 16) & 0xFF) as f32 / 255.0,
            b: ((rgba >> 8) & 0xFF) as f32 / 255.0,
            a: (rgba & 0xFF) as f32 / 255.0,
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self, &'static str> {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex format")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex format")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex format")?;
                Ok(Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex format")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex format")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex format")?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "Invalid hex format")?;
                Ok(Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0))
            }
            _ => Err("Hex color must be 6 or 8 characters"),
        }
    }

    pub fn with_alpha(&self, alpha: f32) -> Self {
        Self { a: alpha, ..*self }
    }

    pub fn premultiply_alpha(&self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }
}

/// Vertex for GPU rendering - optimized layout
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(position: Point, uv: Point, color: Color) -> Self {
        Self {
            position: [position.x, position.y],
            uv: [uv.x, uv.y],
            color: [color.r, color.g, color.b, color.a],
        }
    }
}

/// GPU buffer layout description for vertices
impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // UV coordinates
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}