//! Visualização ortográfica e em perspectiva. Nenhuma operação CAD é realizada pela GPU.

use nator_geometry::{Bounds, Mesh, Point3, RgbColor, cross, dot, norm, sub};
use wgpu::util::DeviceExt;

pub use wgpu;
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Bytes por vértice enviado à GPU: posição e normal em f32, máscara de arestas vivas e cor RGBA8.
const VERTEX_BYTES: usize = 32;

/// Preparação CPU pode ocorrer na thread de trabalho; a interface só faz o upload.
/// Cada triângulo ocupa 96 bytes (três vértices). Triangulação e arestas vivas são
/// desenhadas no shader, sem buffers de linhas.
pub struct PreparedMesh {
    vertices: Vec<u8>,
    /// Vértices opacos no início do buffer; os demais são translúcidos.
    opaque_vertices: usize,
    /// Vértices do plano de sombra suave sob a base da cena.
    ground_shadow: Vec<u8>,
}

/// Estilos de exibição gravados nos bits 8 a 15 do atributo de arestas (ver `view.wgsl`).
pub const STYLE_NORMAL: u8 = 0;
/// Esmaecido: cor puxada para o fundo e quase transparente.
pub const STYLE_DIMMED: u8 = 1;
pub const STYLE_TRANSPARENT: u8 = 2;
impl PreparedMesh {
    /// Analisa as arestas vivas da malha. Para cenas do documento, prefira
    /// `with_creases`, que reaproveita a análise feita uma vez por geometria.
    pub fn new(mesh: &Mesh) -> Result<Self, String> {
        let (_, creases) = mesh.edge_analysis(Some(std::f64::consts::FRAC_PI_6));
        Self::with_creases(mesh, &creases)
    }

    /// `creases[i]` marca as arestas vivas do triângulo i (bit k: aresta oposta ao canto k).
    pub fn with_creases(mesh: &Mesh, creases: &[u8]) -> Result<Self, String> {
        Self::with_styles(mesh, creases, &[], mesh.triangles().len())
    }

    /// Como `with_creases`, com um estilo por triângulo (`STYLE_*`; vazio = todos normais).
    /// Os primeiros `opaque_triangles` são desenhados opacos; os demais, translúcidos e
    /// depois deles, como exige a mistura de cores.
    pub fn with_styles(
        mesh: &Mesh,
        creases: &[u8],
        styles: &[u8],
        opaque_triangles: usize,
    ) -> Result<Self, String> {
        Self::with_appearance(mesh, creases, styles, opaque_triangles, &[])
    }

