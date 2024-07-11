struct Camera {
    view_projection: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(1) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_projection * vec4<f32>(
        (vertex.position + vec3(instance.position, 0.0)),
        1.0,
    );
    return out;
}

@fragment
fn fs_main(out: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(
        smoothstep(0.4, 1.0, sin(out.position.x * 0.6) + 1.0),
        smoothstep(0.4, 1.0, cos(out.position.y * 0.3) + 1.0),
        1.0,
        1.0,
    );
}
