// High-performance quad shader with instanced rendering

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
}

@group(0) @binding(0)
var<uniform> uniforms: RenderUniforms;

@group(1) @binding(0)
var texture_array: texture_2d_array<f32>;

@group(1) @binding(1)
var texture_sampler: sampler;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Scale vertex position by instance size and translate by instance position
    var world_position = vertex.position * instance.instance_size + instance.instance_position;

    // Convert to normalized device coordinates
    var ndc_position = world_position / uniforms.screen_size * 2.0 - 1.0;
    ndc_position.y = -ndc_position.y; // Flip Y for screen coordinates

    out.clip_position = vec4<f32>(ndc_position, 0.0, 1.0);

    // Interpolate texture coordinates
    out.tex_coords = mix(
        instance.instance_tex_coords.xy,
        instance.instance_tex_coords.xy + instance.instance_tex_coords.zw,
        vertex.tex_coords
    );

    // Combine vertex and instance colors
    out.color = vertex.color * instance.instance_color;
    out.texture_index = instance.texture_index;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color = in.color;

    // Sample texture if texture index is valid
    if (in.texture_index > 0u) {
        var texture_color = textureSample(
            texture_array,
            texture_sampler,
            in.tex_coords,
            i32(in.texture_index - 1u)
        );
        base_color = base_color * texture_color;
    }

    return base_color;
}