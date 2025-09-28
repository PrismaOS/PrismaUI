// Rounded rectangle shader with GPU-based SDF rendering
// Optimized for smooth anti-aliased rounded corners

struct Uniforms {
    projection: mat4x4<f32>,
    time: f32,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_position: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.clip_position = uniforms.projection * vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    output.uv = input.uv;
    output.world_position = input.position;

    return output;
}

// Signed distance function for rounded rectangle
fn sdf_rounded_rect(pos: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(pos) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // TODO: Pass these as uniforms or vertex attributes
    let rect_size = vec2<f32>(100.0, 50.0); // This should come from uniform
    let corner_radius = 8.0; // This should come from uniform

    // Convert UV to centered coordinates
    let center_pos = (input.uv - vec2<f32>(0.5)) * rect_size;

    // Calculate SDF distance
    let distance = sdf_rounded_rect(center_pos, rect_size * 0.5, corner_radius);

    // Anti-aliased alpha based on distance
    let alpha = 1.0 - smoothstep(-1.0, 1.0, distance);

    var output_color = input.color;
    output_color.a *= alpha;

    return output_color;
}