    /// Cores sólidas por triângulo; a quantização de exibição usa sRGB de oito bits por
    /// canal e acrescenta quatro bytes ao vértice, sem duplicar geometria no documento.
    pub fn with_appearance(
        mesh: &Mesh,
        creases: &[u8],
        styles: &[u8],
        opaque_triangles: usize,
        colors: &[RgbColor],
    ) -> Result<Self, String> {
        let count = mesh.triangles().len();
        if !colors.is_empty() && colors.len() != count {
            return Err("Cores não correspondem aos triângulos da cena.".into());
        }
        if creases.len() != count {
            return Err("Arestas vivas não correspondem aos triângulos da cena.".into());
        }
        if (!styles.is_empty() && styles.len() != count) || opaque_triangles > count {
            return Err("Estilos não correspondem aos triângulos da cena.".into());
        }
        let center = mesh.bounds().center();
        let scale = mesh.bounds().size().into_iter().fold(0_f64, f64::max);
        if scale <= 0. {
            return Err("A malha não possui extensão visível.".into());
        }
        let mut vertices = Vec::with_capacity(mesh.triangles().len() * 3 * VERTEX_BYTES);
        for (index, (triangle, mask)) in mesh.triangles().iter().zip(creases).enumerate() {
            let style = styles.get(index).copied().unwrap_or(STYLE_NORMAL);
            let attribute = u32::from(*mask) | (u32::from(style) << 8);
            let points = mesh.triangle_points(*triangle);
            let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
            let length = norm(normal);
            for point in points {
                // Coordenadas normalizadas pela caixa da cena: f32 mantém a precisão visual
                // mesmo para modelos longe da origem.
                for v in sub(point, center) {
                    vertices.extend_from_slice(&((v / scale) as f32).to_le_bytes());
                }
                for v in normal {
                    vertices.extend_from_slice(&((v / length) as f32).to_le_bytes());
                }
                vertices.extend_from_slice(&attribute.to_le_bytes());
                vertices.extend_from_slice(
                    &colors
                        .get(index)
                        .copied()
                        .unwrap_or(RgbColor::NEUTRAL)
                        .channels(),
                );
                vertices.push(255);
            }
        }
        let min_z = ((mesh.bounds().min[2] - center[2]) / scale) as f32 - 0.002;
        let half_x = ((mesh.bounds().size()[0] * 0.5) / scale) as f32;
        let half_y = ((mesh.bounds().size()[1] * 0.5) / scale) as f32;
        let rx = (half_x * 1.5).max(0.1);
        let ry = (half_y * 1.5).max(0.1);

        // Quad de sombra suave de contato: 2 triângulos (6 vértices) no plano da base.
        // A normal carrega as coordenadas paramétricas (-1 a 1) para cálculo do decaimento radial no shader.
        let mut ground_shadow = Vec::with_capacity(6 * VERTEX_BYTES);
        let shadow_quad = [
            ([-rx, -ry], [-1.0_f32, -1.0_f32]),
            ([rx, -ry], [1.0_f32, -1.0_f32]),
            ([rx, ry], [1.0_f32, 1.0_f32]),
            ([-rx, -ry], [-1.0_f32, -1.0_f32]),
            ([rx, ry], [1.0_f32, 1.0_f32]),
            ([-rx, ry], [-1.0_f32, 1.0_f32]),
        ];
        for (pos, local) in shadow_quad {
            ground_shadow.extend_from_slice(&(pos[0]).to_le_bytes());
            ground_shadow.extend_from_slice(&(pos[1]).to_le_bytes());
            ground_shadow.extend_from_slice(&(min_z).to_le_bytes());
            ground_shadow.extend_from_slice(&(local[0]).to_le_bytes());
            ground_shadow.extend_from_slice(&(local[1]).to_le_bytes());
            ground_shadow.extend_from_slice(&(0.0_f32).to_le_bytes());
            ground_shadow.extend_from_slice(&0_u32.to_le_bytes());
            ground_shadow.extend_from_slice(&[0, 0, 0, 255]);
        }
        Ok(Self {
            vertices,
            opaque_vertices: opaque_triangles * 3,
            ground_shadow,
        })
    }
    /// Bytes que serão enviados à GPU.
    pub fn gpu_bytes(&self) -> usize {
        self.vertices.len() + self.ground_shadow.len()
    }
}

/// A projeção pertence à câmera da aba, sem alterar o documento ou seu histórico.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Projection {
    #[default]
    Orthographic,
    Perspective,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub span: f64,
    pub projection: Projection,
    target: Point3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.9,
            pitch: 0.6,
            span: 1.5,
            projection: Projection::Orthographic,
            target: [0.; 3],
        }
    }
}

impl Camera {
    /// Mantém o alvo e a escala em coordenadas de mundo quando a cena muda de extensão.
    pub fn reframe(&mut self, previous: Bounds, next: Bounds) {
        let old_scale = previous.size().into_iter().fold(0_f64, f64::max);
        let new_scale = next.size().into_iter().fold(0_f64, f64::max);
        let ratio = old_scale / new_scale;
        if !ratio.is_finite() || ratio <= 0. {
            return;
        }
        let old_center = previous.center();
        let new_center = next.center();
        self.target = std::array::from_fn(|i| {
            (old_center[i] + self.target[i] * old_scale - new_center[i]) / new_scale
        });
        self.span *= ratio;
    }
    pub fn fit(&mut self, aspect: f64) {
        self.target = [0.; 3];
        // A perspectiva precisa de margem para os cantos mais próximos do observador.
        let margin = match self.projection {
            Projection::Orthographic => 1.9,
            Projection::Perspective => 2.1,
        };
        self.span = margin * (1. / aspect.max(0.01)).max(1.);
    }

    /// Campo vertical fixo de 45°; `span` mede a altura no plano do alvo em ambas as
    /// projeções. Alternar a projeção conserva o enquadramento nesse plano. O zoom em
    /// perspectiva desloca o observador, mantendo as mesmas regras de pan e zoom no cursor.
    fn eye_distance(&self) -> f64 {
        self.span * 0.5 / (std::f64::consts::FRAC_PI_4 * 0.5).tan()
    }

