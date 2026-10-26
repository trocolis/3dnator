//! Referências associativas das folhas. Caminhos são relativos à definição nas folhas
//! de peça e completos nas folhas de montagem; não capturam uma posição rígida antiga.
use super::{BodyId, DocumentError, InstanceId};
use crate::features::{EdgeRef, FaceRef, validate_face_name};
use nator_drawing::{AnnotationId, View};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingBodyRef {
    pub body: BodyId,
    pub path: Vec<InstanceId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingFaceRef {
    pub target: DrawingBodyRef,
    pub face: FaceRef,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingEdgeRef {
    pub target: DrawingBodyRef,
    pub edge: EdgeRef,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrawingTopologyRef {
    Face(DrawingFaceRef),
    Edge(DrawingEdgeRef),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrawingMeasure {
    EdgeLength(DrawingEdgeRef),
    Radius {
        topology: DrawingTopologyRef,
        diameter: bool,
    },
    FaceDistance {
        first: DrawingFaceRef,
        second: DrawingFaceRef,
    },
    FaceAngle {
        first: DrawingFaceRef,
        second: DrawingFaceRef,
        supplementary: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingAnnotation {
    pub id: AnnotationId,
    pub view: View,
    pub measurement: DrawingMeasure,
    /// Deslocamento do texto em mm no papel, independente da escala da geometria.
    pub offset_mm: [f64; 2],
}
pub fn validate(annotations: &[DrawingAnnotation], next_id: u64) -> Result<(), DocumentError> {
    fn body(target: &DrawingBodyRef, next: u64) -> Result<(), DocumentError> {
        if target.body.0 == 0
            || target.body.0 >= next
            || target.path.len() > 64
            || target.path.iter().any(|id| id.0 == 0 || id.0 >= next)
        {
            return Err(DocumentError(
                "Referência de corpo ou caminho da cota inválida.".into(),
            ));
        }
        Ok(())
    }
    fn face(reference: &DrawingFaceRef, next: u64) -> Result<(), DocumentError> {
        body(&reference.target, next)?;
        let FaceRef::Named(name) = &reference.face else {
            return Err(DocumentError(
                "Cotas exigem faces com nomes estáveis.".into(),
            ));
        };
        validate_face_name(name)
    }
    fn edge(reference: &DrawingEdgeRef, next: u64) -> Result<(), DocumentError> {
        body(&reference.target, next)?;
        let EdgeRef::Between(names) = &reference.edge else {
            return Err(DocumentError(
                "Cotas exigem arestas com nomes estáveis.".into(),
            ));
        };
        for name in names {
            validate_face_name(name)?;
        }
        Ok(())
    }
    if annotations.len() > 256 {
        return Err(DocumentError(
            "A folha aceita até 256 cotas associativas.".into(),
        ));
    }
    for (index, a) in annotations.iter().enumerate() {
        if a.id.0 == 0
            || annotations[..index].iter().any(|b| b.id == a.id)
            || a.offset_mm
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1000.)
        {
            return Err(DocumentError(
                "Identificador ou posição de cota inválidos.".into(),
            ));
        }
        match &a.measurement {
            DrawingMeasure::EdgeLength(r) => edge(r, next_id)?,
            DrawingMeasure::Radius { topology, .. } => match topology {
                DrawingTopologyRef::Face(r) => face(r, next_id)?,
                DrawingTopologyRef::Edge(r) => edge(r, next_id)?,
            },
            DrawingMeasure::FaceDistance { first, second }
            | DrawingMeasure::FaceAngle { first, second, .. } => {
                face(first, next_id)?;
                face(second, next_id)?;
            }
        }
    }
    // A geometria pode deixar de existir depois de uma edição. Essa situação mantém
    // a anotação no arquivo e é mostrada como revisão pendente ao regenerar a folha.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::FaceRef;
    use nator_drawing::{AnnotationId, View};

    #[test]
    fn validates_annotations_rejecting_invalid_ids_counts_and_coordinates() {
        let body = DrawingBodyRef {
            body: BodyId(1),
            path: Vec::new(),
        };
        let face = DrawingFaceRef {
            target: body.clone(),
            face: FaceRef::Named("b1:f0".into()),
        };
        let valid = DrawingAnnotation {
            id: AnnotationId(1),
            view: View::Front,
            measurement: DrawingMeasure::FaceDistance {
                first: face.clone(),
                second: DrawingFaceRef {
                    target: body.clone(),
                    face: FaceRef::Named("b1:f1".into()),
                },
            },
            offset_mm: [10., 20.],
        };
        assert!(validate(std::slice::from_ref(&valid), 10).is_ok());

        // id == 0
        let mut invalid = valid.clone();
        invalid.id = AnnotationId(0);
        assert!(validate(&[invalid], 10).is_err());

        // duplicate id
        assert!(validate(&[valid.clone(), valid.clone()], 10).is_err());

        // non-finite offset
        let mut invalid = valid.clone();
        invalid.offset_mm = [f64::NAN, 0.];
        assert!(validate(&[invalid], 10).is_err());

        // offset > 1000
        let mut invalid = valid.clone();
        invalid.offset_mm = [1001., 0.];
        assert!(validate(&[invalid], 10).is_err());

        // body id >= next
        let mut invalid = valid.clone();
        if let DrawingMeasure::FaceDistance { first, .. } = &mut invalid.measurement {
            first.target.body = BodyId(10);
        }
        assert!(validate(&[invalid], 10).is_err());

        // invalid face name (empty)
        let mut invalid = valid.clone();
        if let DrawingMeasure::FaceDistance { first, .. } = &mut invalid.measurement {
            first.face = FaceRef::Named("".into());
        }
        assert!(validate(&[invalid], 10).is_err());
    }
}
