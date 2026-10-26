struct Camera { transform: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.transform * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    return output;
}

@fragment fn surface_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.4, -0.6, 1.0));
    let intensity = 0.38 + 0.62 * abs(dot(normalize(input.normal), light));
    return vec4<f32>(vec3<f32>(0.68, 0.72, 0.76) * intensity, 1.0);
}

@fragment fn line_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.11, 0.16, 0.20, 1.0);
}
