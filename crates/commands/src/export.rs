//! Planejamento da exportação STL para impressão 3D, sem gravar nada.
//!
//! Regras do README:
//! - a unidade padrão é o corpo: um arquivo por corpo, em coordenadas locais da peça;
//! - a exportação conjunta gera um único arquivo com as posições de montagem, sem união
//!   booleana entre os corpos;
//! - cada corpo de uma definição compartilhada é exportado uma vez, informando quantas
//!   ocorrências ele tem; opcionalmente, todas as ocorrências nas posições de montagem;
//! - para sólidos CAD, a malha vem da geometria precisa com a qualidade escolhida, nunca
//!   da malha de exibição; corpos de malha são exportados como estão;
//! - nomes de arquivo combinam peça e corpo e nunca repetem um nome do mesmo plano.
use nator_document::{BodyId, Document, DocumentError, PartId};
use nator_geometry::{Bounds, Mesh, Placement};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub enum StepScope {
    Body(BodyId),
    Part(PartId),
    Assembly,
}

/// Constrói o intercâmbio sobre B-reps precisos. Malhas visíveis geram erro explícito;
/// omiti-las ou convertê-las silenciosamente mudaria o conteúdo exportado.
#[cfg(feature = "occt")]
pub fn step_scene(
    doc: &Document,
    scope: StepScope,
) -> Result<nator_kernel::StepAssembly, DocumentError> {
    use nator_kernel::{Solid, StepAssembly, StepBody, StepOccurrence, StepPart};
    let error = |e: nator_kernel::KernelError| DocumentError(e.to_string());
    let body_data = |body: &nator_document::Body| -> Result<StepBody, DocumentError> {
        let brep = body.brep.as_ref().ok_or_else(|| DocumentError("O escopo contém corpos de malha. Use STL ou selecione apenas sólidos CAD para exportar STEP.".into()))?;
        Ok(StepBody {
            name: body.name.clone(),
            solid: Solid::from_brep(brep, None)
                .and_then(|s| s.transformed(body.placement))
                .map_err(error)?,
            color: body.color,
        })
    };
    if let StepScope::Body(id) = scope {
        let body = doc
            .body(id)
            .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
        return Ok(StepAssembly {
            parts: vec![StepPart {
                name: body.name.clone(),
                bodies: vec![body_data(body)?],
            }],
            instances: vec![StepOccurrence {
                part: 0,
                parent: None,
                name: body.name.clone(),
                placement: Placement::default(),
                color: None,
            }],
        });
    }
    doc.validate()?;
    fn keep(
        doc: &Document,
        part: PartId,
        retained: &mut HashMap<PartId, bool>,
    ) -> Result<bool, DocumentError> {
        if let Some(found) = retained.get(&part) {
            return Ok(*found);
        }
        let piece = doc
            .part(part)
            .ok_or_else(|| DocumentError("Peça inexistente.".into()))?;
        let mut visible = piece.shown_bodies().next().is_some();
        for child in doc
            .instances()
            .iter()
            .filter(|i| i.parent == Some(part) && doc.shows_instance(i))
        {
            visible |= keep(doc, child.part, retained)?;
        }
        retained.insert(part, visible);
        Ok(visible)
    }
    let roots = match scope {
        StepScope::Part(part) => vec![(
            part,
            Placement::default(),
            doc.part(part)
                .ok_or_else(|| DocumentError("Peça inexistente.".into()))?
                .name
                .clone(),
            None,
        )],
        _ => doc
            .instances()
            .iter()
            .filter(|i| i.parent.is_none() && doc.shows_instance(i))
            .map(|i| (i.part, i.placement, i.name.clone(), i.color))
            .collect(),
    };
    let mut retained = HashMap::new();
    for (part, _, _, _) in &roots {
        keep(doc, *part, &mut retained)?;
    }
    let selected = doc
        .parts()
        .iter()
        .filter(|p| retained.get(&p.id) == Some(&true))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(DocumentError("O escopo não contém corpos visíveis.".into()));
    }
    let indices = selected
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i))
        .collect::<HashMap<_, _>>();
    let parts = selected
        .iter()
        .map(|p| {
            Ok(StepPart {
                name: p.name.clone(),
                bodies: p
                    .shown_bodies()
                    .map(body_data)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, DocumentError>>()?;
    let mut instances = Vec::new();
    for (part, placement, name, color) in roots {
        if let Some(index) = indices.get(&part) {
            instances.push(StepOccurrence {
                part: *index,
                parent: None,
                name,
                placement,
                color,
            });
        }
    }
    for i in doc.instances().iter().filter(|i| doc.shows_instance(i)) {
        if let Some(parent) = i.parent.and_then(|p| indices.get(&p))
            && let Some(part) = indices.get(&i.part)
        {
            instances.push(StepOccurrence {
                part: *part,
                parent: Some(*parent),
                name: i.name.clone(),
                placement: i.placement,
                color: i.color,
            });
        }
    }
    Ok(StepAssembly { parts, instances })
}

/// Tolerâncias de tesselação da exportação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Draft,
    Standard,
    High,
    /// Desvio linear (mm) e angular (rad) informados pelo usuário.
    Custom {
        linear_mm: f64,
        angular_rad: f64,
    },
}
impl Quality {
    /// Desvio linear em mm e angular em rad.
    pub fn tolerances(self) -> (f64, f64) {
        match self {
            Self::Draft => (0.1, 0.5),
            Self::Standard => (0.02, 0.25),
            Self::High => (0.005, 0.1),
            Self::Custom {
                linear_mm,
                angular_rad,
            } => (linear_mm, angular_rad),
        }
    }
}

/// O que exportar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    /// Um corpo, em coordenadas locais da peça.
    Body(BodyId),
    /// Cada corpo visível da peça em arquivo próprio, em coordenadas locais da peça.
    Part(PartId),
    /// Cada corpo visível de cada definição, uma vez, em coordenadas locais da peça.
    Definitions,
    /// Todos os corpos visíveis em um único arquivo, nas posições de montagem.
    Joined,
    /// Um arquivo por ocorrência de cada corpo, nas posições de montagem.
    AllOccurrences,
}

/// Arquivo planejado.
#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub mesh: Mesh,
    /// Quantas vezes a geometria aparece na montagem (1 para ocorrências e conjunto).
    pub occurrences: usize,
}