    fn depth_range(&self) -> (f64, f64) {
        let distance = self.eye_distance();
        // A cena normalizada cabe numa esfera de raio √3/2. Incluir o deslocamento
        // do alvo no plano distante evita recortar a cena ao deslocar a câmera.
        (distance * 0.001, distance + norm(self.target) + 4.)
    }

    fn basis(&self) -> (Point3, Point3, Point3) {
        let direction = [
            self.yaw.cos() * self.pitch.cos(),
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
        ];
        let right = [-self.yaw.sin(), self.yaw.cos(), 0.];
        (right, cross(direction, right), direction)
    }

    pub fn orbit(&mut self, delta: [f64; 2]) {
        self.yaw -= delta[0] * 0.008;
        self.pitch = (self.pitch + delta[1] * 0.008).clamp(-1.55, 1.55);
    }

    pub fn pan(&mut self, delta: [f64; 2], height: f64) {
        if height <= 0. {
            return;
        }
        let (right, up, _) = self.basis();
        for i in 0..3 {
            self.target[i] += (-right[i] * delta[0] + up[i] * delta[1]) * self.span / height;
        }
    }

    pub fn zoom_at(&mut self, scroll: f64, cursor: [f64; 2], aspect: f64) {
        let previous = self.span;
        self.span = (self.span * (-scroll * 0.002).exp()).clamp(0.02, 100.);
        let (right, up, _) = self.basis();
        for i in 0..3 {
            self.target[i] +=
                (right[i] * cursor[0] * aspect + up[i] * cursor[1]) * (previous - self.span) * 0.5;
        }
    }

    pub fn direction(&self) -> Point3 {
        self.basis().2
    }
    pub fn set_direction(&mut self, direction: Point3) {
        let length = norm(direction);
        if length > 1e-9 {
            self.yaw = direction[1].atan2(direction[0]);
            self.pitch = (direction[2] / length).clamp(-1., 1.).asin();
        }
    }
    /// Coordenadas normalizadas da tela (Y para cima); pontos recortados em profundidade
    /// não geram anotações. A divisão por W deve coincidir com a matriz enviada à GPU.
    pub fn project_world(&self, point: Point3, bounds: Bounds, aspect: f64) -> Option<[f64; 2]> {
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let p = std::array::from_fn(|i| (point[i] - bounds.center()[i]) / scale - self.target[i]);
        let (right, up, direction) = self.basis();
        let factor = match self.projection {
            Projection::Orthographic => {
                if dot(p, direction).abs() > 4. {
                    return None;
                }
                1.
            }
            Projection::Perspective => {
                let depth = self.eye_distance() - dot(p, direction);
                let (near, far) = self.depth_range();
                if depth < near || depth > far {
                    return None;
                }
                self.eye_distance() / depth
            }
        };
        let screen = [
            dot(p, right) * 2. * factor / (self.span * aspect),
            dot(p, up) * 2. * factor / self.span,
        ];
        screen.iter().all(|v| v.is_finite()).then_some(screen)
    }

    /// Interseção com o plano do alvo, em coordenadas de montagem (mm). Movimentação
    /// livre usa esse plano, pois raios em perspectiva partem do mesmo observador.
    pub fn world_on_target_plane(&self, cursor: [f64; 2], bounds: Bounds, aspect: f64) -> Point3 {
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let (right, up, _) = self.basis();
        std::array::from_fn(|i| {
            bounds.center()[i]
                + scale
                    * (self.target[i]
                        + right[i] * cursor[0] * self.span * aspect * 0.5
                        + up[i] * cursor[1] * self.span * 0.5)
        })
    }
    pub fn world_ray(
        &self,
        cursor: [f64; 2],
        bounds: Bounds,
        aspect: f64,
    ) -> nator_geometry::query::Ray {
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let (right, up, direction) = self.basis();
        match self.projection {
            Projection::Orthographic => {
                let plane = self.world_on_target_plane(cursor, bounds, aspect);
                nator_geometry::query::Ray {
                    origin: std::array::from_fn(|i| plane[i] + scale * direction[i] * 4.),
                    direction: direction.map(|v| -v),
                }
            }
            Projection::Perspective => {
                let distance = self.eye_distance();
                let (near, _) = self.depth_range();
                let toward: Point3 = std::array::from_fn(|i| {
                    right[i] * cursor[0] * self.span * aspect * 0.5
                        + up[i] * cursor[1] * self.span * 0.5
                        - direction[i] * distance
                });
                let length = norm(toward);
                // Origem no plano próximo, para não selecionar superfícies que a GPU
                // recortou. A direção unitária mantém as distâncias dos testes de raio em mm.
                nator_geometry::query::Ray {
                    origin: std::array::from_fn(|i| {
                        bounds.center()[i]
                            + scale
                                * (self.target[i]
                                    + direction[i] * distance
                                    + toward[i] * near / distance)
                    }),
                    direction: toward.map(|v| v / length),
                }
            }
        }
    }
    pub fn axis_projection(&self, axis: Point3) -> [f64; 2] {
        let (right, up, _) = self.basis();
        [dot(right, axis), -dot(up, axis)]
    }

