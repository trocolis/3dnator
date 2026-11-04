//! Realce 3D com o mesmo plano de corte e profundidade das superfícies.
use super::*;

#[derive(Default)]
pub(super) struct HoverBuffers {
    pub triangles: Option<wgpu::Buffer>,
    pub lines: Option<wgpu::Buffer>,
    pub triangle_count: u32,
    pub line_count: u32,
}
impl Viewport {
    pub fn clear_hover(&mut self) {
        self.hover_buffers = HoverBuffers::default();
    }

    /// Substitui a pré-seleção. Coordenadas de mundo; rótulos continuam na interface.
    pub fn set_hover(
        &mut self,
        device: &wgpu::Device,
        triangles: &[[Point3; 3]],
        segments: &[[Point3; 2]],
        points: &[Point3],
    ) {
        let (center, scale) = self.normalization;
        let (right, up, direction) = self.camera.basis();
        let mut faces = Vec::new();
        let mut lines = Vec::new();
        let vertex = |bytes: &mut Vec<u8>, p, flag| {
            section::vertex(
                bytes,
                p,
                [0., 0., 1.],
                flag,
                [64, 156, 236, 255],
                center,
                scale,
            )
        };
        for triangle in triangles {
            if dot(
                cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0])),
                direction,
            ) <= 0.
            {
                continue;
            }
            for &p in triangle {
                vertex(&mut faces, p, 0);
            }
        }
        for segment in segments {
            for &p in segment {
                vertex(&mut lines, p, 1 << 18);
            }
        }
        for &p in points {
            let relative = sub(p, center).map(|x| x / scale);
            let depth_factor = match self.camera.projection {
                Projection::Orthographic => 1.,
                Projection::Perspective => {
                    (self.camera.eye_distance() - dot(sub(relative, self.camera.target), direction))
                        / self.camera.eye_distance()
                }
            };
            let radius = self.camera.span * scale * 5. / f64::from(self.size[1]) * depth_factor;
            let at = |x: f64, y: f64| {
                std::array::from_fn(|i| p[i] + radius * (x * right[i] + y * up[i]))
            };
            let quad = [at(-1., -1.), at(1., -1.), at(1., 1.), at(-1., 1.)];
            for index in [0, 1, 2, 0, 2, 3] {
                vertex(&mut faces, quad[index], 1 << 17);
            }
        }
        let buffer = |bytes: &[u8]| {
            (!bytes.is_empty()).then(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Pré-seleção 3D"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                })
            })
        };
        self.hover_buffers = HoverBuffers {
            triangle_count: (faces.len() / VERTEX_BYTES) as u32,
            line_count: (lines.len() / VERTEX_BYTES) as u32,
            triangles: buffer(&faces),
            lines: buffer(&lines),
        };
    }
}
