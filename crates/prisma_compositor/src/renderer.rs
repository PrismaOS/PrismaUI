/// High-performance GPU renderer for UI elements
use wgpu::{RenderPass, RenderPipeline, BindGroup};
use crate::{
    core::{Device, Context, Buffer as TypedBuffer},
    geometry::{Vertex, Color, Rect, Transform},
};

/// Render command for GPU execution - highly optimized
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Draw a solid rectangle with transform
    Rectangle {
        rect: Rect,
        color: Color,
        transform: Transform,
        z_index: f32,
    },
    /// Draw a textured rectangle (image, icon, etc.)
    TexturedRectangle {
        rect: Rect,
        texture_id: u32,
        uv_rect: Rect, // UV coordinates in texture
        color: Color,  // Tint color
        transform: Transform,
        z_index: f32,
    },
    /// Draw text (uses glyph atlas)
    Text {
        rect: Rect,
        glyph_atlas_region: Rect,
        color: Color,
        transform: Transform,
        z_index: f32,
    },
    /// Draw a rounded rectangle
    RoundedRectangle {
        rect: Rect,
        corner_radius: f32,
        color: Color,
        transform: Transform,
        z_index: f32,
    },
    /// Draw a gradient rectangle
    GradientRectangle {
        rect: Rect,
        start_color: Color,
        end_color: Color,
        direction: f32, // angle in radians
        transform: Transform,
        z_index: f32,
    },
    // TODO: Animation commands will be added here
    // AnimatedRectangle { ... },
    // AnimatedTransform { ... },
}

impl RenderCommand {
    pub fn z_index(&self) -> f32 {
        match self {
            RenderCommand::Rectangle { z_index, .. }
            | RenderCommand::TexturedRectangle { z_index, .. }
            | RenderCommand::Text { z_index, .. }
            | RenderCommand::RoundedRectangle { z_index, .. }
            | RenderCommand::GradientRectangle { z_index, .. } => *z_index,
        }
    }
}

/// Render layer for depth sorting and batching
#[derive(Debug, Clone)]
pub struct RenderLayer {
    pub z_index: f32,
    pub commands: Vec<RenderCommand>,
    pub clip_rect: Option<Rect>,
}

impl RenderLayer {
    pub fn new(z_index: f32) -> Self {
        Self {
            z_index,
            commands: Vec::new(),
            clip_rect: None,
        }
    }

    pub fn add_command(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    pub fn set_clip_rect(&mut self, rect: Rect) {
        self.clip_rect = Some(rect);
    }

    pub fn sort_commands_by_z(&mut self) {
        self.commands.sort_by(|a, b| a.z_index().partial_cmp(&b.z_index()).unwrap());
    }
}

/// Uniform data for shaders
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub projection: [[f32; 4]; 4],
    pub time: f32,
    pub _padding: [f32; 3],
}

/// Batch for efficient GPU rendering
#[derive(Debug)]
pub struct RenderBatch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture_id: Option<u32>,
    pub shader_type: ShaderType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderType {
    Solid,
    Textured,
    Text,
    RoundedRect,
    Gradient,
    // TODO: Add animation shader types
    // AnimatedSolid,
    // AnimatedTextured,
}

impl RenderBatch {
    pub fn new(shader_type: ShaderType) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture_id: None,
            shader_type,
        }
    }

    pub fn add_quad(&mut self, vertices: [Vertex; 4]) {
        let start_index = self.vertices.len() as u32;

        // Add vertices
        self.vertices.extend_from_slice(&vertices);

        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            start_index, start_index + 1, start_index + 2,
            start_index, start_index + 2, start_index + 3,
        ]);
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.texture_id = None;
    }
}

/// High-performance renderer optimized for UI
pub struct Renderer {
    // Render pipelines for different shader types
    solid_pipeline: RenderPipeline,
    textured_pipeline: RenderPipeline,
    text_pipeline: RenderPipeline,
    rounded_rect_pipeline: RenderPipeline,
    gradient_pipeline: RenderPipeline,