    fn matrix(&self, aspect: f64) -> [f32; 16] {
        let (right, up, direction) = self.basis();
        if self.projection == Projection::Perspective {
            let distance = self.eye_distance();
            let (near, far) = self.depth_range();
            let focal = distance * 2. / self.span;
            // W = distância ao observador ao longo da direção de vista. O intervalo
            // de profundidade do wgpu é [0, 1], diferente do OpenGL clássico [-1, 1].
            let a = far / (far - near);
            let b = a * near;
            let mut matrix = [0_f32; 16];
            for i in 0..3 {
                matrix[i * 4] = (right[i] * focal / aspect) as f32;
                matrix[i * 4 + 1] = (up[i] * focal) as f32;
                matrix[i * 4 + 2] = (-direction[i] * a) as f32;
                matrix[i * 4 + 3] = -direction[i] as f32;
            }
            let offset = distance + dot(direction, self.target);
            matrix[12] = (-dot(right, self.target) * focal / aspect) as f32;
            matrix[13] = (-dot(up, self.target) * focal) as f32;
            matrix[14] = (a * offset - b) as f32;
            matrix[15] = offset as f32;
            return matrix;
        }
        let sx = 2. / (self.span * aspect);
        let sy = 2. / self.span;
        let mut matrix = [0_f32; 16];
        for i in 0..3 {
            matrix[i * 4] = (right[i] * sx) as f32;
            matrix[i * 4 + 1] = (up[i] * sy) as f32;
            matrix[i * 4 + 2] = (-direction[i] / 8.) as f32;
        }
        matrix[12] = (-dot(right, self.target) * sx) as f32;
        matrix[13] = (-dot(up, self.target) * sy) as f32;
        matrix[14] = (0.5 + dot(direction, self.target) / 8.) as f32;
        matrix[15] = 1.;
        matrix
    }
}

pub struct Viewport {
    pub camera: Camera,
    pub show_triangles: bool,
    /// Arestas vivas (dobras, bordas abertas); ligadas por padrão.
    pub show_edges: bool,
    /// Sombras opcionais (iluminação direcional com faces opostas sombreadas e sombra de apoio); desligadas por padrão.
    pub show_shadows: bool,
    /// Fundo de apresentação; não participa da cena nem da geometria.
    background: [f32; 4],
    color: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
    size: [u32; 2],
    triangles: wgpu::Buffer,
    vertex_count: u32,
    opaque_count: u32,
    shadow_triangles: wgpu::Buffer,
    shadow_count: u32,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    surface_pipeline: wgpu::RenderPipeline,
    /// Corpos translúcidos: mistura de cores e sem escrita de profundidade.
    translucent_pipeline: wgpu::RenderPipeline,
    /// Sombra de apoio suave sob o modelo.
    shadow_pipeline: wgpu::RenderPipeline,
}

/// Uniforme do shader: matriz 4×4, quatro opções e cor do fundo (alinhamento de 16 bytes).
const UNIFORM_BYTES: u64 = 96;

fn float_bytes(values: impl IntoIterator<Item = f32>) -> Vec<u8> {
    values.into_iter().flat_map(f32::to_le_bytes).collect()
}

