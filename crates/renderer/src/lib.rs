//! Visualização ortográfica leve. Nenhuma operação CAD é realizada pela GPU.

use nator_geometry::{Bounds, Mesh, Point3, cross, dot, norm, sub};
use wgpu::util::DeviceExt;

pub use wgpu;
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Preparação CPU pode ocorrer na thread de trabalho; a interface só faz o upload.
pub struct PreparedMesh {
    vertices: Vec<u8>,
    lines: Vec<u8>,
}
impl PreparedMesh {
    pub fn new(mesh: &Mesh) -> Result<Self, String> {
        let triangle_bytes = mesh.triangles().len() * 3 * 24;
        let center = mesh.bounds().center();
        let scale = mesh.bounds().size().into_iter().fold(0_f64, f64::max);
        if scale <= 0. {
            return Err("A malha não possui extensão visível.".into());
        }
        let mut vertices = Vec::with_capacity(triangle_bytes);
        let mut lines = Vec::with_capacity(triangle_bytes * 2);
        for triangle in mesh.triangles() {
            let points = mesh.triangle_points(*triangle);
            let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
            let normal = normal.map(|v| (v / norm(normal)) as f32);
            let records = points.map(|p| {
                float_bytes(
                    sub(p, center)
                        .map(|v| (v / scale) as f32)
                        .into_iter()
                        .chain(normal),
                )
            });
            for record in &records {
                vertices.extend_from_slice(record);
            }
            for index in [0, 1, 1, 2, 2, 0] {
                lines.extend_from_slice(&records[index]);
            }
        }
        Ok(Self { vertices, lines })
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub span: f64,
    target: Point3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.9,
            pitch: 0.6,
            span: 1.5,
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
        self.span = 1.9 * (1. / aspect.max(0.01)).max(1.);
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
    pub fn project_world(&self, point: Point3, bounds: Bounds, aspect: f64) -> [f64; 2] {
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let p = std::array::from_fn(|i| (point[i] - bounds.center()[i]) / scale - self.target[i]);
        let (right, up, _) = self.basis();
        [
            dot(p, right) * 2. / (self.span * aspect),
            dot(p, up) * 2. / self.span,
        ]
    }
    pub fn world_ray(
        &self,
        cursor: [f64; 2],
        bounds: Bounds,
        aspect: f64,
    ) -> nator_geometry::query::Ray {
        let scale = bounds.size().into_iter().fold(0_f64, f64::max);
        let (right, up, direction) = self.basis();
        let origin = std::array::from_fn(|i| {
            bounds.center()[i]
                + scale
                    * (self.target[i]
                        + right[i] * cursor[0] * self.span * aspect * 0.5
                        + up[i] * cursor[1] * self.span * 0.5
                        + direction[i] * 4.)
        });
        nator_geometry::query::Ray {
            origin,
            direction: direction.map(|v| -v),
        }
    }
    pub fn axis_projection(&self, axis: Point3) -> [f64; 2] {
        let (right, up, _) = self.basis();
        [dot(right, axis), -dot(up, axis)]
    }

    fn matrix(&self, aspect: f64) -> [f32; 16] {
        let (right, up, direction) = self.basis();
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
    color: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
    size: [u32; 2],
    triangles: wgpu::Buffer,
    lines: wgpu::Buffer,
    vertex_count: u32,
    line_count: u32,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    surface_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
}

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
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Câmera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
        let pipeline = |lines| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if lines {
                    "Triangulação"
                } else {
                    "Superfícies"
                }),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology: if lines {
                        wgpu::PrimitiveTopology::LineList
                    } else {
                        wgpu::PrimitiveTopology::TriangleList
                    },
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(!lines),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        // O bias só é suportado em triângulos; afasta a superfície das linhas.
                        constant: if lines { 0 } else { 1 },
                        ..Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if lines { "line_main" } else { "surface_main" }),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
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
            color,
            view,
            depth,
            size: [1, 1],
            triangles: empty(),
            lines: empty(),
            vertex_count: 0,
            line_count: 0,
            uniform,
            bind_group,
            surface_pipeline: pipeline(false),
            line_pipeline: pipeline(true),
        };
        viewport.set_mesh(device, mesh)?;
        Ok(viewport)
    }

    pub fn set_mesh(&mut self, device: &wgpu::Device, mesh: &Mesh) -> Result<(), String> {
        self.set_prepared_mesh(device, &PreparedMesh::new(mesh)?)
    }

    pub fn clear(&mut self, device: &wgpu::Device) {
        let empty = || {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Cena vazia"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        };
        self.triangles = empty();
        self.lines = empty();
        self.vertex_count = 0;
        self.line_count = 0;
    }

    pub fn set_prepared_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &PreparedMesh,
    ) -> Result<(), String> {
        if mesh.lines.len() as u64 > device.limits().max_buffer_size {
            return Err("Malha grande demais para os buffers desta GPU.".into());
        }
        // Substituir os buffers somente após preparar ambos os conjuntos de dados.
        self.triangles = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Superfícies"),
            contents: &mesh.vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.lines = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangulação"),
            contents: &mesh.lines,
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vertex_count = (mesh.vertices.len() / 24) as u32;
        self.line_count = (mesh.lines.len() / 24) as u32;
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
        queue.write_buffer(
            &self.uniform,
            0,
            &float_bytes(
                self.camera
                    .matrix(f64::from(self.size[0]) / f64::from(self.size[1])),
            ),
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
                            r: 0.075,
                            g: 0.09,
                            b: 0.11,
                            a: 1.,
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
            pass.set_pipeline(&self.surface_pipeline);
            pass.set_vertex_buffer(0, self.triangles.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
            if self.show_triangles {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_vertex_buffer(0, self.lines.slice(..));
                pass.draw(0..self.line_count, 0..1);
            }
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut camera = Camera::default();
        let before = camera.matrix(1.);
        camera.reframe(previous, next);
        let after = camera.matrix(1.);
        let point = [2., 3., 4.];
        let project = |matrix: [f32; 16], bounds: Bounds| {
            let scale = bounds.size().into_iter().fold(0_f64, f64::max);
            let p = sub(point, bounds.center()).map(|v| (v / scale) as f32);
            [
                matrix[0] * p[0] + matrix[4] * p[1] + matrix[8] * p[2] + matrix[12],
                matrix[1] * p[0] + matrix[5] * p[1] + matrix[9] * p[2] + matrix[13],
            ]
        };
        for (a, b) in project(before, previous)
            .into_iter()
            .zip(project(after, next))
        {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn zoom_preserves_point_beneath_cursor() {
        let mut camera = Camera::default();
        let cursor = [0.4, -0.3];
        let aspect = 1.6;
        let (right, up, _) = camera.basis();
        let point: Point3 = std::array::from_fn(|i| {
            camera.target[i]
                + (right[i] * cursor[0] * aspect + up[i] * cursor[1]) * camera.span * 0.5
        });
        camera.zoom_at(120., cursor, aspect);
        let matrix = camera.matrix(aspect);
        let projected_x = matrix[0] * point[0] as f32
            + matrix[4] * point[1] as f32
            + matrix[8] * point[2] as f32
            + matrix[12];
        let projected_y = matrix[1] * point[0] as f32
            + matrix[5] * point[1] as f32
            + matrix[9] * point[2] as f32
            + matrix[13];
        assert!((f64::from(projected_x) - cursor[0]).abs() < 1e-6);
        assert!((f64::from(projected_y) - cursor[1]).abs() < 1e-6);
    }
}
