struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
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

@group(2) @binding(0)
var<uniform> light: Light;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Orthographic projection
    // DO NOT RENDER A 3D MODEL OF A HUMAN WITH AN ORTHOGRAPHIC PROJECTION
    // WEIRDEST SHIT IVE EVER SEEN
    // out.clip_position = vec4<f32>(in.position.x/2.0, in.position.y/2.0 - 1.0, in.position.z, 1.0);

    // Perspective projection using the camera uniform binding

    // Currently I have no instance data for rei, she is just at the origin
    // with no rotation. Thus, lighting should work even without position/rotation
    // But i will have to implement that.
    out.world_position = in.position;
    out.world_normal = in.normal;
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
    // Ambient light
    let object_colour = textureSample(diffuse_texture, diffuse_sampler, in.tex_coords);
    let world_colour = vec3<f32>(0.5, 0.82, 0.98);
    let ambient_strength = 0.1;
    let world_ambient_strength = 0.5;

    let ambient_colour = light.colour * ambient_strength + world_colour * world_ambient_strength;

    // Diffuse light
    let light_dir = normalize(light.position - in.world_position);
    let diffuse_strength = max(dot(light_dir, in.world_normal), 0.0);
    let diffuse_colour = diffuse_strength * light.colour;

    // Specular light
    let view_dir = normalize(camera.position.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let specular_strength = pow(max(dot(view_dir, half_dir), 0.0), 10.0) * 0.4;
    let specular_colour = light.colour * specular_strength;

    let result = (ambient_colour + diffuse_colour + specular_colour) * object_colour.xyz;

    return vec4<f32>(result, object_colour.a);
}