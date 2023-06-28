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

struct InstanceInput {
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,

    @location(9) r0: vec3<f32>,
    @location(10) r1: vec3<f32>,
    @location(11) r2: vec3<f32>,
};

struct Camera {
    position: vec4<f32>,
    matrix: mat4x4<f32>,
};

struct Light {
    position: vec3<f32>,
    scale: f32,
    colour: vec3<f32>,
    brightness: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(2) @binding(0)
var<uniform> light: Light;

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let instance_matrix = mat4x4<f32>(
        instance.m0,
        instance.m1,
        instance.m2,
        instance.m3
    );

    let rotation_matrix = mat3x3<f32>(
        instance.r0,
        instance.r1,
        instance.r2
    );

    // Perspective projection using the camera uniform binding

    let position = instance_matrix * vec4<f32>(in.position, 1.0);
    out.world_position = position.xyz;
    out.world_normal = rotation_matrix * in.normal;
    out.clip_position = camera.matrix * position;
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

    var distance_scale: f32;
    let distance= distance(in.world_position, light.position);
    let cutoff = 0.1;

    if distance <= cutoff {
        distance_scale = light.brightness;
    } else {
        let dist_from_cutoff = (distance - cutoff + light.scale) / light.scale;
        distance_scale = light.brightness / (dist_from_cutoff*dist_from_cutoff);
    }

    let result = (ambient_colour + (diffuse_colour + specular_colour) * distance_scale) * object_colour.xyz;

    return vec4<f32>(result, object_colour.a);
}