/// Resumo exibido antes de exportar.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    pub files: usize,
    pub triangles: usize,
    /// Tamanho exato dos STL binários: 84 bytes de cabeçalho mais 50 por triângulo.
    pub bytes: u64,
    pub bounds: Option<Bounds>,
}

pub fn summarize(files: &[File]) -> Summary {
    let triangles = files.iter().map(|f| f.mesh.triangles().len()).sum();
    let bounds = files.iter().map(|f| f.mesh.bounds()).reduce(|a, b| Bounds {
        min: std::array::from_fn(|i| a.min[i].min(b.min[i])),
        max: std::array::from_fn(|i| a.max[i].max(b.max[i])),
    });
    Summary {
        files: files.len(),
        triangles,
        bytes: files
            .iter()
            .map(|f| 84 + 50 * f.mesh.triangles().len() as u64)
            .sum(),
        bounds,
    }
}

/// Nome de arquivo seguro em Linux e Windows, sem separadores nem caracteres reservados.
fn file_stem(text: &str) -> String {
    let cleaned = text
        .chars()
        .map(|c| {
            if c.is_control() || "/\\<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "corpo".into()
    } else {
        trimmed.chars().take(120).collect()
    }
}

/// Garante nomes distintos dentro do plano, sem diferenciar maiúsculas (Windows).
fn unique(name: String, used: &mut HashSet<String>) -> String {
    let mut candidate = name.clone();
    let mut counter = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{name}_{counter}");
        counter += 1;
    }
    candidate
}

