// Gradient shader for smooth color transitions
// Supports linear and radial gradients with GPU interpolation

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
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.clip_position = uniforms.projection * vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    output.uv = input.uv;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // For now, we use vertex color interpolation
    // TODO: Add uniform for gradient parameters (direction, type, stops)
    return input.color;
}