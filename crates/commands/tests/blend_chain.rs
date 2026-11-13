//! Cadeia de concordância do Remover faces, sobre a topologia de um corpo STEP real (o mancal
//! de um projeto de impressora): um anel de arredondamentos de 0,99 mm em volta de uma face
//! plana, com remendos de canto B-spline, encostado em concordâncias de outros raios.
use nator_commands::features::{BlendShape, Necessary, RemovalPlan, blend_chain, necessary_faces};
use nator_document::features::FaceRef;
use std::collections::BTreeSet;

#[test]
fn the_ring_around_a_boss_is_one_blend_and_other_radii_stay_out() {
    use BlendShape::{Free, Rigid, Round};
    let mut shapes = vec![Rigid; 140];
    // Anel: cilindros e toros de 0,99 mm alternados com remendos livres pequenos.
    for (face, shape) in [
        (104, Free(2.13)),
        (105, Round(0.99)),
        (106, Free(2.13)),
        (107, Round(0.99)),
        (108, Free(2.13)),
        (109, Round(0.99)),
        (110, Free(2.13)),
        (111, Round(0.99)),
        // Vizinhas tangentes de outra concordância: arredondamentos de 1 mm, um furo de
        // 2,7 mm e uma superfície livre grande.
        (101, Round(1.0)),
        (103, Round(1.0)),
        (102, Round(2.7)),
        (129, Round(2.7)),
        (26, Free(300.)),
        // Arredondamento vertical de 1 mm que chega ao remendo de canto 106.
        (27, Round(1.0)),
    ] {
        shapes[face] = shape;
    }
    let pairs = [
        [99, 104],
        [99, 105],
        [99, 106],
        [99, 107],
        [99, 108],
        [99, 109],
        [99, 110],
        [99, 111],
        [104, 111],
        [104, 105],
        [105, 106],
        [106, 107],
        [107, 108],
        [108, 109],
        [109, 110],
        [110, 111],
        [110, 101],
        [108, 103],
        [109, 102],
        [105, 129],
        [104, 26],
        [100, 111],
        [101, 102],
        [106, 27],
    ];
    let ring = (104..=111).collect::<BTreeSet<_>>();
    // De qualquer face do anel (concordância ou remendo), sai o anel inteiro. O remendo 106
    // também encosta no arredondamento vertical de 1 mm, mas o raio mais frequente em volta
    // dele é o do anel.
    for start in [111, 104, 106, 109] {
        assert_eq!(
            blend_chain(&BTreeSet::from([start]), &pairs, &shapes),
            ring,
            "{start}"
        );
    }
    // Uma face plana não é concordância.
    assert_eq!(
        blend_chain(&BTreeSet::from([99]), &pairs, &shapes),
        BTreeSet::from([99])
    );
}

/// Sondagem simulada de um anel de oito faces: a remoção só funciona com o anel inteiro
/// menos, no máximo, a face 7 (o núcleo estende a vizinha e fecha o vão). A busca começa da
/// face 0 e deve sugerir exatamente as faces 1 a 6, também com as sondagens em paralelo.
#[test]
fn the_search_keeps_only_the_faces_the_removal_needs() -> Result<(), &'static str> {
    let plan = RemovalPlan {
        bytes: std::sync::Arc::new(Vec::new()),
        signature: 9,
        chosen: vec![0],
        layers: vec![vec![1, 7], vec![2, 6], vec![3, 5], vec![4], vec![20, 21]],
    };
    let works = |faces: &[usize], _: std::time::Duration| Some((0..7).all(|f| faces.contains(&f)));
    for parallel in [1, 4] {
        let Necessary::With(found) = necessary_faces(&plan, &works, &|| false, parallel) else {
            return Err("Faces necessárias não encontradas");
        };
        let mut found = found
            .iter()
            .filter_map(|f| match f {
                FaceRef::Index {
                    index,
                    signature: 9,
                } => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>();
        found.sort_unstable();
        assert_eq!(found, vec![1, 2, 3, 4, 5, 6]);
    }
    // Uma seleção que sai sozinha não sugere nada; uma que nunca sai, também não.
    assert_eq!(
        necessary_faces(&plan, &|_, _| Some(true), &|| false, 2),
        Necessary::Alone
    );
    assert_eq!(
        necessary_faces(&plan, &|_, _| Some(false), &|| false, 2),
        Necessary::NotFound
    );
    Ok(())
}