/// Malha de exportação de um corpo, em coordenadas locais da peça.
fn body_mesh(doc: &Document, body: BodyId, quality: Quality) -> Result<Mesh, DocumentError> {
    let record = doc
        .body(body)
        .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
    let (linear, angular) = quality.tolerances();
    if !(linear.is_finite() && angular.is_finite() && linear > 0. && angular > 0.) {
        return Err(DocumentError(
            "Tolerâncias de exportação devem ser positivas.".into(),
        ));
    }
    let local = match &record.brep {
        #[cfg(feature = "occt")]
        Some(bytes) => {
            let mut solid = nator_kernel::Solid::from_brep(bytes, None)
                .map_err(|e| DocumentError(e.to_string()))?;
            solid
                .tessellate(linear, angular)
                .map_err(|e| DocumentError(e.to_string()))?
        }
        _ => record.geometry.mesh().clone(),
    };
    transformed(&local, record.placement)
}

fn transformed(mesh: &Mesh, placement: Placement) -> Result<Mesh, DocumentError> {
    Mesh::new(
        mesh.vertices()
            .iter()
            .map(|p| placement.apply(*p))
            .collect(),
        mesh.triangles().to_vec(),
    )
    .map_err(|e| DocumentError(e.to_string()))
}

fn join(meshes: &[Mesh]) -> Result<Mesh, DocumentError> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for mesh in meshes {
        let base = u32::try_from(vertices.len())
            .map_err(|_| DocumentError("Malha conjunta grande demais.".into()))?;
        vertices.extend_from_slice(mesh.vertices());
        triangles.extend(mesh.triangles().iter().map(|t| t.map(|i| i + base)));
    }
    Mesh::new(vertices, triangles).map_err(|e| DocumentError(e.to_string()))
}

