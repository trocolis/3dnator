//! Hierarquia de definições, ocorrências e grupos organizacionais sem vínculos geométricos.
use super::*;
use nator_geometry::Point3;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub members: Vec<InstanceId>,
}
#[derive(Debug, Clone)]
pub struct Occurrence {
    pub instance: InstanceId,
    pub path: Vec<InstanceId>,
    pub placement: Placement,
}
impl Document {
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
    pub fn restore_groups(&mut self, groups: Vec<Group>) -> Result<(), DocumentError> {
        let mut candidate = self.clone();
        candidate.groups = groups;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn add_instance(
        &mut self,
        part: PartId,
        parent: Option<PartId>,
        placement: Placement,
    ) -> Result<InstanceId, DocumentError> {
        let name = self
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
            .name
            .clone();
        if parent.is_some_and(|p| self.part(p).is_none()) {
            return Err(DocumentError("Peça pai inexistente.".into()));
        }
        let id = InstanceId(self.allocate_id()?);
        self.instances.push(Instance {
            id,
            part,
            parent,
            name,
            placement,
            visible: true,
        });
        self.validate_assembly()?;
        Ok(id)
    }
    pub fn reparent(
        &mut self,
        id: InstanceId,
        parent: Option<PartId>,
    ) -> Result<(), DocumentError> {
        self.instance_mut(id)?.parent = parent;
        self.groups
            .iter_mut()
            .for_each(|g| g.members.retain(|m| *m != id));
        self.validate_assembly()
    }
    pub fn remove_instance(&mut self, id: InstanceId) -> Result<(), DocumentError> {
        if self.instance(id).is_none() {
            return Err(DocumentError("Instância inexistente.".into()));
        }
        self.instances.retain(|i| i.id != id);
        self.groups
            .iter_mut()
            .for_each(|g| g.members.retain(|m| *m != id));
        Ok(())
    }
    pub fn add_group(
        &mut self,
        name: String,
        members: Vec<InstanceId>,
    ) -> Result<GroupId, DocumentError> {
        validate_name(&name)?;
        if members.is_empty() {
            return Err(DocumentError("Selecione elementos para o grupo.".into()));
        }
        let id = GroupId(self.allocate_id()?);
        self.groups.push(Group { id, name, members });
        Ok(id)
    }
    pub fn move_group(&mut self, id: GroupId, delta: Placement) -> Result<(), DocumentError> {
        let members = self
            .groups
            .iter()
            .find(|g| g.id == id)
            .ok_or_else(|| DocumentError("Grupo inexistente.".into()))?
            .members
            .clone();
        for member in members {
            let i = self.instance_mut(member)?;
            i.placement = delta
                .compose(i.placement)
                .map_err(|e| DocumentError(e.to_string()))?;
        }
        Ok(())
    }
    pub fn remove_group(&mut self, id: GroupId) -> Result<(), DocumentError> {
        if !self.groups.iter().any(|g| g.id == id) {
            return Err(DocumentError("Grupo inexistente.".into()));
        }
        self.groups.retain(|g| g.id != id);
        Ok(())
    }
    /// Clonar copia definições e referências internas. Geometria imutável pode ser compartilhada
    /// até uma edição produzir nova geometria; parâmetros e rascunhos já são cópias independentes.
    pub fn clone_part(&mut self, source: PartId, name: String) -> Result<PartId, DocumentError> {
        validate_name(&name)?;
        self.validate_assembly()?;
        let mut map = HashMap::new();
        let id = self.clone_definition(source, &mut map)?;
        self.part_mut(id)?.name = name;
        Ok(id)
    }
    fn clone_definition(
        &mut self,
        source: PartId,
        map: &mut HashMap<PartId, PartId>,
    ) -> Result<PartId, DocumentError> {
        if let Some(id) = map.get(&source) {
            return Ok(*id);
        }
        let mut part = self
            .part(source)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
            .clone();
        let id = PartId(self.allocate_id()?);
        map.insert(source, id);
        part.id = id;
        let mut body_map = HashMap::new();
        let mut sketch_map = HashMap::new();
        for body in &mut part.bodies {
            let old = body.id;
            body.id = BodyId(self.allocate_id()?);
            body_map.insert(old, body.id);
        }
        // Os nomes de faces levam o corpo dono; no clone, passam aos novos corpos.
        for body in &mut part.bodies {
            if !body.face_names.is_empty() {
                body.face_names = Arc::new(
                    body.face_names
                        .iter()
                        .map(|n| super::features::remap_face_name(n, &body_map))
                        .collect(),
                );
            }
        }
        for sketch in &mut part.sketches {
            let old = sketch.id;
            sketch.id = SketchId(self.allocate_id()?);
            sketch_map.insert(old, sketch.id);
        }
        for feature in &mut part.features {
            feature.remap(&body_map, &sketch_map)?;
        }
        self.parts.push(part);
        let children = self
            .instances
            .iter()
            .filter(|i| i.parent == Some(source))
            .cloned()
            .collect::<Vec<_>>();
        for mut child in children {
            child.id = InstanceId(self.allocate_id()?);
            child.parent = Some(id);
            child.part = self.clone_definition(child.part, map)?;
            self.instances.push(child);
        }
        Ok(id)
    }
    pub(super) fn validate_assembly(&self) -> Result<(), DocumentError> {
        let mut done = HashSet::new();
        let mut visiting = HashSet::new();
        fn visit(
            doc: &Document,
            part: PartId,
            depth: usize,
            done: &mut HashSet<PartId>,
            visiting: &mut HashSet<PartId>,
        ) -> Result<(), DocumentError> {
            if depth > 64 {
                return Err(DocumentError("A hierarquia excede 64 níveis.".into()));
            }
            if done.contains(&part) {
                return Ok(());
            }
            if !visiting.insert(part) {
                return Err(DocumentError(
                    "Uma peça não pode conter a si mesma, direta ou indiretamente.".into(),
                ));
            }
            for child in doc.instances.iter().filter(|i| i.parent == Some(part)) {
                visit(doc, child.part, depth + 1, done, visiting)?;
            }
            visiting.remove(&part);
            done.insert(part);
            Ok(())
        }
        for i in &self.instances {
            if self.part(i.part).is_none() || i.parent.is_some_and(|p| self.part(p).is_none()) {
                return Err(DocumentError("Referência de montagem inexistente.".into()));
            }
        }
        for p in &self.parts {
            visit(self, p.id, 0, &mut done, &mut visiting)?;
        }
        Ok(())
    }
    /// Caminhos distinguem ocorrências de uma subpeça quando o pai tem várias instâncias.
    pub fn occurrences(&self) -> Result<Vec<Occurrence>, DocumentError> {
        fn walk(
            doc: &Document,
            parent: Option<PartId>,
            placement: Placement,
            path: &mut Vec<InstanceId>,
            out: &mut Vec<Occurrence>,
        ) -> Result<(), DocumentError> {
            if path.len() > 64 {
                return Err(DocumentError("Hierarquia cíclica ou excessiva.".into()));
            }
            for i in doc
                .instances
                .iter()
                .filter(|i| i.parent == parent && i.visible)
            {
                if out.len() >= MAX_ELEMENTS {
                    return Err(DocumentError(
                        "A montagem excede dez mil ocorrências visíveis.".into(),
                    ));
                }
                let world = placement
                    .compose(i.placement)
                    .map_err(|e| DocumentError(e.to_string()))?;
                path.push(i.id);
                out.push(Occurrence {
                    instance: i.id,
                    path: path.clone(),
                    placement: world,
                });
                walk(doc, Some(i.part), world, path, out)?;
                path.pop();
            }
            Ok(())
        }
        let mut out = Vec::new();
        walk(self, None, Placement::default(), &mut Vec::new(), &mut out)?;
        Ok(out)
    }
    pub fn instance_center(&self, id: InstanceId) -> Result<Point3, DocumentError> {
        let i = self
            .instance(id)
            .ok_or_else(|| DocumentError("Instância inexistente.".into()))?;
        let part = self
            .part(i.part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        let mut sum = [0.; 3];
        let mut weight = 0.;
        for b in &part.bodies {
            let mesh = b.geometry.mesh();
            // Centro de massa de casca aberta não é definido: usar caixa como referência.
            let (center, volume) = if b.geometry.topology().has_closed_oriented_edges() {
                mesh.mass_center()
                    .map_err(|e| DocumentError(e.to_string()))?
            } else {
                (mesh.bounds().center(), 1.)
            };
            let p = b.placement.apply(center);
            for k in 0..3 {
                sum[k] += p[k] * volume.abs();
            }
            weight += volume.abs();
        }
        Ok(i.placement.apply(if weight > 0. {
            sum.map(|v| v / weight)
        } else {
            [0.; 3]
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instances_share_definitions_but_clones_and_groups_are_independent()
    -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Montagem")?;
        let (part, body) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let first = doc.instances()[0].id;
        let second = doc.add_instance(part, None, Placement::new([100., 0., 0.], [0.; 3])?)?;
        let independent = doc.clone_part(part, "Variação".into())?;
        doc.part_mut(independent)?.bodies[0].name = "Somente clone".into();
        assert_eq!(doc.body(body).ok_or("Corpo ausente")?.name, "Suporte");
        let group = doc.add_group("Organização".into(), vec![first, second])?;
        doc.move_group(group, Placement::new([0., 50., 0.], [0.; 3])?)?;
        doc.remove_group(group)?;
        assert_eq!(
            doc.instance(second)
                .ok_or("Instância ausente")?
                .placement
                .translation_mm(),
            [100., 50., 0.]
        );
        assert_eq!(
            doc.body(body)
                .ok_or("Corpo ausente")?
                .placement
                .translation_mm(),
            [0.; 3]
        );
        doc.validate()?;
        Ok(())
    }
    #[test]
    fn nested_origins_compose_and_cycles_are_rejected() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Montagem")?;
        let parent = doc.create_part("Conjunto".into())?;
        let (child, _) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let root = doc.instances()[0].id;
        let nested = doc.instances()[1].id;
        doc.instance_mut(root)?.placement =
            Placement::new([100., 0., 0.], [0., 0., std::f64::consts::FRAC_PI_2])?;
        doc.reparent(nested, Some(parent))?;
        doc.instance_mut(nested)?.placement = Placement::new([20., 0., 0.], [0.; 3])?;
        doc.validate()?;
        let scene = doc.scene_mesh()?.ok_or("Cena ausente")?;
        assert!((scene.bounds().min[0] - 80.).abs() < 1e-9);
        assert!((scene.bounds().min[1] - 20.).abs() < 1e-9);
        assert!(
            doc.add_instance(parent, Some(child), Placement::default())
                .is_err()
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Pick {
    pub instance: InstanceId,
    pub part: PartId,
    pub body: BodyId,
    /// Triângulo atingido na malha local do corpo.
    pub triangle: usize,
    pub point: Point3,
    pub normal: Point3,
    pub vertices: [Point3; 3],
    pub placement: Placement,
}
impl Document {
    pub fn pick(&self, ray: nator_geometry::query::Ray) -> Result<Option<Pick>, DocumentError> {
        self.pick_with(ray, |_| true)
    }
    /// Seleção que ignora ocorrências recusadas por `allowed` (bloqueio de seleção ou
    /// isolamento), mesmo que estejam visíveis.
    pub fn pick_with(
        &self,
        ray: nator_geometry::query::Ray,
        allowed: impl Fn(&Occurrence) -> bool,
    ) -> Result<Option<Pick>, DocumentError> {
        let mut nearest = f64::INFINITY;
        let mut result = None;
        for occurrence in self.occurrences()?.into_iter().filter(|o| allowed(o)) {
            let instance = self
                .instance(occurrence.instance)
                .ok_or_else(|| DocumentError("Instância inexistente.".into()))?;
            let part = self
                .part(instance.part)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
            for body in part.bodies.iter().filter(|b| b.visible) {
                let pose = occurrence
                    .placement
                    .compose(body.placement)
                    .map_err(|e| DocumentError(e.to_string()))?;
                let origin = pose.inverse_apply(ray.origin);
                let end =
                    pose.inverse_apply(std::array::from_fn(|i| ray.origin[i] + ray.direction[i]));
                let local = nator_geometry::query::Ray {
                    origin,
                    direction: nator_geometry::sub(end, origin),
                };
                if let Some(hit) = body.geometry.mesh().ray_hit(local)
                    && hit.distance < nearest
                {
                    nearest = hit.distance;
                    let zero = pose.apply([0.; 3]);
                    result = Some(Pick {
                        instance: instance.id,
                        part: part.id,
                        body: body.id,
                        triangle: hit.triangle,
                        point: pose.apply(hit.point),
                        normal: nator_geometry::sub(pose.apply(hit.normal), zero),
                        vertices: body
                            .geometry
                            .mesh()
                            .triangle_points(body.geometry.mesh().triangles()[hit.triangle])
                            .map(|p| pose.apply(p)),
                        placement: occurrence.placement,
                    });
                }
            }
        }
        Ok(result)
    }
}
