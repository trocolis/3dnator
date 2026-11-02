//! Ferramentas de fixação para peças impressas: anel de ressalto, torre de parafuso e
//! presilha. Devolvem sólidos para unir ou cortar; quem compõe o corpo é a operação.
use super::{
    KernelError, Solid, coordinates, last_error, owner_name, positive, sketch_error, unit,
};
use nator_geometry::{Point3, silhouette};
use nator_sketch::{Curve, Plane, Point2, Sketch, profiles};
use std::ffi::c_void;

unsafe extern "C" {
    fn nator_rim_size(body: *const c_void, index: usize, size: *mut usize) -> bool;
    fn nator_rim_copy(body: *const c_void, index: usize, output: *mut f64, length: usize) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn nator_rim_ring(
        body: *const c_void,
        index: usize,
        wire: usize,
        near: *const f64,
        near_count: usize,
        far: *const f64,
        far_count: usize,
        start: f64,
        end: f64,
    ) -> *mut c_void;
}

/// Medidas de uma haste em balanço, em mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapGeometry {
    pub length: f64,
    pub thickness: f64,
    pub width: f64,
    pub overhang: f64,
    pub head: f64,
    pub fillet: f64,
}

/// Vértice do contorno; com `Some(centro)`, o trecho até o seguinte é um arco horário.
type Corner = (Point2, Option<Point2>);

