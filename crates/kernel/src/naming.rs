//! Nomes estáveis de faces e resolução de referências topológicas.
//!
//! Índices de faces/arestas mudam quando uma cota altera a topologia. Por isso cada face
//! recebe um nome derivado de sua origem:
//!
//! - lateral de extrusão/revolução: `{dono}:e{entidade}`, a entidade do rascunho que a gerou;
//! - tampas: `{dono}:start` e `{dono}:end`;
//! - face nova de arredondamento, chanfro ou borda de casca: `{dono}:{papel}`;
//! - face preservada ou recortada por uma operação: herda o nome da face de entrada;
//! - face importada (STEP) ou de primitiva: `{dono}:f{índice}`, estável porque essa
//!   geometria não é recalculada.
//!
//! `dono` identifica o corpo resultante da operação no documento (por exemplo `b12`).
//! Nome vazio indica face sem origem conhecida: ela não pode ser referenciada, e nenhuma
//! posição na lista é usada como substituta.
//!
//! Uma aresta é referenciada pelo par de nomes das faces adjacentes. Se o par não existir
//! mais, ou existir em mais de uma aresta, a referência é perdida ou ambígua e a operação
//! falha pedindo nova seleção: nunca outra aresta é escolhida silenciosamente.
use super::KernelError;

/// Tipos de origem gravados pela ponte C++ (`enum Origin` em bridge.cpp).
pub(crate) const SAME: i32 = 1;
pub(crate) const MODIFIED: i32 = 2;
pub(crate) const FROM_EDGE: i32 = 3;
pub(crate) const SIDE: i32 = 4;
pub(crate) const START_CAP: i32 = 5;
pub(crate) const END_CAP: i32 = 6;

/// Origem de uma face do resultado: (tipo, entrada, índice).
pub(crate) type Origin = [i32; 3];

/// Nomes das faces de uma varredura de perfil. `entities[i]` é a entidade do rascunho que
/// originou o i-ésimo segmento enviado à ponte.
pub(crate) fn sweep_names(owner: &str, origins: &[Origin], entities: &[u64]) -> Vec<String> {
    origins
        .iter()
        .map(|&[kind, _, index]| match kind {
            SIDE => usize::try_from(index)
                .ok()
                .and_then(|i| entities.get(i))
                .map(|entity| format!("{owner}:e{entity}"))
                .unwrap_or_default(),
            START_CAP => format!("{owner}:start"),
            END_CAP => format!("{owner}:end"),
            _ => String::new(),
        })
        .collect()
}

/// Como uma operação trata as faces modificadas de suas entradas.
pub(crate) struct Inheritance<'a> {
    /// Dono das faces novas geradas a partir de arestas.
    pub owner: &'a str,
    /// Papel das faces geradas a partir de arestas (`round`, `chamfer`, `rim`).
    pub generated: &'a str,
    /// Sufixo das faces modificadas. Na casca, a face interna é derivada da externa, que
    /// também permanece no resultado; o sufixo impede dois nomes iguais para faces distintas.
    pub modified_suffix: Option<&'a str>,
}

/// Nomes por herança: faces preservadas/recortadas mantêm o nome da entrada.
pub(crate) fn inherited_names(
    origins: &[Origin],
    inputs: &[&[String]],
    rule: &Inheritance<'_>,
) -> Vec<String> {
    let source = |input: i32, index: i32| -> Option<&String> {
        let names = inputs.get(usize::try_from(input).ok()?)?;
        names.get(usize::try_from(index).ok()?)
    };
    origins
        .iter()
        .map(|&[kind, input, index]| match kind {
            SAME => source(input, index).cloned().unwrap_or_default(),
            MODIFIED => source(input, index)
                .filter(|name| !name.is_empty())
                .map(|name| match rule.modified_suffix {
                    Some(suffix) => format!("{name}:{suffix}"),
                    None => name.clone(),
                })
                .unwrap_or_default(),
            FROM_EDGE => format!("{}:{}", rule.owner, rule.generated),
            _ => String::new(),
        })
        .collect()
}

/// Nomes posicionais para geometria que nunca é recalculada (STEP importado, primitivas).
pub(crate) fn indexed_names(owner: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{owner}:f{i}")).collect()
}

fn named(names: &[String], index: i32) -> Option<&str> {
    let name = names.get(usize::try_from(index).ok()?)?;
    (!name.is_empty()).then_some(name.as_str())
}