    // GPU buffers with dynamic sizing
    vertex_buffer: TypedBuffer<Vertex>,
    index_buffer: TypedBuffer<u32>,
    uniform_buffer: TypedBuffer<Uniforms>,

    // Bind groups
    uniform_bind_group: BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    // Batching system
    batches: Vec<RenderBatch>,
    current_batch: Option<RenderBatch>,

    // Stats for performance monitoring
    pub stats: RenderStats,
}

#[derive(Debug, Default)]
pub struct RenderStats {
    pub draw_calls: u32,
    pub vertices_rendered: u32,
    pub triangles_rendered: u32,
    pub batches_created: u32,
    pub frame_time_ms: f32,
}

impl Renderer {
    /// Create new renderer with optimized pipelines
    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> anyhow::Result<Self> {
        // Create uniform buffer
        let mut uniform_buffer = TypedBuffer::new(
            &device.device,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            1,
            Some("Uniform Buffer"),
        );

        // Initialize with identity projection
        let uniforms = Uniforms {
            projection: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            time: 0.0,
            _padding: [0.0; 3],
        };
        uniform_buffer.write(&device.queue, &[uniforms]);

        // Create bind group layouts
        let uniform_bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create uniform bind group
        let uniform_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.buffer.as_entire_binding(),
            }],
        });

        // Create shaders
        let solid_shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/solid.wgsl").into()),
        });

        let textured_shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Textured Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/textured.wgsl").into()),
        });

        let text_shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let rounded_rect_shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rounded Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rounded_rect.wgsl").into()),
        });

        let gradient_shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gradient Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gradient.wgsl").into()),
        });

        // Create render pipeline layout
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipelines
        let solid_pipeline = Self::create_render_pipeline(
            &device.device,
            &pipeline_layout,
            &solid_shader,
            surface_format,
            "Solid Pipeline",
        );

        let textured_pipeline = Self::create_render_pipeline(
            &device.device,
            &pipeline_layout,
            &textured_shader,
            surface_format,
            "Textured Pipeline",
        );

        let text_pipeline = Self::create_render_pipeline(
            &device.device,
            &pipeline_layout,
            &text_shader,
            surface_format,
            "Text Pipeline",
        );

        let rounded_rect_pipeline = Self::create_render_pipeline(
            &device.device,
            &pipeline_layout,
            &rounded_rect_shader,
            surface_format,
            "Rounded Rect Pipeline",
        );

        let gradient_pipeline = Self::create_render_pipeline(
            &device.device,
            &pipeline_layout,
            &gradient_shader,
            surface_format,
            "Gradient Pipeline",
        );

        // Create dynamic buffers
        let vertex_buffer = TypedBuffer::new(
            &device.device,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            65536, // Start with space for many vertices
            Some("Vertex Buffer"),
        );

        let index_buffer = TypedBuffer::new(
            &device.device,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            98304, // 65536 * 1.5 for typical UI geometry
            Some("Index Buffer"),
        );

        Ok(Self {
            solid_pipeline,
            textured_pipeline,
            text_pipeline,
            rounded_rect_pipeline,
            gradient_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            batches: Vec::new(),
            current_batch: None,
            stats: RenderStats::default(),
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        label: &str,
    ) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for UI
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None, // 2D UI doesn't need depth testing
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    /// Begin frame - reset stats and batches
    pub fn begin_frame(&mut self, context: &Context, time: f32) {
        // Reset stats
        self.stats = RenderStats::default();

        // Clear batches
        self.batches.clear();
        self.current_batch = None;

        // Update uniforms
        let uniforms = Uniforms {
            projection: context.orthographic_projection(),
            time,
            _padding: [0.0; 3],
        };
        self.uniform_buffer.write(&context.device.queue, &[uniforms]);
    }

    /// Add render layers (automatically sorts by z-index)
    pub fn add_layers(&mut self, mut layers: Vec<RenderLayer>) {
        // Sort layers by z-index
        layers.sort_by(|a, b| a.z_index.partial_cmp(&b.z_index).unwrap());

        // Process each layer
        for mut layer in layers {
            layer.sort_commands_by_z();

            for command in layer.commands {
                self.add_command(command);
            }
        }
    }

    /// Add a single render command (batches automatically)
    fn add_command(&mut self, command: RenderCommand) {
        match command {
            RenderCommand::Rectangle { rect, color, transform, .. } => {
                self.add_rectangle(rect, color, transform);
            }
            RenderCommand::TexturedRectangle { rect, texture_id, uv_rect, color, transform, .. } => {
                self.add_textured_rectangle(rect, texture_id, uv_rect, color, transform);
            }
            RenderCommand::Text { rect, glyph_atlas_region, color, transform, .. } => {
                self.add_text_quad(rect, glyph_atlas_region, color, transform);
            }
            RenderCommand::RoundedRectangle { rect, corner_radius, color, transform, .. } => {
                self.add_rounded_rectangle(rect, corner_radius, color, transform);
            }
            RenderCommand::GradientRectangle { rect, start_color, end_color, direction, transform, .. } => {
                self.add_gradient_rectangle(rect, start_color, end_color, direction, transform);
            }
        }
    }

    /// Ensure we have a batch for the given shader type
    fn ensure_batch(&mut self, shader_type: ShaderType, texture_id: Option<u32>) {
        let needs_new_batch = match &self.current_batch {
            None => true,
            Some(batch) => batch.shader_type != shader_type || batch.texture_id != texture_id,
        };

        if needs_new_batch {
            // Finish current batch
            if let Some(batch) = self.current_batch.take() {
                if !batch.is_empty() {
                    self.batches.push(batch);
                    self.stats.batches_created += 1;
                }
            }

            // Start new batch
            let mut new_batch = RenderBatch::new(shader_type);
            new_batch.texture_id = texture_id;
            self.current_batch = Some(new_batch);
        }
    }

    fn add_rectangle(&mut self, rect: Rect, color: Color, transform: Transform) {
        self.ensure_batch(ShaderType::Solid, None);

        if let Some(batch) = &mut self.current_batch {
            let vertices = Self::create_quad_vertices(rect, color, transform);
            batch.add_quad(vertices);
        }
    }

    fn add_textured_rectangle(&mut self, rect: Rect, texture_id: u32, uv_rect: Rect, color: Color, transform: Transform) {
        self.ensure_batch(ShaderType::Textured, Some(texture_id));

        if let Some(batch) = &mut self.current_batch {
            let vertices = Self::create_textured_quad_vertices(rect, uv_rect, color, transform);
            batch.add_quad(vertices);
        }
    }

    fn add_text_quad(&mut self, rect: Rect, uv_rect: Rect, color: Color, transform: Transform) {
        // Text uses the atlas texture (ID 0 by convention)
        self.ensure_batch(ShaderType::Text, Some(0));

        if let Some(batch) = &mut self.current_batch {
            let vertices = Self::create_textured_quad_vertices(rect, uv_rect, color, transform);
            batch.add_quad(vertices);
        }
    }

    fn add_rounded_rectangle(&mut self, rect: Rect, _corner_radius: f32, color: Color, transform: Transform) {
        self.ensure_batch(ShaderType::RoundedRect, None);

        if let Some(batch) = &mut self.current_batch {
            let vertices = Self::create_quad_vertices(rect, color, transform);
            batch.add_quad(vertices);
        }
    }

    fn add_gradient_rectangle(&mut self, rect: Rect, start_color: Color, end_color: Color, _direction: f32, transform: Transform) {
        self.ensure_batch(ShaderType::Gradient, None);

        if let Some(batch) = &mut self.current_batch {
            // For gradient, we use vertex colors to interpolate
            let vertices = Self::create_gradient_quad_vertices(rect, start_color, end_color, transform);
            batch.add_quad(vertices);
        }
    }

    fn create_quad_vertices(rect: Rect, color: Color, transform: Transform) -> [Vertex; 4] {
        use crate::geometry::Point;

        let tl = transform.transform_point(rect.origin);
        let tr = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y));
        let bl = transform.transform_point(Point::new(rect.origin.x, rect.origin.y + rect.size.height));
        let br = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y + rect.size.height));

        [
            Vertex::new(tl, Point::new(0.0, 0.0), color),
            Vertex::new(tr, Point::new(1.0, 0.0), color),
            Vertex::new(br, Point::new(1.0, 1.0), color),
            Vertex::new(bl, Point::new(0.0, 1.0), color),
        ]
    }

    fn create_textured_quad_vertices(rect: Rect, uv_rect: Rect, color: Color, transform: Transform) -> [Vertex; 4] {
        use crate::geometry::Point;

        let tl = transform.transform_point(rect.origin);
        let tr = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y));
        let bl = transform.transform_point(Point::new(rect.origin.x, rect.origin.y + rect.size.height));
        let br = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y + rect.size.height));

        let uv_tl = uv_rect.origin;
        let uv_tr = Point::new(uv_rect.origin.x + uv_rect.size.width, uv_rect.origin.y);
        let uv_bl = Point::new(uv_rect.origin.x, uv_rect.origin.y + uv_rect.size.height);
        let uv_br = Point::new(uv_rect.origin.x + uv_rect.size.width, uv_rect.origin.y + uv_rect.size.height);

        [
            Vertex::new(tl, uv_tl, color),
            Vertex::new(tr, uv_tr, color),
            Vertex::new(br, uv_br, color),
            Vertex::new(bl, uv_bl, color),
        ]
    }

    fn create_gradient_quad_vertices(rect: Rect, start_color: Color, end_color: Color, transform: Transform) -> [Vertex; 4] {
        use crate::geometry::Point;

        let tl = transform.transform_point(rect.origin);
        let tr = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y));
        let bl = transform.transform_point(Point::new(rect.origin.x, rect.origin.y + rect.size.height));
        let br = transform.transform_point(Point::new(rect.origin.x + rect.size.width, rect.origin.y + rect.size.height));

        // Simple top-to-bottom gradient
        [
            Vertex::new(tl, Point::new(0.0, 0.0), start_color),
            Vertex::new(tr, Point::new(1.0, 0.0), start_color),
            Vertex::new(br, Point::new(1.0, 1.0), end_color),
            Vertex::new(bl, Point::new(0.0, 1.0), end_color),
        ]
    }

    /// Finish frame and render all batches
    pub fn end_frame(&mut self, _render_pass: &mut RenderPass<'_>, _device: &Device) -> anyhow::Result<()> {
        // Finalize current batch
        if let Some(batch) = self.current_batch.take() {
            if !batch.is_empty() {
                self.batches.push(batch);
                self.stats.batches_created += 1;
            }
        }

        // Collect all vertices and indices
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut index_offset = 0u32;

        for batch in &self.batches {
            all_vertices.extend_from_slice(&batch.vertices);

            // Adjust indices for concatenated buffer
            for &index in &batch.indices {
                all_indices.push(index + index_offset);
            }
            index_offset += batch.vertices.len() as u32;
        }

        if all_vertices.is_empty() {
            return Ok(());
        }

        // TODO: Actual GPU rendering implementation
        // For now, just update stats
        self.stats.draw_calls = self.batches.len() as u32;
        self.stats.vertices_rendered = all_vertices.len() as u32;
        self.stats.triangles_rendered = all_indices.len() as u32 / 3;

        Ok(())
    }
}