fn targets(
    device: &wgpu::Device,
    size: [u32; 2],
) -> (wgpu::Texture, wgpu::TextureView, wgpu::TextureView) {
    let descriptor = wgpu::TextureDescriptor {
        label: Some("Área de edição"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    let color = device.create_texture(&descriptor);
    let view = color.create_view(&Default::default());
    let depth = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Profundidade"),
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..descriptor
        })
        .create_view(&Default::default());
    (color, view, depth)
}

impl Viewport {
    pub fn new(device: &wgpu::Device, mesh: &Mesh) -> Result<Self, String> {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Câmera"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Câmera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Câmera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Visualização"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Superfície neutra"),
            source: wgpu::ShaderSource::Wgsl(include_str!("view.wgsl").into()),
        });
        let pipeline = |translucent: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(if translucent { "Superfícies translúcidas" } else { "Superfícies" }),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_BYTES as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Uint32,3=>Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(!translucent),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("surface_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: translucent.then_some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
        };
        let surface_pipeline = pipeline(false);
        let translucent_pipeline = pipeline(true);
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sombra de contato"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("shadow_vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_BYTES as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Uint32,3=>Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("shadow_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let (color, view, depth) = targets(device, [1, 1]);
        let empty = || {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Malha vazia"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        };
        let mut viewport = Self {
            camera: Camera::default(),
            show_triangles: false,
            show_edges: true,
            show_shadows: false,
            background: [0.075, 0.09, 0.11, 1.],
            color,
            view,
            depth,
            size: [1, 1],
            triangles: empty(),
            vertex_count: 0,
            opaque_count: 0,
            shadow_triangles: empty(),
            shadow_count: 0,
            uniform,
            bind_group,
            surface_pipeline,
            translucent_pipeline,
            shadow_pipeline,
        };
        viewport.set_mesh(device, mesh)?;
        Ok(viewport)
    }

    /// Cor sRGB do fundo; a interface pode escolher seu tema sem alterar os corpos.
    pub fn set_background(&mut self, color: RgbColor) {
        let [r, g, b] = color.channels().map(|v| f32::from(v) / 255.);
        self.background = [r, g, b, 1.];
    }

    pub fn set_mesh(&mut self, device: &wgpu::Device, mesh: &Mesh) -> Result<(), String> {
        self.set_prepared_mesh(device, &PreparedMesh::new(mesh)?)
    }

    pub fn clear(&mut self, device: &wgpu::Device) {
        self.triangles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cena vazia"),
            size: 24,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        self.shadow_triangles = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sombra vazia"),
            size: 24,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        self.vertex_count = 0;
        self.opaque_count = 0;
        self.shadow_count = 0;
    }

    pub fn set_prepared_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &PreparedMesh,
    ) -> Result<(), String> {
        if mesh.vertices.len() as u64 > device.limits().max_buffer_size {
            return Err("Malha grande demais para os buffers desta GPU.".into());
        }
        self.triangles = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Superfícies"),
            contents: &mesh.vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.shadow_triangles = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sombra de contato"),
            contents: &mesh.ground_shadow,
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vertex_count = (mesh.vertices.len() / VERTEX_BYTES) as u32;
        self.opaque_count = (mesh.opaque_vertices as u32).min(self.vertex_count);
        self.shadow_count = (mesh.ground_shadow.len() / VERTEX_BYTES) as u32;
        self.camera = Camera::default();
        self.camera
            .fit(f64::from(self.size[0]) / f64::from(self.size[1]));
        Ok(())
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.color
    }
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: [u32; 2]) -> bool {
        let limit = device.limits().max_texture_dimension_2d.min(4096);
        // Reduzir ambos os eixos pelo mesmo fator evita distorção em telas de alta resolução.
        let size = size.map(|n| n.max(1));
        let factor = (f64::from(limit) / f64::from(size[0].max(size[1]))).min(1.);
        let size = size.map(|n| (f64::from(n) * factor).round().max(1.) as u32);
        if size == self.size {
            return false;
        }
        if self.size == [1, 1] {
            self.camera.fit(f64::from(size[0]) / f64::from(size[1]));
        }
        (self.color, self.view, self.depth) = targets(device, size);
        self.size = size;
        true
    }

    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let matrix = self
            .camera
            .matrix(f64::from(self.size[0]) / f64::from(self.size[1]));
        let flag = |on: bool| if on { 1. } else { 0. };
        let options = [
            flag(self.show_triangles),
            flag(self.show_edges),
            flag(self.show_shadows),
            0.,
        ];
        queue.write_buffer(
            &self.uniform,
            0,
            &float_bytes(matrix.into_iter().chain(options).chain(self.background)),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Área 3D"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.background[0]),
                            g: f64::from(self.background[1]),
                            b: f64::from(self.background[2]),
                            a: f64::from(self.background[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            if self.show_shadows && self.shadow_count > 0 {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_vertex_buffer(0, self.shadow_triangles.slice(..));
                pass.draw(0..self.shadow_count, 0..1);
            }
            pass.set_pipeline(&self.surface_pipeline);
            pass.set_vertex_buffer(0, self.triangles.slice(..));
            pass.draw(0..self.opaque_count, 0..1);
            if self.opaque_count < self.vertex_count {
                pass.set_pipeline(&self.translucent_pipeline);
                pass.draw(self.opaque_count..self.vertex_count, 0..1);
            }
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECTIONS: [Projection; 2] = [Projection::Orthographic, Projection::Perspective];
    const BOUNDS: Bounds = Bounds {
        min: [-5.; 3],
        max: [5.; 3],
    };

    /// Reproduz o produto coluna-major e a divisão homogênea feitos pelo pipeline.
    fn gpu_projection(camera: &Camera, bounds: Bounds, point: Point3, aspect: f64) -> Point3 {
        let matrix = camera.matrix(aspect);
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let p = sub(point, bounds.center()).map(|v| (v / scale) as f32);
        let clip: [f32; 4] = std::array::from_fn(|i| {
            matrix[i] * p[0] + matrix[4 + i] * p[1] + matrix[8 + i] * p[2] + matrix[12 + i]
        });
        std::array::from_fn(|i| f64::from(clip[i] / clip[3]))
    }

    #[test]
    fn changing_scene_bounds_preserves_world_projection() {
        let previous = Bounds {
            min: [0.; 3],
            max: [10.; 3],
        };
        let next = Bounds {
            min: [0.; 3],
            max: [20.; 3],
        };
        let point = [2., 3., 4.];
        for projection in PROJECTIONS {
            let mut camera = Camera {
                projection,
                ..Camera::default()
            };
            camera.pan([70., -30.], 600.);
            let before = gpu_projection(&camera, previous, point, 1.);
            camera.reframe(previous, next);
            let after = gpu_projection(&camera, next, point, 1.);
            for i in 0..2 {
                assert!((before[i] - after[i]).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn zoom_preserves_point_beneath_cursor() {
        let cursor = [0.4, -0.3];
        let aspect = 1.6;
        for projection in PROJECTIONS {
            let mut camera = Camera {
                projection,
                ..Camera::default()
            };
            let point = camera.world_on_target_plane(cursor, BOUNDS, aspect);
            camera.zoom_at(120., cursor, aspect);
            let projected = gpu_projection(&camera, BOUNDS, point, aspect);
            assert!((projected[0] - cursor[0]).abs() < 1e-6);
            assert!((projected[1] - cursor[1]).abs() < 1e-6);
        }
    }

    #[test]
    fn picking_rays_and_overlays_match_the_gpu() -> Result<(), String> {
        // A caixa longe da origem exercita também a normalização mundo/cena.
        let bounds = Bounds {
            min: [120., -80., 30.],
            max: [180., -60., 60.],
        };
        for projection in PROJECTIONS {
            let mut camera = Camera {
                projection,
                ..Camera::default()
            };
            camera.pan([80., -25.], 600.);
            camera.orbit([20., -10.]);
            for aspect in [0.6, 1., 1.8] {
                for cursor in [[0.; 2], [-0.7, 0.3], [0.4, -0.8]] {
                    let ray = camera.world_ray(cursor, bounds, aspect);
                    assert!((norm(ray.direction) - 1.).abs() < 1e-12);
                    for distance in [5., 35., 100.] {
                        let point =
                            std::array::from_fn(|i| ray.origin[i] + ray.direction[i] * distance);
                        let gpu = gpu_projection(&camera, bounds, point, aspect);
                        let overlay = camera
                            .project_world(point, bounds, aspect)
                            .ok_or("Ponto do raio foi recortado")?;
                        for i in 0..2 {
                            assert!((gpu[i] - cursor[i]).abs() < 2e-5, "{projection:?}: {gpu:?}");
                            assert!((overlay[i] - cursor[i]).abs() < 1e-12);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn perspective_makes_nearby_geometry_larger() -> Result<(), String> {
        let camera = Camera {
            projection: Projection::Perspective,
            ..Camera::default()
        };
        let (right, _, direction) = camera.basis();
        let near = std::array::from_fn(|i| 10. * (right[i] * 0.2 + direction[i] * 0.4));
        let far = std::array::from_fn(|i| 10. * (right[i] * 0.2 - direction[i] * 0.4));
        let a = camera
            .project_world(near, BOUNDS, 1.)
            .ok_or("Ponto próximo recortado")?;
        let b = camera
            .project_world(far, BOUNDS, 1.)
            .ok_or("Ponto distante recortado")?;
        assert!(a[0] > b[0] * 1.5);
        Ok(())
    }

    #[test]
    fn perspective_clipping_uses_wgpu_depth_and_hides_annotations() {
        let camera = Camera {
            projection: Projection::Perspective,
            ..Camera::default()
        };
        let direction = camera.direction();
        let point_at_depth =
            |depth: f64| direction.map(|v| 10. * v * (camera.eye_distance() - depth));
        let (near, far) = camera.depth_range();
        let a = gpu_projection(&camera, BOUNDS, point_at_depth(near), 1.);
        let b = gpu_projection(&camera, BOUNDS, point_at_depth(far), 1.);
        assert!(a[2].abs() < 2e-4);
        assert!((b[2] - 1.).abs() < 1e-6);
        for depth in [-1., 0., near * 0.5, far + 1.] {
            assert!(
                camera
                    .project_world(point_at_depth(depth), BOUNDS, 1.)
                    .is_none()
            );
        }
    }

    #[test]
    fn fit_contains_the_scene_in_both_projections() {
        for projection in PROJECTIONS {
            for aspect in [0.4, 1., 2.5] {
                for direction in [[1., 1., 1.], [1., 0., 0.], [0., 0., 1.]] {
                    let mut camera = Camera {
                        projection,
                        ..Camera::default()
                    };
                    camera.set_direction(direction);
                    camera.fit(aspect);
                    for x in [-5., 5.] {
                        for y in [-5., 5.] {
                            for z in [-5., 5.] {
                                let p = gpu_projection(&camera, BOUNDS, [x, y, z], aspect);
                                assert!(
                                    p[0].abs() < 1.
                                        && p[1].abs() < 1.
                                        && (0. ..=1.).contains(&p[2]),
                                    "{projection:?}: {p:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn projection_switch_preserves_target_plane_and_drag_distance() {
        let mut camera = Camera::default();
        let cursor = [0.4, -0.3];
        let point = camera.world_on_target_plane(cursor, BOUNDS, 1.6);
        let before = gpu_projection(&camera, BOUNDS, point, 1.6);
        camera.projection = Projection::Perspective;
        let after = gpu_projection(&camera, BOUNDS, point, 1.6);
        for i in 0..2 {
            assert!((before[i] - after[i]).abs() < 1e-6);
        }
        let center = camera.world_on_target_plane([0.; 2], BOUNDS, 1.6);
        let dragged = camera.world_on_target_plane(cursor, BOUNDS, 1.6);
        // O movimento permanece não nulo: usar a origem dos raios daria praticamente
        // zero em perspectiva, pois ela está muito próxima do observador comum.
        assert!(
            (norm(sub(dragged, center))
                - 10. * camera.span * 0.5 * (0.64_f64.powi(2) + 0.3_f64.powi(2)).sqrt())
            .abs()
                < 1e-12
        );
        camera.pan([40., -30.], 600.);
        assert!(
            norm(sub(
                camera.world_on_target_plane([0.; 2], BOUNDS, 1.6),
                center
            )) > 0.
        );
    }

    #[test]
    fn optional_shadows_generate_ground_contact_buffer() -> Result<(), String> {
        let mesh = nator_geometry::reference_bracket().map_err(|e| e.to_string())?;
        let prepared = PreparedMesh::new(&mesh)?;
        // 6 vértices para os 2 triângulos do quad de apoio
        assert_eq!(prepared.ground_shadow.len(), 6 * VERTEX_BYTES);
        assert_eq!(
            prepared.gpu_bytes(),
            prepared.vertices.len() + 6 * VERTEX_BYTES
        );
        Ok(())
    }
}