/// Planeja os arquivos de uma exportação. Nada é gravado.
pub fn plan(doc: &Document, scope: Scope, quality: Quality) -> Result<Vec<File>, DocumentError> {
    let mut used = HashSet::new();
    let mut files = Vec::new();
    let occurrences = doc.occurrences()?;
    // Ocorrências de cada definição na montagem (instâncias visíveis em toda a hierarquia).
    let mut count: HashMap<PartId, usize> = HashMap::new();
    for occurrence in &occurrences {
        if let Some(instance) = doc.instance(occurrence.instance) {
            *count.entry(instance.part).or_default() += 1;
        }
    }
    let name_of = |part: PartId, body: BodyId| -> String {
        let part = doc.part(part).map(|p| p.name.as_str()).unwrap_or("peça");
        let body = doc.body(body).map(|b| b.name.as_str()).unwrap_or("corpo");
        file_stem(&format!("{part}_{body}"))
    };
    let visible_bodies = |part: PartId| -> Vec<BodyId> {
        doc.part(part)
            .map(|p| p.shown_bodies().map(|b| b.id).collect())
            .unwrap_or_default()
    };
    match scope {
        Scope::Body(body) => {
            let part = doc
                .parts()
                .iter()
                .find(|p| p.bodies.iter().any(|b| b.id == body))
                .map(|p| p.id)
                .ok_or_else(|| DocumentError("Corpo inexistente.".into()))?;
            files.push(File {
                name: unique(name_of(part, body), &mut used),
                mesh: body_mesh(doc, body, quality)?,
                occurrences: count.get(&part).copied().unwrap_or(0).max(1),
            });
        }
        Scope::Part(part) => {
            for body in visible_bodies(part) {
                files.push(File {
                    name: unique(name_of(part, body), &mut used),
                    mesh: body_mesh(doc, body, quality)?,
                    occurrences: count.get(&part).copied().unwrap_or(0).max(1),
                });
            }
        }
        Scope::Definitions => {
            for part in doc.parts().iter().filter(|p| count.contains_key(&p.id)) {
                for body in visible_bodies(part.id) {
                    files.push(File {
                        name: unique(name_of(part.id, body), &mut used),
                        mesh: body_mesh(doc, body, quality)?,
                        occurrences: count[&part.id],
                    });
                }
            }
        }
        Scope::Joined | Scope::AllOccurrences => {
            // Cada definição é tesselada uma vez e reposicionada em cada ocorrência.
            let mut cache: HashMap<BodyId, Mesh> = HashMap::new();
            let mut placed = Vec::new();
            for occurrence in &occurrences {
                let Some(instance) = doc.instance(occurrence.instance) else {
                    continue;
                };
                for body in visible_bodies(instance.part) {
                    let local = match cache.get(&body) {
                        Some(mesh) => mesh.clone(),
                        None => {
                            let mesh = body_mesh(doc, body, quality)?;
                            cache.insert(body, mesh.clone());
                            mesh
                        }
                    };
                    placed.push((
                        instance.part,
                        body,
                        transformed(&local, occurrence.placement)?,
                    ));
                }
            }
            if scope == Scope::Joined {
                if placed.is_empty() {
                    return Err(DocumentError(
                        "Não há corpos visíveis para exportar.".into(),
                    ));
                }
                let meshes = placed.into_iter().map(|(_, _, m)| m).collect::<Vec<_>>();
                files.push(File {
                    name: unique(file_stem(doc.name()), &mut used),
                    mesh: join(&meshes)?,
                    occurrences: 1,
                });
            } else {
                for (part, body, mesh) in placed {
                    files.push(File {
                        name: unique(name_of(part, body), &mut used),
                        mesh,
                        occurrences: 1,
                    });
                }
            }
        }
    }
    if files.is_empty() {
        return Err(DocumentError(
            "Não há corpos visíveis para exportar.".into(),
        ));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_document::MeshAsset;
    use std::{error::Error, sync::Arc};

    fn assembly() -> Result<(Document, PartId), Box<dyn Error>> {
        let mut doc = Document::new("Montagem: teste/1")?;
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let (part, _) = doc.add_mesh(None, "Suporte".into(), Arc::clone(&asset))?;
        doc.add_mesh(Some(part), "Suporte".into(), asset)?;
        doc.add_instance(part, None, Placement::new([100., 0., 0.], [0.; 3])?)?;
        Ok((doc, part))
    }

    #[test]
    fn separate_files_use_local_coordinates_and_unique_names() -> Result<(), Box<dyn Error>> {
        let (doc, part) = assembly()?;
        let files = plan(&doc, Scope::Part(part), Quality::Standard)?;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "Suporte_Suporte");
        assert_eq!(files[1].name, "Suporte_Suporte_2");
        assert_eq!(files[0].occurrences, 2);
        assert_eq!(files[0].mesh.bounds().min, [0.; 3]);
        let definitions = plan(&doc, Scope::Definitions, Quality::Standard)?;
        assert_eq!(definitions.len(), 2);
        Ok(())
    }

    #[test]
    fn joined_and_occurrence_exports_use_assembly_positions() -> Result<(), Box<dyn Error>> {
        let (doc, _) = assembly()?;
        let joined = plan(&doc, Scope::Joined, Quality::Draft)?;
        assert_eq!(joined.len(), 1);
        // Nome do documento sem os caracteres reservados ':' e '/'.
        assert_eq!(joined[0].name, "Montagem_ teste_1");
        assert_eq!(joined[0].mesh.bounds().max[0], 160.);
        assert_eq!(joined[0].mesh.triangles().len(), 4 * 20);
        let all = plan(&doc, Scope::AllOccurrences, Quality::Draft)?;
        assert_eq!(all.len(), 4);
        let summary = summarize(&all);
        assert_eq!(summary.triangles, 80);
        assert_eq!(summary.bytes, 4 * 84 + 80 * 50);
        assert_eq!(summary.bounds.ok_or("Caixa ausente")?.max[0], 160.);
        assert!(
            plan(
                &doc,
                Scope::Joined,
                Quality::Custom {
                    linear_mm: 0.,
                    angular_rad: 0.1
                }
            )
            .is_err()
        );
        Ok(())
    }
}
