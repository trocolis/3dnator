// Matriz da câmera e opções de exibição:
// `options.x > 0.5` mostra a triangulação; `options.y > 0.5` mostra as arestas vivas.
struct View {
    transform: mat4x4<f32>,
    options: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: View;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    // Bit k: a aresta oposta ao canto k é viva (dobra acentuada ou borda aberta).
    @location(2) edges: u32,
}
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    // Coordenadas baricêntricas: o canto i do triângulo recebe o eixo i. A distância às
    // arestas permite desenhar linhas no próprio passe de superfície, sem buffers de linhas
    // (que custariam 144 bytes por triângulo na memória da GPU).
    @location(1) barycentric: vec3<f32>,
    @location(2) @interpolate(flat) edges: u32,
}

@vertex fn vertex_main(input: VertexInput, @builtin(vertex_index) index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = view.transform * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    // A malha é uma lista de triângulos sem índices: vertex_index % 3 é o canto.
    let corner = index % 3u;
    output.barycentric = vec3<f32>(
        select(0.0, 1.0, corner == 0u),
        select(0.0, 1.0, corner == 1u),
        select(0.0, 1.0, corner == 2u),
    );
    output.edges = input.edges;
    return output;
}

@fragment fn surface_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.4, -0.6, 1.0));
    let intensity = 0.38 + 0.62 * abs(dot(normalize(input.normal), light));
    let surface = vec3<f32>(0.68, 0.72, 0.76) * intensity;
    // Derivadas calculadas fora de desvios, como exige a uniformidade do WGSL.
    let width = fwidth(input.barycentric);
    // Proximidade de cada aresta: 1 sobre a aresta, 0 a ~1,2 px dela.
    let near = vec3<f32>(1.0) - smoothstep(vec3<f32>(0.0), width * 1.2, input.barycentric);
    let flags = vec3<f32>(
        f32(input.edges & 1u),
        f32((input.edges >> 1u) & 1u),
        f32((input.edges >> 2u) & 1u),
    );
    let triangulation = max(max(near.x, near.y), near.z) * view.options.x;
    let crease_near = near * flags;
    let crease = max(max(crease_near.x, crease_near.y), crease_near.z) * view.options.y;
    var color = mix(surface, vec3<f32>(0.11, 0.16, 0.20), triangulation);
    color = mix(color, vec3<f32>(0.05, 0.06, 0.07), crease);
    return vec4<f32>(color, 1.0);
}
