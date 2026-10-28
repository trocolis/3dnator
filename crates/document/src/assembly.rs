//! Hierarquia de definições, ocorrências e grupos organizacionais sem vínculos geométricos.
use super::*;
use nator_geometry::{Bounds, Point3};
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
            color: None,
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
    pub fn rename_group(&mut self, id: GroupId, name: String) -> Result<(), DocumentError> {
        validate_name(&name)?;
        self.groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| DocumentError("Grupo inexistente.".into()))?
            .name = name;
        Ok(())
    }
    /// Cópia independente de um corpo na mesma peça, sem histórico: o B-rep e a malha são
    /// compartilhados até uma edição, e os nomes de faces passam ao novo dono, para que
    /// operações futuras sobre a cópia não confundam suas faces com as do original.
    pub fn copy_body(&mut self, id: BodyId, placement: Placement) -> Result<BodyId, DocumentError> {
        let source = self
            .body(id)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?
            .clone();
        let part = self
            .parts
            .iter()
            .find(|p| p.bodies.iter().any(|b| b.id == id))
            .map(|p| p.id)
            .ok_or_else(|| DocumentError("Peça do corpo inexistente.".into()))?;
        let copy = self.allocate_body_id()?;
        let map = HashMap::from([(id, copy)]);
        let face_names = Arc::new(
            source
                .face_names
                .iter()
                .map(|n| super::features::remap_face_name(n, &map))
                .collect(),
        );
        self.add_body(
            part,
            Body {
                id: copy,
                name: format!("{} (cópia)", source.name),
                face_names,
                placement,
                visible: true,
                ..source
            },
        )?;
        Ok(copy)
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
        let mut instances = HashMap::new();
        let snapshot = self.clone();
        let id = self.clone_definition(&snapshot, source, &mut map, &mut instances)?;
        self.part_mut(id)?.name = name;
        // Grupos internos pertencem à organização do conjunto clonado. Só copiamos
        // grupos inteiramente contidos nele; grupos do nível externo não ganham membros
        // inesperados nem continuam apontando para instâncias da peça original.
        let groups = self
            .groups
            .iter()
            .filter(|g| !g.members.is_empty())
            .filter_map(|g| {
                let members = g
                    .members
                    .iter()
                    .map(|id| instances.get(id).copied())
                    .collect::<Option<Vec<_>>>()?;
                Some((g.name.clone(), members))
            })
            .collect::<Vec<_>>();
        for (name, members) in groups {
            self.add_group(name, members)?;
        }
        Ok(id)
    }
    /// Copia uma definição de `from` (este documento ou outro) e suas subpeças com
    /// identificadores novos. Definições repetidas na hierarquia são copiadas uma vez.
    fn clone_definition(
        &mut self,
        from: &Document,
        source: PartId,
        map: &mut HashMap<PartId, PartId>,
        instances: &mut HashMap<InstanceId, InstanceId>,
    ) -> Result<PartId, DocumentError> {
        if let Some(id) = map.get(&source) {
            return Ok(*id);
        }
        let mut part = from
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
        // Planos de trabalho não são referenciados pelo histórico: só ganham identidade nova.
        for plane in &mut part.work_planes {
            plane.id = WorkPlaneId(self.allocate_id()?);
        }
        for feature in &mut part.features {
            feature.remap(&body_map, &sketch_map)?;
        }
        self.parts.push(part);
        let children = from
            .instances
            .iter()
            .filter(|i| i.parent == Some(source))
            .cloned()
            .collect::<Vec<_>>();
        for mut child in children {
            let instance = InstanceId(self.allocate_id()?);
            instances.insert(child.id, instance);
            child.id = instance;
            child.parent = Some(id);
            child.part = self.clone_definition(from, child.part, map, instances)?;
            self.instances.push(child);
        }
        Ok(id)
    }
    /// Importa a definição `part` de outro documento como peças novas e independentes,
    /// sem vínculo com o arquivo de origem. Subpeças repetidas continuam compartilhando a
    /// definição copiada; grupos internos ao conjunto vêm junto.
    pub fn import_part(&mut self, from: &Document, part: PartId) -> Result<PartId, DocumentError> {
        from.validate_assembly()?;
        let mut map = HashMap::new();
        let mut instances = HashMap::new();
        let id = self.clone_definition(from, part, &mut map, &mut instances)?;
        let groups = from
            .groups
            .iter()
            .filter(|g| !g.members.is_empty())
            .filter_map(|g| {
                let members = g
                    .members
                    .iter()
                    .map(|id| instances.get(id).copied())
                    .collect::<Option<Vec<_>>>()?;
                Some((g.name.clone(), members))
            })
            .collect::<Vec<_>>();
        for (name, members) in groups {
            self.add_group(name, members)?;
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
                .filter(|i| i.parent == parent && doc.shows_instance(i))
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
    /// Caixa no mundo dos corpos visíveis de cada ocorrência visível, com o caminho e a
    /// origem da ocorrência. Ocorrências sem corpos (peças vazias) não têm caixa.
    fn occurrence_boxes(&self) -> Result<Vec<(Occurrence, Option<Bounds>)>, DocumentError> {
        let mut out = Vec::new();
        for occurrence in self.occurrences()? {
            let part = self
                .instance(occurrence.instance)
                .and_then(|i| self.part(i.part))
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
            let mut bounds: Option<Bounds> = None;
            for body in part.shown_bodies() {
                let local = body.geometry.mesh().bounds();
                for corner in 0..8 {
                    let p = std::array::from_fn(|k| {
                        if corner >> k & 1 == 0 {
                            local.min[k]
                        } else {
                            local.max[k]
                        }
                    });
                    let world = occurrence.placement.apply(body.placement.apply(p));
                    bounds = Some(match bounds {
                        None => Bounds {
                            min: world,
                            max: world,
                        },
                        Some(b) => Bounds {
                            min: std::array::from_fn(|k| b.min[k].min(world[k])),
                            max: std::array::from_fn(|k| b.max[k].max(world[k])),
                        },
                    });
                }
            }
            out.push((occurrence, bounds));
        }
        Ok(out)
    }
    /// Desloca as instâncias dadas ao longo de +X do mundo, todas juntas, até que fiquem
    /// a uma pequena folga das demais ocorrências visíveis. Usado ao criar, importar ou
    /// copiar componentes, para que não apareçam sobrepostos. Sem sobreposição, nada muda.
    pub fn place_beside(&mut self, ids: &[InstanceId]) -> Result<(), DocumentError> {
        let boxes = self.occurrence_boxes()?;
        let moving = |o: &Occurrence| o.path.iter().any(|i| ids.contains(i));
        let union = |a: Option<Bounds>, b: Bounds| {
            Some(match a {
                None => b,
                Some(a) => Bounds {
                    min: std::array::from_fn(|k| a.min[k].min(b.min[k])),
                    max: std::array::from_fn(|k| a.max[k].max(b.max[k])),
                },
            })
        };
        let mut item: Option<Bounds> = None;
        let mut origin = None;
        let mut obstacles = Vec::new();
        let mut scene: Option<Bounds> = None;
        for (occurrence, bounds) in &boxes {
            if moving(occurrence) {
                origin.get_or_insert(occurrence.placement.apply([0.; 3]));
                if let Some(b) = bounds {
                    item = union(item, *b);
                }
            } else if let Some(b) = bounds {
                obstacles.push(*b);
                scene = union(scene, *b);
            }
        }
        // Peça vazia: a origem da ocorrência ocupa o lugar de um ponto.
        let Some(item) = item.or(origin.map(|p| Bounds { min: p, max: p })) else {
            return Ok(());
        };
        let largest = |b: Bounds| b.size().into_iter().fold(0_f64, f64::max);
        let reference = if largest(item) > 0. {
            largest(item)
        } else {
            scene.map(largest).unwrap_or(0.)
        };
        let gap = (0.1 * reference).clamp(SPACING.0, SPACING.1);
        let near = |a: Bounds, b: &Bounds| {
            (0..3).all(|k| a.min[k] < b.max[k] + gap && a.max[k] + gap > b.min[k])
        };
        let mut dx = 0_f64;
        // Cada passo ultrapassa ao menos um obstáculo; o deslocamento só cresce.
        for _ in 0..=obstacles.len() {
            let shifted = Bounds {
                min: [item.min[0] + dx, item.min[1], item.min[2]],
                max: [item.max[0] + dx, item.max[1], item.max[2]],
            };
            let push = obstacles
                .iter()
                .filter(|o| near(shifted, o))
                .map(|o| o.max[0] + gap - item.min[0])
                .fold(f64::NEG_INFINITY, f64::max);
            if push == f64::NEG_INFINITY {
                break;
            }
            dx = push;
        }
        if dx <= 0. {
            return Ok(());
        }
        for id in ids {
            // Deslocamento no mundo convertido para o sistema do pai da instância.
            let Some((occurrence, _)) = boxes.iter().find(|(o, _)| o.instance == *id) else {
                continue;
            };
            let local = self
                .instance(*id)
                .ok_or_else(|| DocumentError("Instância inexistente.".into()))?
                .placement;
            let parent = occurrence
                .placement
                .compose(local.inverse().map_err(|e| DocumentError(e.to_string()))?)
                .map_err(|e| DocumentError(e.to_string()))?;
            let delta = parent.inverse_vector([dx, 0., 0.]);
            self.instance_mut(*id)?.placement = local
                .translated(delta)
                .map_err(|e| DocumentError(e.to_string()))?;
        }
        Ok(())
    }
}

/// Folga mínima e máxima, em mm, entre um componente posicionado ao lado e os vizinhos;
/// entre esses limites, um décimo da maior dimensão do componente.
const SPACING: (f64, f64) = (5., 100.);

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
    #[test]
    fn importing_from_another_document_copies_definitions_independently()
    -> Result<(), Box<dyn Error>> {
        // Origem: conjunto com duas instâncias da mesma subpeça, agrupadas.
        let mut source = Document::new("Origem")?;
        let assembly = source.create_part("Conjunto".into())?;
        let (child, body) = source.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let first = source.instances()[1].id;
        source.reparent(first, Some(assembly))?;
        let second = source.add_instance(
            child,
            Some(assembly),
            Placement::new([80., 0., 0.], [0.; 3])?,
        )?;
        source.add_group("Par".into(), vec![first, second])?;
        // Destino com identificadores que colidiriam com os da origem.
        let mut target = Document::new("Destino")?;
        target.create_part("Existente".into())?;
        let imported = target.import_part(&source, assembly)?;
        target.add_instance(imported, None, Placement::default())?;
        target.validate()?;
        assert_eq!(target.parts().len(), 3);
        let children = target
            .instances()
            .iter()
            .filter(|i| i.parent == Some(imported))
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 2);
        // As duas ocorrências continuam compartilhando uma única definição copiada.
        assert_eq!(children[0].part, children[1].part);
        assert_ne!(children[0].part, child);
        assert_eq!(target.groups().len(), 1);
        // A origem não muda; alterar a cópia não afeta a definição original.
        let copied = children[0].part;
        target.part_mut(copied)?.bodies[0].name = "Somente na cópia".into();
        assert_eq!(source.body(body).ok_or("Corpo ausente")?.name, "Suporte");
        assert_eq!(source.parts().len(), 2);
        Ok(())
    }

    #[test]
    fn picking_lists_every_body_along_the_ray_nearest_first() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Sobrepostos")?;
        let (part, _) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let far = doc.add_instance(part, None, Placement::new([0., 0., -100.], [0.; 3])?)?;
        let near = doc.instances()[0].id;
        // Raio de cima para baixo sobre a base do suporte (60 × 25 mm).
        let ray = nator_geometry::query::Ray {
            origin: [30., 12., 500.],
            direction: [0., 0., -1.],
        };
        let hits = doc.pick_all_with(ray, |_| true)?;
        assert_eq!(
            hits.iter().map(|h| h.instance).collect::<Vec<_>>(),
            [near, far]
        );
        assert!(hits[0].point[2] > hits[1].point[2]);
        assert_eq!(doc.pick(ray)?.map(|p| p.instance), Some(near));
        // Ocorrências recusadas (bloqueadas) não entram na lista.
        let only_far = doc.pick_all_with(ray, |o| o.instance != near)?;
        assert_eq!(only_far.len(), 1);
        Ok(())
    }

    #[test]
    fn folder_visibility_combines_with_each_element() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Pastas")?;
        let bracket = || -> Result<Arc<MeshAsset>, Box<dyn Error>> {
            Ok(Arc::new(MeshAsset::new(
                nator_geometry::reference_bracket()?
            )))
        };
        let (part, body) = doc.add_mesh(None, "Suporte".into(), bracket()?)?;
        let (inner, _) = doc.add_mesh(None, "Interna".into(), bracket()?)?;
        let nested = doc
            .instances()
            .iter()
            .find(|i| i.part == inner)
            .ok_or("Instância ausente")?
            .id;
        doc.reparent(nested, Some(part))?;
        let triangles = |doc: &Document| -> Result<usize, Box<dyn Error>> {
            Ok(doc
                .scene()?
                .map(|scene| scene.mesh.triangles().len())
                .unwrap_or(0))
        };
        let full = triangles(&doc)?;
        assert_eq!(doc.occurrences()?.len(), 2);
        // Origem das peças oculta por padrão; as demais pastas, visíveis.
        assert!(!doc.folder_visible(Folder::PartOrigin(part))?);
        assert!(doc.shows_origin());
        // Subpeças ocultas: a ocorrência interna sai da cena, sem mudar a instância.
        doc.set_folder_visible(Folder::Subparts(part), false)?;
        assert_eq!(doc.occurrences()?.len(), 1);
        assert_eq!(triangles(&doc)? * 2, full);
        assert!(doc.instance(nested).ok_or("Instância ausente")?.visible);
        // Corpos ocultos: a escolha de cada corpo (que o histórico usa) é preservada.
        doc.set_folder_visible(Folder::Bodies(part), false)?;
        assert_eq!(triangles(&doc)?, 0);
        assert!(doc.body(body).ok_or("Corpo ausente")?.visible);
        doc.set_folder_visible(Folder::Subparts(part), true)?;
        doc.set_folder_visible(Folder::Bodies(part), true)?;
        assert_eq!(triangles(&doc)?, full);
        // A pasta Peças controla as ocorrências da raiz; o projeto permanece visível.
        doc.set_folder_visible(Folder::Parts, false)?;
        assert!(doc.occurrences()?.is_empty());
        doc.set_folder_visible(Folder::Parts, true)?;
        let mut layout = nator_sketch::Sketch::new("Layout", nator_sketch::Plane::xy());
        layout.add_point([0., 0.])?;
        let reference = doc.add_reference_sketch(layout)?;
        let record = doc.sketch(reference).ok_or("Rascunho ausente")?.clone();
        assert!(doc.shows_reference_sketch(&record));
        assert!(doc.set_folder_visible(Folder::Project, false).is_err());
        assert!(!doc.occurrences()?.is_empty());
        assert!(doc.shows_origin() && doc.shows_reference_sketch(&record));
        doc.restore_folders(ProjectFolders {
            project: false,
            sketches: false,
            ..doc.folders()
        });
        assert!(doc.folder_visible(Folder::Project)?);
        assert!(!doc.folder_visible(Folder::ReferenceSketches)?);
        assert!(doc.folder_visible(Folder::Parts)?);
        Ok(())
    }

    #[test]
    fn new_components_slide_beside_the_existing_ones() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Lado a lado")?;
        let (part, _) = doc.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?)),
        )?;
        let first = doc.instances()[0].id;
        let world = |doc: &Document, id: InstanceId| -> Result<Bounds, Box<dyn Error>> {
            doc.occurrence_boxes()?
                .into_iter()
                .find(|(o, _)| o.instance == id)
                .and_then(|(_, b)| b)
                .ok_or_else(|| "Caixa ausente".into())
        };
        let original = world(&doc, first)?;
        let gap = (0.1 * original.size().into_iter().fold(0_f64, f64::max)).clamp(5., 100.);
        // Cópia na mesma posição: vai para a direita, com a folga, sem mudar Y nem Z.
        let second = doc.add_instance(part, None, Placement::default())?;
        doc.place_beside(&[second])?;
        let placed = world(&doc, second)?;
        assert!((placed.min[0] - (original.max[0] + gap)).abs() < 1e-9);
        assert_eq!(
            (placed.min[1], placed.min[2]),
            (original.min[1], original.min[2])
        );
        // A terceira passa pelas duas.
        let third = doc.add_instance(part, None, Placement::default())?;
        doc.place_beside(&[third])?;
        assert!((world(&doc, third)?.min[0] - (placed.max[0] + gap)).abs() < 1e-9);
        // Sem sobreposição, nada muda.
        let away = Placement::new([0., 500., 0.], [0.; 3])?;
        let distant = doc.add_instance(part, None, away)?;
        doc.place_beside(&[distant])?;
        assert_eq!(
            doc.instance(distant)
                .ok_or("Instância ausente")?
                .placement
                .translation_mm(),
            [0., 500., 0.]
        );
        // Peça vazia: a origem fica à direita de tudo, na altura da origem.
        let empty = doc.create_part("Vazia".into())?;
        let instance = doc
            .instances()
            .iter()
            .find(|i| i.part == empty)
            .ok_or("Instância ausente")?
            .id;
        doc.place_beside(&[instance])?;
        let origin = doc
            .instance(instance)
            .ok_or("Instância ausente")?
            .placement
            .translation_mm();
        assert!(origin[0] > world(&doc, third)?.max[0]);
        assert_eq!((origin[1], origin[2]), (0., 0.));
        // Subpeça de um pai girado: o deslocamento é aplicado no mundo, em +X.
        let parent = doc.create_part("Conjunto".into())?;
        let holder = doc
            .instances()
            .iter()
            .find(|i| i.part == parent)
            .ok_or("Instância ausente")?
            .id;
        doc.instance_mut(holder)?.placement =
            Placement::new([0., -400., 0.], [0., 0., std::f64::consts::FRAC_PI_2])?;
        let inner = doc.add_instance(part, Some(parent), Placement::default())?;
        let sibling = doc.add_instance(part, Some(parent), Placement::default())?;
        let before = world(&doc, sibling)?;
        doc.place_beside(&[sibling])?;
        let after = world(&doc, sibling)?;
        assert!(after.min[0] > world(&doc, inner)?.max[0]);
        assert!((after.min[1] - before.min[1]).abs() < 1e-9);
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
        Ok(self.pick_all_with(ray, allowed)?.into_iter().next())
    }
    /// Todos os corpos atravessados pelo raio, do mais próximo ao mais distante, com o primeiro
    /// ponto de cada corpo em cada ocorrência. Serve à escolha entre elementos sobrepostos.
    pub fn pick_all_with(
        &self,
        ray: nator_geometry::query::Ray,
        allowed: impl Fn(&Occurrence) -> bool,
    ) -> Result<Vec<Pick>, DocumentError> {
        let mut hits = Vec::new();
        for occurrence in self.occurrences()?.into_iter().filter(|o| allowed(o)) {
            let instance = self
                .instance(occurrence.instance)
                .ok_or_else(|| DocumentError("Instância inexistente.".into()))?;
            let part = self
                .part(instance.part)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
            for body in part.shown_bodies() {
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
                if let Some(hit) = body.geometry.mesh().ray_hit(local) {
                    let zero = pose.apply([0.; 3]);
                    let point = pose.apply(hit.point);
                    // A distância ao longo do raio do mundo ordena os corpos de todas as poses.
                    let distance =
                        nator_geometry::dot(nator_geometry::sub(point, ray.origin), ray.direction)
                            / nator_geometry::dot(ray.direction, ray.direction)
                                .max(f64::MIN_POSITIVE);
                    hits.push((
                        distance,
                        Pick {
                            instance: instance.id,
                            part: part.id,
                            body: body.id,
                            triangle: hit.triangle,
                            point,
                            normal: nator_geometry::sub(pose.apply(hit.normal), zero),
                            vertices: body
                                .geometry
                                .mesh()
                                .triangle_points(body.geometry.mesh().triangles()[hit.triangle])
                                .map(|p| pose.apply(p)),
                            placement: occurrence.placement,
                        },
                    ));
                }
            }
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(hits.into_iter().map(|(_, pick)| pick).collect())
    }
}
