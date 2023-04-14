struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct Camera {
    position: vec4<f32>,
    matrix: mat4x4<f32>,
};

struct Light {
    position: vec3<f32>,
    colour: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> light: Light;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Perspective projection using the camera uniform binding
    let scale = 0.25;
    out.clip_position = camera.matrix * vec4<f32>(in.position * scale + light.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(light.colour, 1.0);
}