struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
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
fn vs_main(in: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let instance_matrix = mat4x4<f32>(
        instance.m0,
        instance.m1,
        instance.m2,
        instance.m3
    );
    out.position = camera.matrix * instance_matrix * vec4<f32>(in.position, 1.0);
    //out.position = camera.matrix * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
