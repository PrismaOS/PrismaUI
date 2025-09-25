// High-performance image rendering with advanced blending

struct RenderUniforms {
    view_projection: mat4x4<f32>,
    screen_size: vec2<f32>,
    time: f32,
    frame_count: u32,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct InstanceInput {
    @location(3) instance_position: vec2<f32>,
    @location(4) instance_size: vec2<f32>,
    @location(5) instance_color: vec4<f32>,
    @location(6) instance_tex_coords: vec4<f32>,
    @location(7) texture_index: u32,
    @location(8) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) texture_index: u32,
    @location(3) flags: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: RenderUniforms;

@group(1) @binding(0)
var image_array: texture_2d_array<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Transform vertex position
    var world_position = vertex.position * instance.instance_size + instance.instance_position;

    // Convert to NDC
    var ndc_position = world_position / uniforms.screen_size * 2.0 - 1.0;
    ndc_position.y = -ndc_position.y;

    out.clip_position = vec4<f32>(ndc_position, 0.0, 1.0);

    // Map texture coordinates
    out.tex_coords = mix(
        instance.instance_tex_coords.xy,
        instance.instance_tex_coords.xy + instance.instance_tex_coords.zw,
        vertex.tex_coords
    );

    out.color = vertex.color * instance.instance_color;
    out.texture_index = instance.texture_index;
    out.flags = instance.flags;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample base image
    var image_color = textureSample(
        image_array,
        image_sampler,
        in.tex_coords,
        i32(in.texture_index)
    );

    // Apply tinting
    var final_color = image_color * in.color;

    // Apply effects based on flags
    if ((in.flags & 1u) != 0u) {
        // Grayscale effect
        var luminance = dot(final_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        final_color = vec4<f32>(vec3<f32>(luminance), final_color.a);
    }

    if ((in.flags & 2u) != 0u) {
        // Sepia effect
        var sepia_r = dot(final_color.rgb, vec3<f32>(0.393, 0.769, 0.189));
        var sepia_g = dot(final_color.rgb, vec3<f32>(0.349, 0.686, 0.168));
        var sepia_b = dot(final_color.rgb, vec3<f32>(0.272, 0.534, 0.131));
        final_color = vec4<f32>(sepia_r, sepia_g, sepia_b, final_color.a);
    }

    return final_color;
}