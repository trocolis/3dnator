//! Preparação assíncrona de tampas de visualização, sem objetos CAD na GPU.
use super::*;
use std::{
    ops::Range,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
};

#[derive(Clone)]
pub(super) struct Source {
    pub mesh: Mesh,
    pub ranges: Vec<Range<usize>>,
    pub styles: Vec<u8>,
    pub center: Point3,
    pub scale: f64,
}
pub(super) struct Prepared {
    pub triangles: Vec<u8>,
    pub lines: Vec<u8>,
    pub opaque: usize,
}

pub(super) fn vertex(
    bytes: &mut Vec<u8>,
    p: Point3,
    n: Point3,
    attribute: u32,
    color: [u8; 4],
    center: Point3,
    scale: f64,
) {
    for v in sub(p, center) {
        bytes.extend_from_slice(&((v / scale) as f32).to_le_bytes());
    }
    for v in n {
        bytes.extend_from_slice(&(v as f32).to_le_bytes());
    }
    bytes.extend_from_slice(&attribute.to_le_bytes());
    bytes.extend_from_slice(&color);
}

fn prepare(source: &Source, plane: SectionPlane) -> Prepared {
    let mut opaque = Vec::new();
    let mut transparent = Vec::new();
    let mut lines = Vec::new();
    let n = plane.normal.map(|x| x / norm(plane.normal));
    for (group, range) in source.ranges.iter().enumerate() {
        let cap = nator_geometry::section::cap(&source.mesh, range.clone(), plane.point, n);
        let style = source
            .styles
            .get(range.start)
            .copied()
            .unwrap_or(STYLE_NORMAL);
        let target = if style == STYLE_DIMMED || style == STYLE_TRANSPARENT {
            &mut transparent
        } else {
            &mut opaque
        };
        for triangle in cap.triangles {
            for p in triangle {
                // Bit 16 identifica tampa; estilos continuam nos bits 8–15.
                vertex(
                    target,
                    p,
                    n,
                    (u32::from(style) << 8) | (1 << 16) | (((group % 2) as u32) << 19),
                    [204, 92, 76, 255],
                    source.center,
                    source.scale,
                );
            }
        }
        for segment in cap.segments {
            for p in segment {
                vertex(
                    &mut lines,
                    p,
                    n,
                    1 << 18,
                    [64, 23, 18, 255],
                    source.center,
                    source.scale,
                );
            }
        }
    }
    let count = opaque.len() / VERTEX_BYTES;
    opaque.extend(transparent);
    Prepared {
        triangles: opaque,
        lines,
        opaque: count,
    }
}

#[derive(Default)]
pub(super) struct State {
    pub source: Option<Arc<Source>>,
    running: Option<(SectionPlane, Receiver<Prepared>)>,
    ready: Option<SectionPlane>,
    pub triangles: Option<wgpu::Buffer>,
    pub lines: Option<wgpu::Buffer>,
    pub count: u32,
    pub opaque: u32,
    pub line_count: u32,
}
impl State {
    pub fn pending(&self, plane: Option<SectionPlane>) -> bool {
        plane.is_some() && self.source.is_some() && self.ready != plane
    }
    pub fn update(&mut self, device: &wgpu::Device, plane: Option<SectionPlane>) {
        if self.ready != plane {
            self.ready = None;
            self.count = 0;
            self.line_count = 0;
        }
        if let Some((requested, receiver)) = &self.running {
            match receiver.try_recv() {
                Ok(prepared) => {
                    if Some(*requested) == plane {
                        let buffer = |bytes: &[u8], label| {
                            (!bytes.is_empty()).then(|| {
                                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some(label),
                                    contents: bytes,
                                    usage: wgpu::BufferUsages::VERTEX,
                                })
                            })
                        };
                        self.count = (prepared.triangles.len() / VERTEX_BYTES) as u32;
                        self.opaque = prepared.opaque as u32;
                        self.line_count = (prepared.lines.len() / VERTEX_BYTES) as u32;
                        self.triangles = buffer(&prepared.triangles, "Tampas de seção");
                        self.lines = buffer(&prepared.lines, "Divisórias de seção");
                        self.ready = plane;
                    }
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.running = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if plane.is_none() {
            self.ready = None;
        }
        if self.pending(plane)
            && self.running.is_none()
            && let (Some(plane), Some(source)) = (plane, self.source.clone())
        {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = sender.send(prepare(&source, plane));
            });
            self.running = Some((plane, receiver));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touching_occurrences_keep_two_caps_and_the_shared_border() -> Result<(), String> {
        let first = nator_geometry::reference_bracket().map_err(|e| e.to_string())?;
        let mut vertices = first.vertices().to_vec();
        let mut triangles = first.triangles().to_vec();
        let offset = vertices.len() as u32;
        vertices.extend(first.vertices().iter().map(|p| [p[0] + 60., p[1], p[2]]));
        triangles.extend(first.triangles().iter().map(|t| t.map(|i| i + offset)));
        let count = first.triangles().len();
        let mesh = Mesh::new(vertices, triangles).map_err(|e| e.to_string())?;
        let source = Source {
            center: mesh.bounds().center(),
            scale: 120.,
            mesh,
            ranges: vec![0..count, count..2 * count],
            styles: Vec::new(),
        };
        for normal in [[0., 0., 1.], [0., 0., -1.]] {
            let plane = SectionPlane {
                point: [0., 0., 3.],
                normal,
            };
            let result = prepare(&source, plane);
            assert!(!result.triangles.is_empty());
            assert_eq!(result.opaque * VERTEX_BYTES, result.triangles.len());
            assert!(!result.lines.is_empty());
            let boundary =
                nator_geometry::section::cap(&source.mesh, 0..count, plane.point, normal);
            assert!(
                boundary
                    .segments
                    .iter()
                    .any(|s| s.iter().all(|p| (p[0] - 60.).abs() < 1e-8))
            );
            let neighbor =
                nator_geometry::section::cap(&source.mesh, count..2 * count, plane.point, normal);
            assert!(
                neighbor
                    .segments
                    .iter()
                    .any(|s| s.iter().all(|p| (p[0] - 60.).abs() < 1e-8))
            );
        }
        Ok(())
    }
}
