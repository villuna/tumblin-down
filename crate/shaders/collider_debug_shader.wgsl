// Shader for drawing a collider mesh with a single transparent colour
// Ideally i will do a wireframe pass as well

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct Camera {
    position: vec4<f32>,
    matrix: mat4x4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0) 
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.matrix * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.26, 0.65, 0.96, 0.6);
}