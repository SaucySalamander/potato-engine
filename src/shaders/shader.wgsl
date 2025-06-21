struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct Model {
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> model: Model;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world_pos = model.model * vec4(in.position, 1.0);
    let view_pos = camera.view * world_pos;
    let clip_pos = camera.proj * view_pos;

    var out: VertexOutput;
    out.position = clip_pos;
    out.color = vec3<f32>(0.75, 0.75, 0.75);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}