/// Nomes das faces adjacentes a uma aresta, para transformar uma seleção em referência.
pub(crate) fn edge_face_names(
    names: &[String],
    adjacency: &[[i32; 2]],
    edge: usize,
) -> Result<[String; 2], KernelError> {
    let [a, b] = adjacency
        .get(edge)
        .ok_or_else(|| KernelError("Aresta inexistente no sólido.".into()))?;
    match (named(names, *a), named(names, *b)) {
        (Some(a), Some(b)) => Ok([a.to_owned(), b.to_owned()]),
        _ => Err(KernelError(
            "A aresta toca uma face sem nome estável; ela não pode ser referenciada no histórico."
                .into(),
        )),
    }
}

/// Única aresta entre duas faces nomeadas.
pub(crate) fn edge_between(
    names: &[String],
    adjacency: &[[i32; 2]],
    faces: &[String; 2],
) -> Result<usize, KernelError> {
    let [a, b] = faces;
    if a.is_empty() || b.is_empty() {
        return Err(KernelError("Referência de aresta sem nome.".into()));
    }
    let matches = adjacency
        .iter()
        .enumerate()
        .filter(|(_, [x, y])| {
            let (x, y) = (named(names, *x), named(names, *y));
            (x == Some(a) && y == Some(b)) || (x == Some(b) && y == Some(a))
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [edge] => Ok(*edge),
        [] => Err(KernelError(format!(
            "Referência perdida: não existe mais aresta entre {a} e {b}. Selecione a aresta novamente."
        ))),
        many => Err(KernelError(format!(
            "Referência ambígua: {} arestas entre {a} e {b}. Selecione a aresta novamente.",
            many.len()
        ))),
    }
}

/// Única face com o nome dado.
pub(crate) fn face_named(names: &[String], face: &str) -> Result<usize, KernelError> {
    if face.is_empty() {
        return Err(KernelError("Referência de face sem nome.".into()));
    }
    let matches = names
        .iter()
        .enumerate()
        .filter(|(_, n)| n.as_str() == face)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(KernelError(format!(
            "Referência perdida: a face {face} não existe mais. Selecione a face novamente."
        ))),
        many => Err(KernelError(format!(
            "Referência ambígua: {} faces chamadas {face}. Selecione a face novamente.",
            many.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn sweep_faces_are_named_by_sketch_entities_and_caps() {
        let origins = [
            [SIDE, 0, 1],
            [START_CAP, 0, 0],
            [SIDE, 0, 0],
            [END_CAP, 0, 0],
            [0, 0, 0],
        ];
        let names = sweep_names("b7", &origins, &[40, 41]);
        assert_eq!(
            names,
            strings(&["b7:e41", "b7:start", "b7:e40", "b7:end", ""])
        );
    }

    #[test]
    fn inheritance_keeps_names_and_marks_new_faces() {
        let base = strings(&["b1:e3", "b1:start"]);
        let tool = strings(&["b2:e9"]);
        let origins = [
            [SAME, 0, 1],
            [MODIFIED, 1, 0],
            [FROM_EDGE, 0, 4],
            [MODIFIED, 0, 0],
        ];
        let plain = Inheritance {
            owner: "b5",
            generated: "round",
            modified_suffix: None,
        };
        assert_eq!(
            inherited_names(&origins, &[&base, &tool], &plain),
            strings(&["b1:start", "b2:e9", "b5:round", "b1:e3"])
        );
        let shell = Inheritance {
            modified_suffix: Some("inner"),
            ..plain
        };
        assert_eq!(
            inherited_names(&origins, &[&base, &tool], &shell)[3],
            "b1:e3:inner"
        );
    }

    #[test]
    fn edges_resolve_only_when_the_pair_is_unique() -> Result<(), KernelError> {
        let names = strings(&["a", "b", "c", ""]);
        let adjacency = [[0, 1], [1, 2], [2, 0], [2, 3]];
        let pair = ["b".to_owned(), "a".to_owned()];
        assert_eq!(edge_between(&names, &adjacency, &pair)?, 0);
        assert_eq!(edge_face_names(&names, &adjacency, 1)?, ["b", "c"]);
        // Face sem nome não gera referência.
        assert!(edge_face_names(&names, &adjacency, 3).is_err());
        // Par repetido é ambíguo; par ausente é perdido.
        assert!(edge_between(&names, &[[0, 1], [1, 0]], &pair).is_err());
        assert!(edge_between(&names, &adjacency, &["a".into(), "z".into()]).is_err());
        assert_eq!(face_named(&names, "c")?, 2);
        assert!(face_named(&strings(&["x", "x"]), "x").is_err());
        Ok(())
    }
}
