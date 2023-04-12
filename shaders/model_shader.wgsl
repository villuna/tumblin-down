struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
    // Don't need the normal just yet
};

struct Camera {
    @location(0) matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Orthographic projection
    // DO NOT RENDER A 3D MODEL OF A HUMAN WITH AN ORTHOGRAPHIC PROJECTION
    // WEIRDEST SHIT IVE EVER SEEN
    // out.clip_position = vec4<f32>(in.position.x/2.0, in.position.y/2.0 - 1.0, in.position.z, 1.0);

    // Perspective projection using the camera uniform binding
    out.clip_position = camera.matrix * vec4<f32>(in.position, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

@group(1) @binding(0)
var diffuse_texture: texture_2d<f32>;
@group(1) @binding(1)
var diffuse_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
}