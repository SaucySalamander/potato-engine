struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};

struct IndirectDraw {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
    model_index: u32,
    _padding: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> models: array<mat4x4<f32>>;

@group(2) @binding(0)
var<storage, read> draw_commands: array<IndirectDraw>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @builtin(instance_index) instance_idx: u32,
    // @builtin(draw_index) draw_idx: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let model_matrix = models[in.instance_idx];
    let world_pos = model_matrix * vec4(in.position, 1.0);
    let view_pos = camera.view * world_pos;
    let clip_pos = camera.proj * view_pos;

    var out: VertexOutput;
    out.position = clip_pos;
    out.color = vec3<f32>(f32(in.instance_idx) * 0.1, 0.75, 0.75);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}