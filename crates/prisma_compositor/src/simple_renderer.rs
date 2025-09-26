/// Simple immediate-mode renderer for basic UI elements
use std::sync::Arc;
use wgpu::*;
use bytemuck::{Pod, Zeroable};

/// Simple vertex for UI rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl SimpleVertex {
    const ATTRIBS: [VertexAttribute; 2] = [
        VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32x2,
        },
        VertexAttribute {
            offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
            shader_location: 1,
            format: VertexFormat::Float32x4,
        },
    ];

    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<SimpleVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Simple immediate-mode renderer
pub struct SimpleRenderer {
    device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

impl SimpleRenderer {
    /// Create a new simple renderer
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, format: TextureFormat) -> Self {
        // Create shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Simple Shader"),
            source: ShaderSource::Wgsl(r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = color;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
            "#.into()),
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Simple Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Simple Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SimpleVertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create buffers for rendering
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Simple Vertex Buffer"),
            size: 65536, // 64KB for vertices
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Simple Index Buffer"),
            size: 32768, // 32KB for indices
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    /// Begin a UI render pass - call this once per frame
    pub fn begin_ui_pass<'a>(&'a self, encoder: &'a mut CommandEncoder, target: &'a TextureView) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("UI Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // Don't clear, preserve existing content
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }

    /// Render a rectangle within an existing render pass
    pub fn render_rect_in_pass<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        screen_width: f32,
        screen_height: f32,
    ) {
        // Convert screen coordinates to NDC - COMPLETELY REWRITTEN
        // Standard OpenGL/WGPU NDC conversion
        let x1 = (x / screen_width) * 2.0 - 1.0;                    // Left edge
        let x2 = ((x + width) / screen_width) * 2.0 - 1.0;          // Right edge
        let y1 = -((y / screen_height) * 2.0 - 1.0);                // Top edge (flip Y)
        let y2 = -(((y + height) / screen_height) * 2.0 - 1.0);     // Bottom edge (flip Y)

        println!("       Screen: ({}, {}, {}x{}) -> NDC: ({:.2}, {:.2}) to ({:.2}, {:.2})",
            x, y, width, height, x1, y1, x2, y2);

        // Create rectangle as two triangles (6 vertices) - NO INDICES
        let triangles = [
            // First triangle (top-left, top-right, bottom-left)
            SimpleVertex { position: [x1, y1], color }, // Top-left
            SimpleVertex { position: [x2, y1], color }, // Top-right
            SimpleVertex { position: [x1, y2], color }, // Bottom-left
            // Second triangle (top-right, bottom-right, bottom-left)
            SimpleVertex { position: [x2, y1], color }, // Top-right
            SimpleVertex { position: [x2, y2], color }, // Bottom-right
            SimpleVertex { position: [x1, y2], color }, // Bottom-left
        ];

        println!("       Triangle vertices: TL({:.2},{:.2}) TR({:.2},{:.2}) BL({:.2},{:.2})",
            x1, y1, x2, y1, x1, y2);
        println!("       Color: [{:.2}, {:.2}, {:.2}, {:.2}]", color[0], color[1], color[2], color[3]);

        // Upload triangle data
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&triangles));

        // Set buffers and draw as triangles (no indices)
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        println!("       Issuing draw(0..6, 0..1) - two triangles");
        render_pass.draw(0..6, 0..1);
        println!("       Draw call completed");
    }

    /// Render a rectangle in its own render pass (preserves existing content)
    pub fn render_rect(
        &self,
        encoder: &mut CommandEncoder,
        target: &TextureView,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        screen_width: f32,
        screen_height: f32,
    ) {
        // Convert screen coordinates to NDC - COMPLETELY REWRITTEN
        // Standard OpenGL/WGPU NDC conversion
        let x1 = (x / screen_width) * 2.0 - 1.0;                    // Left edge
        let x2 = ((x + width) / screen_width) * 2.0 - 1.0;          // Right edge
        let y1 = -((y / screen_height) * 2.0 - 1.0);                // Top edge (flip Y)
        let y2 = -(((y + height) / screen_height) * 2.0 - 1.0);     // Bottom edge (flip Y)

        // Debug output for coordinate conversion
        println!("       Screen: ({}, {}, {}x{}) -> NDC: ({:.2}, {:.2}) to ({:.2}, {:.2})",
            x, y, width, height, x1, y1, x2, y2);

        // Create vertices for rectangle
        let vertices = [
            SimpleVertex { position: [x1, y1], color },  // Top-left
            SimpleVertex { position: [x2, y1], color },  // Top-right
            SimpleVertex { position: [x2, y2], color },  // Bottom-right
            SimpleVertex { position: [x1, y2], color },  // Bottom-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        // Upload vertex data to buffer
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        // Create render pass that preserves existing content
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Rectangle Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // Preserve existing content
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Set pipeline and buffers
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);

        // Draw the rectangle
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}