/// Área única de um contorno de retas e arcos no plano.
fn outline(
    name: &str,
    plane: Plane,
    corners: &[Corner],
) -> Result<(Sketch, profiles::Region), KernelError> {
    let mut sketch = Sketch::new(name, plane);
    let ids = corners
        .iter()
        .map(|(p, _)| sketch.add_point(*p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(sketch_error)?;
    for (i, (_, center)) in corners.iter().enumerate() {
        let (start, end) = (ids[i], ids[(i + 1) % ids.len()]);
        let curve = match center {
            Some(center) => Curve::Arc {
                center: sketch.add_point(*center).map_err(sketch_error)?,
                start,
                end,
                clockwise: true,
            },
            None => Curve::Line { start, end },
        };
        sketch.add_curve(curve, false).map_err(sketch_error)?;
    }
    let region = profiles::regions(&sketch)
        .map_err(sketch_error)?
        .into_iter()
        .next()
        .ok_or_else(|| KernelError("Contorno sem área.".into()))?;
    Ok((sketch, region))
}

/// Cada ocorrência tem faces próprias: `b1:e3` vira `b1:torre0:e3`.
fn occurrence_names(solid: &mut Solid, role: &str, index: usize) {
    solid.map_face_names(|name| name.replacen(':', &format!(":{role}{index}:"), 1));
}

impl Solid {
    /// Plano de uma face plana e seus contornos nesse plano, com o externo primeiro.
    fn rim_contours(&self, face: usize) -> Result<(Plane, Vec<Vec<Point2>>), KernelError> {
        let mut size = 0;
        // SAFETY: sólido vivo e destino exclusivo de um usize.
        if !unsafe { nator_rim_size(self.pointer.as_ptr(), face, &mut size) } {
            return Err(last_error());
        }
        let mut records = vec![0.; size];
        // SAFETY: buffer exclusivo do tamanho que a ponte acabou de informar.
        if !unsafe { nator_rim_copy(self.pointer.as_ptr(), face, records.as_mut_ptr(), size) } {
            return Err(last_error());
        }
        let invalid = || KernelError("Registro de contornos inválido.".into());
        let (head, mut rest) = records.split_at_checked(10).ok_or_else(invalid)?;
        let vector = |at: usize| [head[at], head[at + 1], head[at + 2]];
        let plane = Plane {
            origin: vector(0),
            u: vector(3),
            v: vector(6),
        };
        let mut contours = Vec::new();
        for _ in 0..head[9] as usize {
            let (count, tail) = rest.split_first().ok_or_else(invalid)?;
            let (points, tail) = tail
                .split_at_checked(*count as usize * 2)
                .ok_or_else(invalid)?;
            contours.push(points.as_chunks().0.to_vec());
            rest = tail;
        }
        Ok((plane, contours))
    }

    /// Prismas dos anéis entre `from` e `to` mm do contorno da face, para dentro do material,
    /// e entre `start` e `end` mm ao longo da normal externa. Com `inner`, os anéis acompanham
    /// cada contorno interno; sem ele, o contorno externo.
    #[allow(clippy::too_many_arguments)]
    pub fn rim_prism(
        &self,
        face: usize,
        inner: bool,
        from: f64,
        to: f64,
        start: f64,
        end: f64,
        owner: &str,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        if !from.is_finite() || !to.is_finite() || from < 0. || to <= from {
            return Err(KernelError("Medidas do anel inválidas.".into()));
        }
        let (_, contours) = self.rim_contours(face)?;
        let wires = if inner { 1..contours.len() } else { 0..1 };
        if wires.is_empty() {
            return Err(KernelError("A face não tem contorno interno.".into()));
        }
        let sign = if inner { 1. } else { -1. };
        let shifted = |contour: &[Point2], distance: f64| {
            silhouette::shifted(contour, sign * distance)
                .map(|points| points.concat())
                .ok_or_else(|| KernelError("A largura divide o contorno da face.".into()))
        };
        let mut result: Option<Self> = None;
        for wire in wires {
            let contour = &contours[wire];
            let near = if from > 1e-9 {
                shifted(contour, from)?
            } else {
                Vec::new()
            };
            let far = shifted(contour, to)?;
            // SAFETY: buffers de pares de doubles vivos durante a chamada, sem retenção.
            let ring = Self::from_pointer(unsafe {
                nator_rim_ring(
                    self.pointer.as_ptr(),
                    face,
                    wire,
                    near.as_ptr(),
                    near.len() / 2,
                    far.as_ptr(),
                    far.len() / 2,
                    start,
                    end,
                )
            })?;
            result = Some(match result {
                Some(rings) => {
                    let mut rings = rings;
                    rings.name_faces_by_index(owner)?;
                    let mut ring = ring;
                    ring.name_faces_by_index(owner)?;
                    rings.boolean(&ring, 0, owner)?
                }
                None => ring,
            });
        }
        let mut tool = result.ok_or_else(|| KernelError("Anel vazio.".into()))?;
        tool.name_faces_by_index(owner)?;
        occurrence_names(&mut tool, "rim", face);
        tool.volume()?;
        Ok(tool)
    }

    /// Torre cilíndrica com concordância na base, revolvida em torno de `axis`.
    pub fn boss_tool(
        center: Point3,
        axis: Point3,
        diameter: f64,
        height: f64,
        fillet: f64,
        owner: &str,
        index: usize,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        coordinates(center)?;
        positive(diameter, "Diâmetro")?;
        positive(height, "Altura")?;
        let axis = unit(axis, "O eixo da torre")?;
        let helper = if axis[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let radial = unit(
            nator_geometry::cross(axis, helper),
            "O eixo radial da torre",
        )?;
        let plane = Plane {
            origin: center,
            u: radial,
            v: axis,
        };
        let radius = diameter / 2.;
        let mut corners = vec![([0., 0.], None)];
        if fillet > 0. {
            corners.push(([radius + fillet, 0.], Some([radius + fillet, fillet])));
            corners.push(([radius, fillet], None));
        } else {
            corners.push(([radius, 0.], None));
        }
        corners.extend([([radius, height], None), ([0., height], None)]);
        let (sketch, region) = outline("Torre", plane, &corners)?;
        let mut tool = Self::revolve(&sketch, &region, center, axis, std::f64::consts::TAU, owner)?;
        occurrence_names(&mut tool, "boss", index);
        Ok(tool)
    }

    /// Haste com cabeça no plano `(u, v)`: nasce em `v = 0`, sobe por `v` e a cabeça avança
    /// por `u`. A largura fica centrada no plano.
    pub fn snap_hook(
        plane: Plane,
        size: SnapGeometry,
        owner: &str,
        index: usize,
    ) -> Result<Self, KernelError> {
        owner_name(owner)?;
        let SnapGeometry {
            length: l,
            thickness: t,
            width,
            overhang: o,
            head: h,
            fillet: r,
        } = size;
        for value in [l, t, width, o, h] {
            positive(value, "Medida da presilha")?;
        }
        let mut corners: Vec<Corner> = Vec::new();
        if r > 0. {
            corners.extend([
                ([-r, 0.], None),
                ([t + r, 0.], Some([t + r, r])),
                ([t, r], None),
            ]);
        } else {
            corners.extend([([0., 0.], None), ([t, 0.], None)]);
        }
        corners.extend([
            ([t, l], None),
            ([t + o, l], None),
            ([t, l + h], None),
            ([0., l + h], None),
        ]);
        if r > 0. {
            corners.push(([0., r], Some([-r, r])));
        }
        let (sketch, region) = outline("Presilha", plane, &corners)?;
        let mut tool = Self::extrude(&sketch, &region, -width / 2., width / 2., owner)?;
        occurrence_names(&mut tool, "snap", index);
        Ok(tool)
    }

    /// Janela da presilha: envolve a cabeça com `clearance` e avança `window` além dela.
    pub fn snap_window(
        plane: Plane,
        size: SnapGeometry,
        clearance: f64,
        window: f64,
        owner: &str,
        index: usize,
    ) -> Result<Self, KernelError> {
        let SnapGeometry {
            length,
            thickness,
            width,
            overhang,
            head,
            ..
        } = size;
        let (left, right) = (thickness - clearance, thickness + overhang + window);
        let (bottom, top) = (length - clearance, length + head + clearance);
        let half = width / 2. + clearance;
        let contour = [[left, bottom], [right, bottom], [right, top], [left, top]];
        let mut tool = Self::contour_prism(plane, &contour, 1, -half, half, owner)?;
        occurrence_names(&mut tool, "snap", index);
        Ok(tool)
    }
}
