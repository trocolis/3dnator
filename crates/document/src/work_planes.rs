//! Planos de trabalho da peça: suportes auxiliares para novos rascunhos, obtidos ao
//! deslocar um plano-base (da origem, de outro plano de trabalho ou de uma face plana) ao
//! longo da sua normal. Como os rascunhos criados sobre faces, o plano-base é uma captura
//! fixa em coordenadas da peça, sem vínculo associativo com a geometria de onde veio.
//! Já os rascunhos criados num plano de trabalho ficam apoiados nele (`SketchRecord::support`)
//! e acompanham as edições do plano.
use super::{Document, DocumentError, Part, PartId, SketchId, WorkPlaneId, validate_name};
use nator_sketch::Plane;

/// Limite simétrico do deslocamento (mm), o mesmo aceito pelos campos numéricos da
/// interface. Valores fora da faixa são recusados, sem truncamento.
pub const MAX_OFFSET: f64 = 1e7;

/// Definição geométrica do plano de trabalho, em milímetros e coordenadas da peça.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkPlane {
    /// Plano de partida. Os eixos U/V são conservados no plano deslocado, para que um
    /// rascunho nele tenha a mesma orientação de um rascunho no suporte original.
    pub base: Plane,
    /// Distância ao longo da normal do plano-base (U × V); negativa vai no sentido oposto.
    pub offset: f64,
}
impl WorkPlane {
    /// Plano resultante: a origem anda `offset` ao longo da normal; U e V não mudam.
    pub fn plane(&self) -> Plane {
        let normal = self.base.normal();
        Plane {
            origin: std::array::from_fn(|i| self.base.origin[i] + normal[i] * self.offset),
            ..self.base
        }
    }
    pub fn validate(&self) -> Result<(), DocumentError> {
        self.base
            .validate()
            .map_err(|e| DocumentError(e.to_string()))?;
        if !self.offset.is_finite() || self.offset.abs() > MAX_OFFSET {
            return Err(DocumentError(
                "Deslocamento do plano inválido ou fora do limite.".into(),
            ));
        }
        self.plane()
            .validate()
            .map_err(|e| DocumentError(e.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkPlaneRecord {
    pub id: WorkPlaneId,
    pub name: String,
    pub plane: WorkPlane,
    /// Visibilidade própria; a efetiva combina com a da pasta Planos da peça.
    pub visible: bool,
}

impl Part {
    /// Plano exibido: visível e com a pasta Planos visível.
    pub fn shows_work_plane(&self, record: &WorkPlaneRecord) -> bool {
        self.folders.planes && record.visible
    }
}

impl Document {
    pub fn work_plane(&self, id: WorkPlaneId) -> Option<&WorkPlaneRecord> {
        self.parts
            .iter()
            .flat_map(|p| &p.work_planes)
            .find(|w| w.id == id)
    }
    pub fn work_plane_mut(
        &mut self,
        id: WorkPlaneId,
    ) -> Result<&mut WorkPlaneRecord, DocumentError> {
        self.parts
            .iter_mut()
            .flat_map(|p| &mut p.work_planes)
            .find(|w| w.id == id)
            .ok_or_else(|| DocumentError("Plano de trabalho inexistente.".into()))
    }
    /// Peça dona do plano de trabalho.
    pub fn work_plane_part(&self, id: WorkPlaneId) -> Option<PartId> {
        self.parts
            .iter()
            .find(|p| p.work_planes.iter().any(|w| w.id == id))
            .map(|p| p.id)
    }
    /// Acrescenta um plano visível à peça.
    pub fn add_work_plane(
        &mut self,
        part: PartId,
        name: String,
        plane: WorkPlane,
    ) -> Result<WorkPlaneId, DocumentError> {
        validate_name(&name)?;
        plane.validate()?;
        self.part_mut(part)?;
        self.record_creation(super::ElementKind::WorkPlane)?;
        let id = WorkPlaneId(self.allocate_id()?);
        self.part_mut(part)?.work_planes.push(WorkPlaneRecord {
            id,
            name,
            plane,
            visible: true,
        });
        Ok(id)
    }
    /// Rascunho vazio apoiado no plano, na peça dona dele. O plano é ocultado: usado,
    /// sai da vista como o perfil consumido por uma operação.
    pub fn add_sketch_on_work_plane(
        &mut self,
        id: WorkPlaneId,
        name: String,
    ) -> Result<(PartId, SketchId), DocumentError> {
        let part = self
            .work_plane_part(id)
            .ok_or_else(|| DocumentError("Plano de trabalho inexistente.".into()))?;
        let record = self.work_plane_mut(id)?;
        record.visible = false;
        let plane = record.plane.plane();
        let (part, sketch) = self.add_sketch(Some(part), nator_sketch::Sketch::new(name, plane))?;
        self.sketch_mut(sketch)?.support = Some(id);
        Ok((part, sketch))
    }
    /// Troca a definição do plano e leva junto os rascunhos apoiados nele, conservando a
    /// geometria local (coordenadas U/V) de cada um. Devolve a peça e se algum rascunho
    /// mudou, caso em que o histórico da peça precisa ser recalculado.
    pub fn update_work_plane(
        &mut self,
        id: WorkPlaneId,
        plane: WorkPlane,
    ) -> Result<(PartId, bool), DocumentError> {
        plane.validate()?;
        let part = self
            .work_plane_part(id)
            .ok_or_else(|| DocumentError("Plano de trabalho inexistente.".into()))?;
        self.work_plane_mut(id)?.plane = plane;
        let support = plane.plane();
        let mut moved = false;
        for record in self
            .part_mut(part)?
            .sketches
            .iter_mut()
            .filter(|s| s.support == Some(id))
        {
            record.sketch.plane = support;
            moved = true;
        }
        Ok((part, moved))
    }
    /// Exclui o plano. Os rascunhos apoiados nele ficam onde estão, com o plano capturado
    /// como suporte fixo, e deixam de acompanhá-lo.
    pub fn remove_work_plane(&mut self, id: WorkPlaneId) -> Result<(), DocumentError> {
        let part = self
            .parts
            .iter_mut()
            .find(|p| p.work_planes.iter().any(|w| w.id == id))
            .ok_or_else(|| DocumentError("Plano de trabalho inexistente.".into()))?;
        part.work_planes.retain(|w| w.id != id);
        for record in part.sketches.iter_mut().filter(|s| s.support == Some(id)) {
            record.support = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Folder;
    use std::error::Error;

    #[test]
    fn offset_moves_along_the_base_normal_and_keeps_its_axes() -> Result<(), Box<dyn Error>> {
        let xz = WorkPlane {
            base: Plane::xz(),
            offset: 12.5,
        };
        let plane = xz.plane();
        // A normal de XZ é X × Z = -Y: o deslocamento positivo vai para Y negativo.
        assert_eq!(plane.origin, [0., -12.5, 0.]);
        assert_eq!((plane.u, plane.v), (Plane::xz().u, Plane::xz().v));
        let face = WorkPlane {
            base: Plane::from_normal([3., 4., 5.], [0., 0., 2.])?,
            offset: -5.,
        };
        assert_eq!(face.plane().origin, [3., 4., 0.]);
        for invalid in [f64::NAN, f64::INFINITY, MAX_OFFSET * 2.] {
            assert!(
                WorkPlane {
                    base: Plane::xy(),
                    offset: invalid,
                }
                .validate()
                .is_err()
            );
        }
        let skewed = Plane {
            u: [1., 1., 0.],
            ..Plane::xy()
        };
        assert!(
            WorkPlane {
                base: skewed,
                offset: 0.,
            }
            .validate()
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn planes_belong_to_their_part_and_follow_the_folder() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Planos")?;
        let part = doc.create_part("Suporte".into())?;
        let plane = WorkPlane {
            base: Plane::xy(),
            offset: 20.,
        };
        assert!(
            doc.add_work_plane(PartId(999), "Plano".into(), plane)
                .is_err()
        );
        assert!(doc.add_work_plane(part, String::new(), plane).is_err());
        let id = doc.add_work_plane(part, "Plano 1".into(), plane)?;
        doc.validate()?;
        assert_eq!(doc.work_plane_part(id), Some(part));
        let record = doc.work_plane(id).ok_or("Plano ausente")?.clone();
        let piece = doc.part(part).ok_or("Peça ausente")?;
        assert!(piece.shows_work_plane(&record));
        doc.set_folder_visible(Folder::WorkPlanes(part), false)?;
        let piece = doc.part(part).ok_or("Peça ausente")?;
        assert!(!piece.shows_work_plane(&record));
        // A escolha de cada plano é preservada ao ocultar e reexibir a pasta.
        assert!(doc.work_plane(id).is_some_and(|w| w.visible));
        doc.remove_work_plane(id)?;
        assert!(doc.work_plane(id).is_none());
        assert!(doc.remove_work_plane(id).is_err());
        Ok(())
    }

    #[test]
    fn supported_sketches_follow_edits_until_the_plane_is_removed() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Rascunho apoiado")?;
        let part = doc.create_part("Suporte".into())?;
        let base = Plane::from_normal([0., 5., 0.], [0., 1., 0.])?;
        let plane = doc.add_work_plane(part, "Plano 1".into(), WorkPlane { base, offset: 4. })?;
        let (owner, sketch) = doc.add_sketch_on_work_plane(plane, "Perfil".into())?;
        assert_eq!(owner, part);
        assert!(doc.work_plane(plane).is_some_and(|w| !w.visible));
        doc.sketch_mut(sketch)?.sketch.add_point([3., -2.])?;
        let before = doc.sketch(sketch).ok_or("Rascunho ausente")?.sketch.clone();
        assert_eq!(before.plane.origin, [0., 9., 0.]);
        // Um rascunho na origem, sem apoio, não acompanha o plano.
        let (_, free) = doc.add_sketch(Some(part), nator_sketch::Sketch::new("Livre", base))?;
        assert_eq!(
            doc.update_work_plane(plane, WorkPlane { base, offset: -6. })?,
            (part, true)
        );
        doc.validate()?;
        let after = &doc.sketch(sketch).ok_or("Rascunho ausente")?.sketch;
        assert_eq!(after.plane.origin, [0., -1., 0.]);
        assert_eq!((after.plane.u, after.plane.v), (base.u, base.v));
        // A geometria local é preservada: o ponto anda junto com o plano.
        assert_eq!(after.points[0].position, before.points[0].position);
        assert_eq!(doc.sketch(free).ok_or("Livre ausente")?.sketch.plane, base);
        assert!(
            doc.update_work_plane(
                plane,
                WorkPlane {
                    base,
                    offset: f64::NAN,
                },
            )
            .is_err()
        );
        doc.remove_work_plane(plane)?;
        doc.validate()?;
        let record = doc.sketch(sketch).ok_or("Rascunho ausente")?;
        assert_eq!(record.support, None);
        assert_eq!(record.sketch.plane.origin, [0., -1., 0.]);
        Ok(())
    }

    #[test]
    fn cloned_parts_receive_new_plane_identifiers() -> Result<(), Box<dyn Error>> {
        let mut doc = Document::new("Clone")?;
        let part = doc.create_part("Original".into())?;
        let plane = WorkPlane {
            base: Plane::yz(),
            offset: -3.,
        };
        let id = doc.add_work_plane(part, "Plano 1".into(), plane)?;
        doc.add_sketch_on_work_plane(id, "Perfil".into())?;
        let copy = doc.clone_part(part, "Cópia".into())?;
        doc.validate()?;
        let piece = doc.part(copy).ok_or("Cópia ausente")?;
        let cloned = &piece.work_planes;
        assert_eq!(cloned.len(), 1);
        assert_ne!(cloned[0].id, id);
        assert_eq!(cloned[0].plane, plane);
        assert_eq!(doc.work_plane_part(cloned[0].id), Some(copy));
        // O rascunho copiado acompanha o plano copiado, não o original.
        assert_eq!(piece.sketches[0].support, Some(cloned[0].id));
        Ok(())
    }
}
