//! Geometria e intenção de desenho independentes da interface e do núcleo de sólidos.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error::Error, fmt};

pub mod profiles;
pub mod repair;
pub mod snapping;
pub mod solver;
pub mod tools;
pub type Point2 = [f64; 2];
pub const MAX_POINTS: usize = 512;
pub const MAX_ENTITIES: usize = 1024;
pub const MAX_CONSTRAINTS: usize = 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct SketchError(pub String);
impl fmt::Display for SketchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for SketchError {}
pub type Result<T> = std::result::Result<T, SketchError>;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    pub origin: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
}
impl Default for Plane {
    fn default() -> Self {
        Self::xy()
    }
}
impl Plane {
    pub fn xy() -> Self {
        Self {
            origin: [0.; 3],
            u: [1., 0., 0.],
            v: [0., 1., 0.],
        }
    }
    pub fn xz() -> Self {
        Self {
            origin: [0.; 3],
            u: [1., 0., 0.],
            v: [0., 0., 1.],
        }
    }
    pub fn yz() -> Self {
        Self {
            origin: [0.; 3],
            u: [0., 1., 0.],
            v: [0., 0., 1.],
        }
    }
    /// Plano por um ponto e uma normal, com eixos U/V escolhidos de forma determinística
    /// (U perpendicular à normal e ao eixo principal menos alinhado com ela; V = N × U),
    /// de modo que U × V = N.
    pub fn from_normal(origin: [f64; 3], normal: [f64; 3]) -> Result<Self> {
        use nator_geometry::{cross, norm};
        let length = norm(normal);
        if !length.is_finite() || length < 1e-9 || origin.iter().any(|v| !v.is_finite()) {
            return Err(SketchError(
                "Face sem normal definida para o rascunho.".into(),
            ));
        }
        let n = normal.map(|v| v / length);
        let helper = if n[2].abs() < 0.9 {
            [0., 0., 1.]
        } else {
            [0., 1., 0.]
        };
        let u = cross(helper, n);
        let u = u.map(|v| v / norm(u));
        let plane = Self {
            origin,
            u,
            v: cross(n, u),
        };
        plane.validate()?;
        Ok(plane)
    }
    pub fn validate(self) -> Result<()> {
        use nator_geometry::{dot, norm};
        if self
            .origin
            .into_iter()
            .chain(self.u)
            .chain(self.v)
            .any(|x| !x.is_finite())
            || (norm(self.u) - 1.).abs() > 1e-8
            || (norm(self.v) - 1.).abs() > 1e-8
            || dot(self.u, self.v).abs() > 1e-8
        {
            return Err(SketchError(
                "O plano deve ter eixos unitários e perpendiculares.".into(),
            ));
        }
        Ok(())
    }
    pub fn world(self, p: Point2) -> [f64; 3] {
        std::array::from_fn(|i| self.origin[i] + self.u[i] * p[0] + self.v[i] * p[1])
    }
    pub fn project(self, p: [f64; 3]) -> Point2 {
        let delta = nator_geometry::sub(p, self.origin);
        [
            nator_geometry::dot(delta, self.u),
            nator_geometry::dot(delta, self.v),
        ]
    }
    pub fn normal(self) -> [f64; 3] {
        nator_geometry::cross(self.u, self.v)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: u64,
    pub position: Point2,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Curve {
    Line {
        start: u64,
        end: u64,
    },
    Circle {
        center: u64,
        radius: f64,
    },
    Arc {
        center: u64,
        start: u64,
        end: u64,
        clockwise: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: u64,
    pub curve: Curve,
    pub construction: bool,
}
impl Entity {
    pub fn point_ids(&self) -> Vec<u64> {
        match self.curve {
            Curve::Line { start, end } => vec![start, end],
            Curve::Circle { center, .. } => vec![center],
            Curve::Arc {
                center, start, end, ..
            } => vec![center, start, end],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    Tangent { line: u64, arc: u64, point: u64 },
    Fixed { point: u64, position: Point2 },
    Coincident { a: u64, b: u64 },
    Horizontal { line: u64 },
    Vertical { line: u64 },
    Distance { a: u64, b: u64, value: f64 },
    DeltaX { a: u64, b: u64, value: f64 },
    DeltaY { a: u64, b: u64, value: f64 },
    Radius { circle: u64, value: f64 },
    Parallel { a: u64, b: u64 },
    Perpendicular { a: u64, b: u64 },
    EqualLength { a: u64, b: u64 },
    Midpoint { point: u64, line: u64 },
    PointOnLine { point: u64, line: u64 },
    Concentric { a: u64, b: u64 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: u64,
    pub kind: ConstraintKind,
    pub reference: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sketch {
    pub name: String,
    pub plane: Plane,
    pub next_id: u64,
    pub points: Vec<Point>,
    pub entities: Vec<Entity>,
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub dimension_expressions: std::collections::BTreeMap<u64, String>,
}
impl Sketch {
    pub fn new(name: impl Into<String>, plane: Plane) -> Self {
        Self {
            name: name.into(),
            plane,
            next_id: 1,
            points: Vec::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
            dimension_expressions: std::collections::BTreeMap::new(),
        }
    }
    fn allocate(&mut self) -> Result<u64> {
        let id = self.next_id;
        self.next_id = id
            .checked_add(1)
            .ok_or_else(|| SketchError("Identificadores do rascunho esgotados.".into()))?;
        Ok(id)
    }
    pub fn point(&self, id: u64) -> Result<Point2> {
        self.points
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.position)
            .ok_or_else(|| SketchError(format!("Ponto {id} inexistente.")))
    }
    pub fn entity(&self, id: u64) -> Result<&Entity> {
        self.entities
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| SketchError(format!("Entidade {id} inexistente.")))
    }
    pub fn line(&self, id: u64) -> Result<(Point2, Point2)> {
        match self.entity(id)?.curve {
            Curve::Line { start, end } => Ok((self.point(start)?, self.point(end)?)),
            _ => Err(SketchError("A entidade deve ser uma linha.".into())),
        }
    }
    pub fn circle(&self, id: u64) -> Result<(Point2, f64)> {
        match self.entity(id)?.curve {
            Curve::Circle { center, radius } => Ok((self.point(center)?, radius)),
            _ => Err(SketchError("A entidade deve ser um círculo.".into())),
        }
    }
    pub fn radial(&self, id: u64) -> Result<(Point2, f64)> {
        match self.entity(id)?.curve {
            Curve::Circle { center, radius } => Ok((self.point(center)?, radius)),
            Curve::Arc { center, start, .. } => {
                let c = self.point(center)?;
                Ok((c, distance(c, self.point(start)?)))
            }
            _ => Err(SketchError(
                "A entidade deve ser um círculo ou arco.".into(),
            )),
        }
    }
    pub fn refresh_reference_dimensions(&mut self) -> Result<()> {
        let source = self.clone();
        for c in self.constraints.iter_mut().filter(|c| c.reference) {
            match &mut c.kind {
                ConstraintKind::Distance { a, b, value } => {
                    *value = distance(source.point(*a)?, source.point(*b)?)
                }
                ConstraintKind::DeltaX { a, b, value } => {
                    *value = source.point(*b)?[0] - source.point(*a)?[0]
                }
                ConstraintKind::DeltaY { a, b, value } => {
                    *value = source.point(*b)?[1] - source.point(*a)?[1]
                }
                ConstraintKind::Radius { circle, value } => *value = source.radial(*circle)?.1,
                _ => {}
            }
        }
        Ok(())
    }
    pub fn add_point(&mut self, position: Point2) -> Result<u64> {
        if position.iter().any(|v| !v.is_finite()) || self.points.len() >= MAX_POINTS {
            return Err(SketchError(
                "Ponto inválido ou limite de pontos atingido.".into(),
            ));
        }
        let id = self.allocate()?;
        self.points.push(Point { id, position });
        Ok(id)
    }
    pub fn add_curve(&mut self, curve: Curve, construction: bool) -> Result<u64> {
        if self.entities.len() >= MAX_ENTITIES {
            return Err(SketchError("Limite de entidades atingido.".into()));
        }
        let id = self.allocate()?;
        self.entities.push(Entity {
            id,
            curve,
            construction,
        });
        Ok(id)
    }
    pub fn add_line(&mut self, a: Point2, b: Point2, construction: bool) -> Result<u64> {
        if distance(a, b) < 1e-9 {
            return Err(SketchError(
                "Uma linha precisa ter comprimento positivo.".into(),
            ));
        }
        let start = self.add_point(a)?;
        let end = self.add_point(b)?;
        self.add_curve(Curve::Line { start, end }, construction)
    }
    pub fn add_circle(&mut self, center: Point2, radius: f64, construction: bool) -> Result<u64> {
        if !radius.is_finite() || radius <= 1e-9 {
            return Err(SketchError("O raio deve ser positivo e finito.".into()));
        }
        let center = self.add_point(center)?;
        self.add_curve(Curve::Circle { center, radius }, construction)
    }
    pub fn add_rectangle(&mut self, a: Point2, b: Point2, construction: bool) -> Result<Vec<u64>> {
        if (a[0] - b[0]).abs() < 1e-9 || (a[1] - b[1]).abs() < 1e-9 {
            return Err(SketchError(
                "O retângulo precisa ter largura e altura positivas.".into(),
            ));
        }
        let points = [a, [b[0], a[1]], b, [a[0], b[1]]]
            .map(|p| self.add_point(p))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let mut lines = Vec::new();
        for i in 0..4 {
            let line = self.add_curve(
                Curve::Line {
                    start: points[i],
                    end: points[(i + 1) % 4],
                },
                construction,
            )?;
            self.add_constraint(
                if i % 2 == 0 {
                    ConstraintKind::Horizontal { line }
                } else {
                    ConstraintKind::Vertical { line }
                },
                false,
            )?;
            lines.push(line);
        }
        Ok(lines)
    }
    pub fn add_constraint(&mut self, kind: ConstraintKind, reference: bool) -> Result<u64> {
        if self.constraints.len() >= MAX_CONSTRAINTS {
            return Err(SketchError("Limite de restrições atingido.".into()));
        }
        let id = self.allocate()?;
        self.constraints.push(Constraint {
            id,
            kind,
            reference,
        });
        Ok(id)
    }
    pub fn validate(&self) -> Result<()> {
        self.plane.validate()?;
        if self.name.trim().is_empty()
            || self.name.len() > 1024
            || self.next_id == 0
            || self.points.len() > MAX_POINTS
            || self.entities.len() > MAX_ENTITIES
            || self.constraints.len() > MAX_CONSTRAINTS
        {
            return Err(SketchError(
                "Metadados ou tamanho do rascunho inválidos.".into(),
            ));
        }
        let mut ids = HashSet::new();
        for id in self
            .points
            .iter()
            .map(|p| p.id)
            .chain(self.entities.iter().map(|e| e.id))
            .chain(self.constraints.iter().map(|c| c.id))
        {
            if id == 0 || id >= self.next_id || !ids.insert(id) {
                return Err(SketchError(
                    "Identificadores de rascunho inválidos ou repetidos.".into(),
                ));
            }
        }
        for p in &self.points {
            if p.position.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
                return Err(SketchError("Coordenada do rascunho inválida.".into()));
            }
        }
        for entity in &self.entities {
            for point in entity.point_ids() {
                self.point(point)?;
            }
            match entity.curve {
                Curve::Line { start, end } => {
                    if distance(self.point(start)?, self.point(end)?) < 1e-9 {
                        return Err(SketchError("Linha de comprimento zero.".into()));
                    }
                }
                Curve::Circle { radius, .. } => {
                    if !radius.is_finite() || radius <= 1e-9 || radius > 1e12 {
                        return Err(SketchError("Raio inválido.".into()));
                    }
                }
                Curve::Arc {
                    center, start, end, ..
                } => {
                    let radius = distance(self.point(center)?, self.point(start)?);
                    let other = distance(self.point(center)?, self.point(end)?);
                    if radius < 1e-9 || (radius - other).abs() > 1e-6 * radius.max(1.) {
                        return Err(SketchError(
                            "Os extremos do arco devem pertencer ao mesmo círculo.".into(),
                        ));
                    }
                }
            }
        }
        for constraint in &self.constraints {
            solver::constraint_residuals(self, &constraint.kind)?;
        }
        for (id, expression) in &self.dimension_expressions {
            if expression.is_empty()
                || expression.len() > 4096
                || !self.constraints.iter().any(|c| {
                    c.id == *id
                        && !c.reference
                        && matches!(
                            c.kind,
                            ConstraintKind::Distance { .. }
                                | ConstraintKind::DeltaX { .. }
                                | ConstraintKind::DeltaY { .. }
                                | ConstraintKind::Radius { .. }
                        )
                })
            {
                return Err(SketchError(
                    "Expressão de cota inválida ou sem restrição correspondente.".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn prune_unused_points(&mut self) {
        let used: HashSet<_> = self.entities.iter().flat_map(Entity::point_ids).collect();
        self.points.retain(|p| used.contains(&p.id));
        // Restrições que perderam suas entidades/pontos são removidas apenas nesta ferramenta explícita.
        let candidate = self.clone();
        self.constraints
            .retain(|c| solver::constraint_residuals(&candidate, &c.kind).is_ok());
        self.dimension_expressions
            .retain(|id, _| self.constraints.iter().any(|c| c.id == *id));
    }
}
pub fn sub(a: Point2, b: Point2) -> Point2 {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn add(a: Point2, b: Point2) -> Point2 {
    [a[0] + b[0], a[1] + b[1]]
}
pub fn scale(p: Point2, s: f64) -> Point2 {
    [p[0] * s, p[1] * s]
}
pub fn dot(a: Point2, b: Point2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub fn cross(a: Point2, b: Point2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
pub fn distance(a: Point2, b: Point2) -> f64 {
    let d = sub(a, b);
    d[0].hypot(d[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planes_from_normals_are_right_handed_and_contain_the_origin() -> Result<()> {
        for normal in [[0., 0., 1.], [0., 0., -2.], [1., 1., 0.], [0.3, -0.2, 0.9]] {
            let plane = Plane::from_normal([5., 6., 7.], normal)?;
            let n = plane.normal();
            let length = nator_geometry::norm(normal);
            for (a, b) in n.iter().zip(normal.map(|v| v / length)) {
                assert!((a - b).abs() < 1e-12);
            }
            assert_eq!(plane.world([0., 0.]), [5., 6., 7.]);
            assert_eq!(
                plane.project(plane.world([3., -4.])).map(|v| v.round()),
                [3., -4.]
            );
        }
        assert!(Plane::from_normal([0.; 3], [0.; 3]).is_err());
        Ok(())
    }
}
