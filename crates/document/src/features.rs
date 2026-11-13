//! Histórico explícito por peça. Referências nunca atravessam definições de peças.
//!
//! Referências a regiões, faces e arestas são estáveis: guardam a identidade de origem do
//! elemento (entidades do rascunho, nomes de faces), não sua posição em uma lista. Quando a
//! origem desaparece, o recálculo falha com uma mensagem pedindo nova seleção; nenhuma outra
//! região, face ou aresta é escolhida no lugar.
use super::*;
mod extrusion;
mod labels;
pub use extrusion::ExtrusionProfile;
pub use labels::{LabelMode, LabelSpec};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub output: BodyId,
    pub operation: Operation,
    /// Suprimida pelo usuário: não é construída, mas conserva parâmetros e referências para
    /// voltar a ser ativada. Operações que dependem dela também deixam de ser construídas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppressed: bool,
}

/// Situação de uma operação no recálculo da peça.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureState {
    Active,
    /// Suprimida explicitamente.
    Suppressed,
    /// Consome ou consulta o resultado de uma operação inativa; nenhuma outra geometria é
    /// escolhida no lugar, então ela também fica sem construir.
    Dependent,
    /// Posterior ao marcador do histórico (`Part::history_marker`).
    RolledBack,
}
impl FeatureState {
    pub fn is_active(self) -> bool {
        self == Self::Active
    }
}

/// Estado de cada operação, na ordem do histórico.
pub fn states(part: &Part) -> Vec<FeatureState> {
    let marker = part.history_marker.unwrap_or(usize::MAX);
    let mut inactive = HashSet::new();
    part.features
        .iter()
        .enumerate()
        .map(|(index, feature)| {
            let state = if index >= marker {
                FeatureState::RolledBack
            } else if feature.suppressed {
                FeatureState::Suppressed
            } else if feature
                .operation
                .references()
                .iter()
                .any(|b| inactive.contains(b))
            {
                FeatureState::Dependent
            } else {
                FeatureState::Active
            };
            if !state.is_active() {
                inactive.insert(feature.output);
            }
            state
        })
        .collect()
}

/// Resultados de operações inativas. Guardam a última geometria construída, mas ficam ocultos
/// e não participam da cena, de medições nem de exportações.
pub fn inactive_outputs(part: &Part) -> HashSet<BodyId> {
    part.features
        .iter()
        .zip(states(part))
        .filter(|(_, state)| !state.is_active())
        .map(|(f, _)| f.output)
        .collect()
}

/// Corpos que o histórico mantém ocultos: resultados inativos e entradas consumidas por
/// operações ativas.
pub fn hidden_by_history(part: &Part) -> HashSet<BodyId> {
    let mut hidden = HashSet::new();
    for (feature, state) in part.features.iter().zip(states(part)) {
        if state.is_active() {
            hidden.extend(feature.operation.inputs());
        } else {
            hidden.insert(feature.output);
        }
    }
    hidden
}

/// Atualiza a visibilidade apenas dos corpos cuja condição no histórico mudou desde `before`
/// (resultado de `hidden_by_history` antes da edição). Escolhas manuais de visibilidade dos
/// demais corpos são preservadas; resultados inativos sempre ficam ocultos.
pub fn sync_visibility(part: &mut Part, before: &HashSet<BodyId>) {
    let after = hidden_by_history(part);
    let inactive = inactive_outputs(part);
    for body in &mut part.bodies {
        if inactive.contains(&body.id) || (after.contains(&body.id) && !before.contains(&body.id)) {
            body.visible = false;
        } else if before.contains(&body.id) && !after.contains(&body.id) {
            body.visible = true;
        }
    }
}

/// Operações que dependem, direta ou indiretamente, do resultado `output`, na ordem do histórico.
pub fn dependents(part: &Part, output: BodyId) -> Vec<BodyId> {
    let mut affected = HashSet::from([output]);
    let mut result = Vec::new();
    for feature in &part.features {
        if feature
            .operation
            .references()
            .iter()
            .any(|b| affected.contains(b))
        {
            affected.insert(feature.output);
            result.push(feature.output);
        }
    }
    result
}

/// Identidade de uma parte desconectada no recálculo: nomes estáveis das suas faces e
/// centro de massa na última construção. Vale a parte com mais faces em comum; no empate,
/// a mais próxima. Partes indistinguíveis fazem o recálculo falhar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieceKey {
    /// Nomes não vazios, ordenados e sem repetição.
    pub faces: Vec<String>,
    pub center: [f64; 3],
}

/// Região de um rascunho escolhida para extrusão ou revolução.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionRef {
    /// Seleção de várias células do mesmo rascunho, na ordem dos cliques.
    /// Não aceita listas aninhadas, vazias ou duplicadas.
    Multiple(Vec<RegionRef>),
    /// Célula do arranjo plano: ciclo canônico de entidades com orientação. Permite
    /// distinguir lentes/crescentes que usam as mesmas curvas em sentidos diferentes.
    Area(Vec<(u64, bool)>),
    /// Entidades do contorno externo no momento da seleção (`profiles::find_region`).
    Boundary(Vec<u64>),
    /// Parte de uma face dividida por traços abertos do rascunho, como se o contorno da
    /// face fizesse parte dele. `seed` é um ponto da parte no plano do rascunho, em
    /// nanômetros inteiros, e `entities` os traços que a delimitam, cada um com o lado
    /// em que a parte fica. Conferir os dois impede cair na parte vizinha em silêncio.
    Cell {
        seed: [i64; 2],
        entities: Vec<(u64, bool)>,
    },
    /// Formato até a versão 6: posição na lista de regiões ordenada por área. Mantida só
    /// para abrir projetos antigos; a próxima edição da operação a converte em `Boundary`.
    Index(usize),
}
impl RegionRef {
    pub fn selections(&self) -> &[RegionRef] {
        match self {
            Self::Multiple(regions) => regions,
            _ => std::slice::from_ref(self),
        }
    }
}

/// Perfil referenciado por identidade. Compartilhado por revolução e transições.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRef {
    pub sketch: SketchId,
    pub region: RegionRef,
}

/// Correspondência explícita de contornos internos, capturada ao criar a
/// transição. IDs são locais ao perfil, preservados nas edições e clonagem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolePair {
    pub start: Vec<u64>,
    pub end: Vec<u64>,
}

/// Guia associativa de uma transição curva. Eixo pertence ao plano inicial;
/// trajetória referencia uma cadeia aberta de linhas/arcos de outro rascunho.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SweepRoute {
    Automatic,
    Axis {
        sketch: SketchId,
        line: u64,
        #[serde(default)]
        reverse: bool,
    },
    Sketch {
        sketch: SketchId,
        entities: Vec<u64>,
    },
}

/// Aresta usada por arredondamento ou chanfro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeRef {
    /// Aresta entre duas faces nomeadas pela origem.
    Between([String; 2]),
    /// Índice válido apenas enquanto o B-rep de entrada for idêntico ao da seleção
    /// (`signature`). Usado quando as faces adjacentes não têm nome estável.
    Index { index: usize, signature: u64 },
}

/// Face usada pela casca.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceRef {
    Named(String),
    Index { index: usize, signature: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Prisma exato de uma face plana, com furos. Distâncias seguem a normal externa;
    /// a referência à face acompanha o corpo no recálculo paramétrico.
    ExtrudeFace {
        base: BodyId,
        face: FaceRef,
        from: String,
        to: String,
        combine: Option<Combine>,
        until: Option<ExtrusionLimit>,
        /// Célula da face delimitada por rascunhos, inclusive o lado de fora de um
        /// contorno: o prisma é a interseção destas condições. Vazia usa a face inteira.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        boundaries: Vec<PaintBoundary>,
    },
    /// Com limites iguais e base explícita, subdivide as faces coplanares sem alterar volume.
    Extrude {
        sketch: SketchId,
        region: RegionRef,
        from: String,
        /// Fim da extrusão; ignorado quando `until` define um plano limite.
        to: String,
        /// Une, corta ou intersecta o resultado com um corpo existente, que é consumido.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        combine: Option<Combine>,
        /// Termina em uma face plana, rascunho ou plano de trabalho da mesma peça.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until: Option<ExtrusionLimit>,
    },
    /// Uma extrusão de áreas coplanares de corpos/rascunhos distintos da mesma peça.
    ExtrudeMany {
        profiles: Vec<ExtrusionProfile>,
        /// Identidades persistentes das áreas; conservadas ao clonar e reordenar.
        #[serde(default)]
        profile_keys: Vec<u64>,
        from: String,
        to: String,
        combine: Option<Combine>,
        until: Option<ExtrusionLimit>,
    },
    Revolve {
        sketch: SketchId,
        region: RegionRef,
        /// Eixo fixo em coordenadas da peça, usado quando `axis_line` é `None`.
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        angle: String,
        /// Linha do rascunho usada como eixo (em geral de construção). Acompanha as cotas
        /// do rascunho a cada recálculo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis_line: Option<u64>,
        /// Ausente nos arquivos antigos: a linha pertence ao rascunho principal.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis_sketch: Option<SketchId>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_profiles: Vec<ProfileRef>,
    },
    /// Sólido entre duas áreas em planos diferentes. Furos correspondem pela
    /// ordem de criação de seus contornos e precisam existir nos dois perfis.
    Loft {
        start: ProfileRef,
        end: ProfileRef,
        #[serde(default)]
        ruled: bool,
        /// Polígono convexo → círculo em triângulos planos e cones, que aceitam casca.
        /// Outros pares seguem a transição comum. Ausente em projetos anteriores.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        developable: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        holes: Option<Vec<HolePair>>,
    },
    Sweep {
        start: ProfileRef,
        end: ProfileRef,
        route: SweepRoute,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        holes: Option<Vec<HolePair>>,
    },
    /// Seção circular constante ao longo de um círculo ou cadeia de linhas e arcos.
    Pipe {
        sketch: SketchId,
        entities: Vec<u64>,
        diameter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thickness: Option<String>,
    },
    Label {
        base: BodyId,
        face: FaceRef,
        spec: LabelSpec,
    },
    Boolean {
        base: BodyId,
        tool: BodyId,
        mode: BooleanMode,
    },
    /// Arredondamento de raio constante. O núcleo propaga cada aresta pelas tangentes.
    Fillet {
        base: BodyId,
        edge: EdgeRef,
        radius: String,
        /// Outras arestas arredondadas na mesma operação.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_edges: Vec<EdgeRef>,
    },
    /// Chanfro simétrico, ou por distância e ângulo quando `angle` é informado.
    Chamfer {
        base: BodyId,
        edge: EdgeRef,
        distance: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angle: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_edges: Vec<EdgeRef>,
    },
    /// Casca com as faces indicadas removidas, para dentro ou para fora.
    Shell {
        base: BodyId,
        face: FaceRef,
        thickness: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_faces: Vec<FaceRef>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        outward: bool,
        /// Remove também uma saída. Ausente: primeira face oposta única.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        through: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_face: Option<FaceRef>,
    },
    /// Furos centrados em pontos de um rascunho, furando contra a normal do plano do
    /// rascunho (a favor dela com `reverse`).
    Hole {
        base: BodyId,
        sketch: SketchId,
        /// Pontos do rascunho usados como centros; a identidade dos pontos é estável.
        points: Vec<u64>,
        spec: HoleSpec,
    },
    /// Cópias do corpo unidas em padrão linear ou circular.
    Pattern { base: BodyId, layout: PatternLayout },
    /// Espelhamento por plano. Com `keep_original`, une o corpo e sua imagem.
    Mirror {
        base: BodyId,
        origin: [f64; 3],
        normal: [f64; 3],
        keep_original: bool,
    },
    /// Parte do corpo do lado para onde `normal` aponta. Referência opcional
    /// mantém o plano associativo; coordenadas fixas preservam projetos antigos.
    Split {
        base: BodyId,
        origin: [f64; 3],
        normal: [f64; 3],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cutting_plane: Option<ExtrusionLimit>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        reverse: bool,
    },
    /// Resultado complementar de uma divisão. Consulta o mesmo plano da etapa
    /// primária, impedindo divergência entre os lados durante edição/recálculo.
    SplitRemainder { base: BodyId, division: BodyId },
    /// Uma parte desconectada do resultado de `base`: um corpo é sempre um objeto só.
    /// Criada automaticamente quando uma operação deixa partes soltas; `base` fica oculto.
    Piece { base: BodyId, key: PieceKey },
    /// Elimina contatos em linha com um pino de `radius` ao longo de cada um. `contacts`
    /// guarda os nomes ordenados das faces em volta do contato: se ele deixar de existir,
    /// a operação falha em vez de agir sobre outra geometria.
    RepairContacts {
        base: BodyId,
        contacts: Vec<Vec<String>>,
        mode: ContactRepair,
        radius: String,
    },
    /// Passagem de um componente: a silhueta na direção de inserção, afastada de `clearance`,
    /// removida do corpo desde o fundo do componente até a abertura.
    ComponentCut {
        base: BodyId,
        outline: ComponentOutline,
        clearance: String,
        /// Folga abaixo do ponto mais fundo do componente.
        floor: String,
        /// Comprimento da passagem a partir do fundo; `None` atravessa o corpo até a saída.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        depth: Option<String>,
    },
    /// Une áreas vizinhas no mesmo plano, retirando a linha divisória. Volume igual.
    MergeFaces { base: BodyId, faces: Vec<FaceRef> },
    /// Remove faces escolhidas (arredondamentos, chanfros, furos) e estende as vizinhas até
    /// fecharem o sólido, como a exclusão de faces de outros CADs. Útil em STEP importado,
    /// onde o arredondamento não está no histórico.
    RemoveFaces { base: BodyId, faces: Vec<FaceRef> },
    /// Ressalto ou rebaixo ao longo do contorno de uma face plana, para fechar tampas.
    Lip {
        base: BodyId,
        face: FaceRef,
        spec: LipSpec,
    },
    /// Torres de parafuso sobre pontos de um rascunho, ao longo da normal do plano (contra ela
    /// com `reverse`).
    ScrewBoss {
        base: BodyId,
        sketch: SketchId,
        points: Vec<u64>,
        spec: BossSpec,
    },
    /// Presilhas em balanço sobre pontos de um rascunho, ou a janela que as recebe.
    SnapFit {
        base: BodyId,
        sketch: SketchId,
        points: Vec<u64>,
        spec: SnapSpec,
    },
}

/// Se a operação acrescenta o ressalto ou abre o rebaixo que o recebe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LipKind {
    Lip,
    Groove,
}

/// Contorno da face a partir do qual o anel é medido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LipReference {
    Outer,
    Inner,
}

/// Anel medido a partir do contorno de referência, para dentro do material da face.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LipSpec {
    pub kind: LipKind,
    pub reference: LipReference,
    /// Distância do contorno até o início do anel.
    pub inset: String,
    pub width: String,
    /// Altura do ressalto. O rebaixo correspondente desce a mesma medida mais a folga.
    pub height: String,
    /// Só no rebaixo: alarga o anel e aprofunda o corte em relação ao ressalto.
    pub clearance: String,
}

/// Torre com furo para parafuso, medida a partir do plano do rascunho.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossSpec {
    pub outer_diameter: String,
    pub hole_diameter: String,
    pub height: String,
    /// Profundidade do furo a partir do topo; `None` desce até a base da torre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_depth: Option<String>,
    /// Raio de concordância na base: sem ele, a torre quebra rente ao piso.
    pub fillet: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reverse: bool,
}

/// Se a operação acrescenta a presilha ou abre a janela que a recebe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapKind {
    Hook,
    Window,
}

/// Haste em balanço com cabeça. A janela usa as mesmas medidas, para os dois lados
/// continuarem casados quando uma delas muda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapSpec {
    pub kind: SnapKind,
    /// Comprimento da haste até a face de retenção.
    pub length: String,
    pub thickness: String,
    pub width: String,
    /// Quanto a cabeça avança além da haste.
    pub overhang: String,
    /// Altura da rampa de entrada.
    pub head: String,
    /// Raio na raiz da haste, onde a tensão é maior.
    pub fillet: String,
    /// Direção da cabeça no plano do rascunho, a partir do eixo U.
    pub angle: String,
    /// Só na janela: folga em volta da cabeça.
    pub clearance: String,
    /// Só na janela: quanto ela avança além da cabeça, para atravessar a parede.
    pub window: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reverse: bool,
}
impl SnapSpec {
    /// Deformação na raiz ao passar a cabeça, pela fórmula da viga em balanço de seção
    /// retangular: 1,5 × espessura × avanço / comprimento². Estimativa para avisar, não cálculo
    /// de engenharia.
    pub fn strain(thickness: f64, overhang: f64, length: f64) -> f64 {
        1.5 * thickness * overhang / (length * length)
    }
}

/// Limite de pontos da silhueta gravada em uma operação de recorte.
pub const MAX_OUTLINE_POINTS: usize = 100_000;

/// Silhueta de um componente, congelada ao criar o recorte para as peças continuarem
/// independentes: mudar a folga recalcula, mas mover o componente pede nova captura.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentOutline {
    /// Corpo de origem, só para o usuário reconhecer o recorte.
    pub source: String,
    /// Plano no ponto mais fundo do componente, em coordenadas da peça. A normal aponta para
    /// a abertura, contra a direção de inserção.
    pub plane: nator_sketch::Plane,
    /// Contornos externos no plano, em mm.
    pub contours: Vec<Vec<[f64; 2]>>,
}

/// Como o reparo elimina um contato em linha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactRepair {
    /// Une os dois volumes acrescentando material.
    Weld,
    /// Afasta os dois volumes removendo material.
    Separate,
}

/// Furo com medidas em expressões dos parâmetros da peça.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoleSpec {
    pub diameter: String,
    /// Profundidade da parede cilíndrica; `None` atravessa o corpo.
    pub depth: Option<String>,
    pub head: HoleHeadSpec,
    /// Ângulo total da ponta de broca em furos cegos; `None` deixa o fundo plano.
    pub tip_angle: Option<String>,
    pub reverse: bool,
    /// Rosca simplificada: registrada para documentação, sem hélice na geometria.
    pub thread: Option<ThreadSpec>,
    /// Direção de cima na impressão: a seção vira gota, que imprime sem suporte em furos
    /// deitados. Fica gravada na operação para o furo não mudar se a face na mesa mudar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teardrop: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HoleHeadSpec {
    Simple,
    Counterbore {
        diameter: String,
        depth: String,
    },
    Countersink {
        diameter: String,
        angle: String,
    },
    /// Alojamento sextavado de porca cativa: medida entre faces e profundidade.
    Nut {
        across_flats: String,
        depth: String,
    },
}

/// Rosca registrada no furo: designação (por exemplo `M6`), diâmetro nominal, passo e
/// profundidade útil. O furo modelado é o de broca; a rosca não altera a geometria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadSpec {
    pub designation: String,
    pub nominal: String,
    pub pitch: String,
    pub depth: Option<String>,
}

/// Direção de repetição de um padrão linear: `count` ocorrências espaçadas de `spacing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearAxis {
    pub direction: [f64; 3],
    pub count: u32,
    pub spacing: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternLayout {
    /// Uma ou duas direções; com duas, as ocorrências formam uma grade.
    Linear {
        first: LinearAxis,
        second: Option<LinearAxis>,
    },
    /// `count` ocorrências ao redor do eixo. Um ângulo total de 360° distribui as cópias
    /// por igual; outro ângulo coloca a última cópia exatamente nele.
    Circular {
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        count: u32,
        angle: String,
    },
}

/// Limite de ocorrências por padrão e de furos por operação neste protótipo.
pub const MAX_OCCURRENCES: usize = 1000;
/// Combinação do resultado de uma extrusão com um corpo existente da peça.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combine {
    pub target: BodyId,
    pub mode: BooleanMode,
    /// Na união, junta numa só face cada face nova da extrusão com as vizinhas no mesmo
    /// plano. Ausente em projetos anteriores, que assim recalculam sem mudança.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub merge_coplanar: bool,
}

/// Limite de extrusão por uma face plana de um corpo da mesma peça. A distância é recalculada
/// a cada recálculo, então a extrusão acompanha a face; faces de outras peças não são aceitas,
/// para manter as peças independentes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntilFace {
    pub body: BodyId,
    pub face: FaceRef,
}

/// O formato de face permanece compatível com os arquivos antigos. Planos de
/// rascunho/trabalho são referências da mesma peça, atualizadas no recálculo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtrusionLimit {
    Face(UntilFace),
    Sketch { sketch: SketchId },
    WorkPlane { work_plane: WorkPlaneId },
}
impl From<UntilFace> for ExtrusionLimit {
    fn from(face: UntilFace) -> Self {
        Self::Face(face)
    }
}
impl ExtrusionLimit {
    fn validate(&self, part: &Part) -> Result<(), DocumentError> {
        match self {
            Self::Face(UntilFace {
                face: FaceRef::Named(name),
                ..
            }) => validate_face_name(name),
            Self::Face(_) => Ok(()),
            Self::Sketch { sketch } if part.sketches.iter().any(|s| s.id == *sketch) => Ok(()),
            Self::WorkPlane { work_plane }
                if part.work_planes.iter().any(|p| p.id == *work_plane) =>
            {
                Ok(())
            }
            _ => Err(DocumentError("Plano limite inexistente nesta peça.".into())),
        }
    }
    fn remap(
        &mut self,
        bodies: &HashMap<BodyId, BodyId>,
        sketches: &HashMap<SketchId, SketchId>,
    ) -> Result<(), DocumentError> {
        match self {
            Self::Face(limit) => {
                limit.body = *bodies
                    .get(&limit.body)
                    .ok_or_else(|| DocumentError("Corpo limite ausente ao clonar.".into()))?;
                if let FaceRef::Named(name) = &mut limit.face {
                    *name = remap_face_name(name, bodies);
                }
            }
            Self::Sketch { sketch } => {
                *sketch = *sketches
                    .get(sketch)
                    .ok_or_else(|| DocumentError("Rascunho limite ausente ao clonar.".into()))?
            }
            Self::WorkPlane { .. } => {} // Remapeado com os planos da peça em assembly::clone_definition.
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanMode {
    Join,
    Cut,
    Intersect,
}
impl Operation {
    /// Corpos consumidos pela operação: ficam ocultos enquanto ela existir.
    pub fn inputs(&self) -> Vec<BodyId> {
        match self {
            Self::ExtrudeMany {
                profiles,
                combine: Some(combine),
                ..
            } => {
                let mut inputs = vec![combine.target];
                inputs.extend(profiles.iter().filter_map(ExtrusionProfile::base));
                inputs.sort_unstable_by_key(|b| b.0);
                inputs.dedup();
                inputs
            }
            Self::Extrude {
                combine: Some(combine),
                ..
            }
            | Self::ExtrudeFace {
                combine: Some(combine),
                ..
            } => vec![combine.target],
            Self::Boolean { base, tool, .. } => vec![*base, *tool],
            Self::Fillet { base, .. }
            | Self::Chamfer { base, .. }
            | Self::Shell { base, .. }
            | Self::Label { base, .. }
            | Self::Hole { base, .. }
            | Self::Pattern { base, .. }
            | Self::Mirror { base, .. }
            | Self::Split { base, .. }
            | Self::SplitRemainder { base, .. }
            | Self::Piece { base, .. }
            | Self::RepairContacts { base, .. }
            | Self::ComponentCut { base, .. }
            | Self::Lip { base, .. }
            | Self::MergeFaces { base, .. }
            | Self::RemoveFaces { base, .. }
            | Self::ScrewBoss { base, .. }
            | Self::SnapFit { base, .. } => vec![*base],
            _ => Vec::new(),
        }
    }
    /// Todos os corpos de que a operação depende (consumidos ou apenas consultados), para a
    /// ordem do histórico e para impedir a exclusão de um corpo ainda referenciado.
    pub fn references(&self) -> Vec<BodyId> {
        let mut bodies = self.inputs();
        match self {
            Self::Extrude { until, .. }
            | Self::ExtrudeFace { until, .. }
            | Self::ExtrudeMany { until, .. } => {
                if let Some(ExtrusionLimit::Face(limit)) = until {
                    bodies.push(limit.body);
                }
                if let Self::ExtrudeFace { base, .. } = self {
                    bodies.push(*base);
                }
            }
            Self::Split {
                cutting_plane: Some(ExtrusionLimit::Face(limit)),
                ..
            } => bodies.push(limit.body),
            Self::SplitRemainder { division, .. } => bodies.push(*division),
            _ => {}
        }
        if let Self::ExtrudeMany { profiles, .. } = self {
            bodies.extend(profiles.iter().filter_map(ExtrusionProfile::base));
        }
        bodies.sort_unstable_by_key(|b| b.0);
        bodies.dedup();
        bodies
    }
    /// Referências de face da operação, com o corpo a que cada uma pertence.
    pub fn face_refs_mut(&mut self) -> Vec<(BodyId, &mut FaceRef)> {
        fn limit(until: Option<&mut ExtrusionLimit>) -> Option<(BodyId, &mut FaceRef)> {
            match until {
                Some(ExtrusionLimit::Face(limit)) => Some((limit.body, &mut limit.face)),
                _ => None,
            }
        }
        match self {
            Self::ExtrudeMany {
                profiles, until, ..
            } => profiles
                .iter_mut()
                .filter_map(|p| match p {
                    ExtrusionProfile::Face { base, face, .. } => Some((*base, face)),
                    _ => None,
                })
                .chain(limit(until.as_mut()))
                .collect(),
            Self::ExtrudeFace {
                base, face, until, ..
            } => std::iter::once((*base, face))
                .chain(limit(until.as_mut()))
                .collect(),
            Self::Extrude { until, .. } => limit(until.as_mut()).into_iter().collect(),
            Self::Split { cutting_plane, .. } => {
                limit(cutting_plane.as_mut()).into_iter().collect()
            }
            Self::Label { base, face, .. } | Self::Lip { base, face, .. } => vec![(*base, face)],
            Self::MergeFaces { base, faces } | Self::RemoveFaces { base, faces } => {
                faces.iter_mut().map(|f| (*base, f)).collect()
            }
            Self::Shell {
                base,
                face,
                additional_faces,
                exit_face,
                ..
            } => std::iter::once(face)
                .chain(additional_faces)
                .chain(exit_face)
                .map(|face| (*base, face))
                .collect(),
            _ => Vec::new(),
        }
    }
    /// Referências de aresta da operação, com o corpo a que cada uma pertence.
    pub fn edge_refs_mut(&mut self) -> Vec<(BodyId, &mut EdgeRef)> {
        match self {
            Self::Fillet {
                base,
                edge,
                additional_edges,
                ..
            }
            | Self::Chamfer {
                base,
                edge,
                additional_edges,
                ..
            } => std::iter::once(edge)
                .chain(additional_edges)
                .map(|edge| (*base, edge))
                .collect(),
            _ => Vec::new(),
        }
    }
    pub fn uses_sketch(&self, id: SketchId) -> bool {
        matches!(self,Self::ExtrudeMany {profiles,until,..} if profiles.iter().any(|p|p.uses_sketch(id)) || matches!(until,Some(ExtrusionLimit::Sketch{sketch}) if *sketch==id))
            || self.sketch() == Some(id)
            || matches!(self,Self::Loft {end,..} | Self::Sweep {end,..} if end.sketch==id)
            || matches!(self,Self::Sweep {route:SweepRoute::Axis {sketch,..}|SweepRoute::Sketch {sketch,..},..} if *sketch==id)
            || matches!(self,Self::Revolve {additional_profiles,axis_sketch,..} if *axis_sketch==Some(id) || additional_profiles.iter().any(|p|p.sketch==id))
            || matches!(self,Self::Extrude{until:Some(ExtrusionLimit::Sketch{sketch}),..}|Self::ExtrudeFace{until:Some(ExtrusionLimit::Sketch{sketch}),..}|Self::Split{cutting_plane:Some(ExtrusionLimit::Sketch{sketch}),..} if *sketch==id)
            || matches!(self,Self::ExtrudeFace{boundaries,..} if boundaries.iter().any(|b|b.mask.sketch==id))
    }
    pub fn uses_work_plane(&self, id: WorkPlaneId) -> bool {
        matches!(self,
            Self::ExtrudeMany {until:Some(ExtrusionLimit::WorkPlane {work_plane}),..}
            | Self::Extrude {until:Some(ExtrusionLimit::WorkPlane {work_plane}),..}
            | Self::ExtrudeFace {until:Some(ExtrusionLimit::WorkPlane {work_plane}),..}
            | Self::Split {cutting_plane:Some(ExtrusionLimit::WorkPlane {work_plane}),..} if *work_plane==id)
    }
    pub fn sketch(&self) -> Option<SketchId> {
        match self {
            Self::ExtrudeMany { profiles, .. } => profiles.iter().find_map(|p| match p {
                ExtrusionProfile::Sketch(p) => Some(p.sketch),
                _ => None,
            }),
            Self::Loft { start, .. } | Self::Sweep { start, .. } => Some(start.sketch),
            Self::Extrude { sketch, .. }
            | Self::Revolve { sketch, .. }
            | Self::Pipe { sketch, .. }
            | Self::Hole { sketch, .. }
            | Self::ScrewBoss { sketch, .. }
            | Self::SnapFit { sketch, .. } => Some(*sketch),
            _ => None,
        }
    }
}

/// Dono das faces produzidas pela operação cujo resultado é o corpo `id`.
/// Os nomes de faces têm a forma `b{id}:...` (ver `nator_kernel`).
pub fn body_owner(id: BodyId) -> String {
    format!("b{}", id.0)
}

/// Ao clonar uma peça, nomes `b{antigo}:...` passam a `b{novo}:...`, para que as
/// referências do clone apontem para os corpos do próprio clone.
pub fn remap_face_name(name: &str, bodies: &HashMap<BodyId, BodyId>) -> String {
    let Some((digits, tail)) = name.strip_prefix('b').and_then(|rest| rest.split_once(':')) else {
        return name.to_owned();
    };
    match digits
        .parse::<u64>()
        .ok()
        .and_then(|id| bodies.get(&BodyId(id)))
    {
        Some(new) => format!("{}:{tail}", body_owner(*new)),
        None => name.to_owned(),
    }
}

impl Feature {
    pub fn remap(
        &mut self,
        bodies: &HashMap<BodyId, BodyId>,
        sketches: &HashMap<SketchId, SketchId>,
    ) -> Result<(), DocumentError> {
        let body = |id: &mut BodyId| -> Result<(), DocumentError> {
            *id = *bodies
                .get(id)
                .ok_or_else(|| DocumentError("Referência de corpo ausente ao clonar.".into()))?;
            Ok(())
        };
        body(&mut self.output)?;
        let sketch = |id: &mut SketchId| -> Result<(), DocumentError> {
            *id = *sketches
                .get(id)
                .ok_or_else(|| DocumentError("Rascunho ausente ao clonar.".into()))?;
            Ok(())
        };
        match &mut self.operation {
            Operation::ExtrudeMany {
                profiles,
                combine,
                until,
                ..
            } => {
                for profile in profiles {
                    profile.remap(bodies, sketches)?;
                }
                if let Some(combine) = combine {
                    body(&mut combine.target)?;
                }
                if let Some(limit) = until {
                    limit.remap(bodies, sketches)?;
                }
            }
            Operation::ExtrudeFace {
                base,
                face,
                combine,
                until,
                boundaries,
                ..
            } => {
                body(base)?;
                if let FaceRef::Named(name) = face {
                    *name = remap_face_name(name, bodies);
                }
                if let Some(combine) = combine {
                    body(&mut combine.target)?;
                }
                if let Some(limit) = until {
                    limit.remap(bodies, sketches)?;
                }
                for boundary in boundaries {
                    sketch(&mut boundary.mask.sketch)?;
                }
            }
            Operation::Extrude {
                sketch: id,
                combine,
                until,
                ..
            } => {
                sketch(id)?;
                if let Some(combine) = combine {
                    body(&mut combine.target)?;
                }
                if let Some(limit) = until {
                    limit.remap(bodies, sketches)?;
                }
            }
            Operation::Pipe { sketch: id, .. } => sketch(id)?,
            Operation::Loft { start, end, .. } | Operation::Sweep { start, end, .. } => {
                sketch(&mut start.sketch)?;
                sketch(&mut end.sketch)?;
                if let Operation::Sweep {
                    route:
                        SweepRoute::Axis { sketch: id, .. } | SweepRoute::Sketch { sketch: id, .. },
                    ..
                } = &mut self.operation
                {
                    sketch(id)?;
                }
            }
            Operation::Revolve {
                sketch: id,
                axis_sketch,
                additional_profiles,
                ..
            } => {
                sketch(id)?;
                if let Some(id) = axis_sketch {
                    sketch(id)?;
                }
                for profile in additional_profiles {
                    sketch(&mut profile.sketch)?;
                }
            }
            Operation::Hole {
                base, sketch: id, ..
            }
            | Operation::ScrewBoss {
                base, sketch: id, ..
            }
            | Operation::SnapFit {
                base, sketch: id, ..
            } => {
                body(base)?;
                sketch(id)?;
            }
            Operation::Lip { base, face, .. } => {
                body(base)?;
                if let FaceRef::Named(name) = face {
                    *name = remap_face_name(name, bodies);
                }
            }
            Operation::MergeFaces { base, faces } | Operation::RemoveFaces { base, faces } => {
                body(base)?;
                for face in faces {
                    if let FaceRef::Named(name) = face {
                        *name = remap_face_name(name, bodies);
                    }
                }
            }
            Operation::Pattern { base, .. }
            | Operation::Mirror { base, .. }
            | Operation::ComponentCut { base, .. } => body(base)?,
            Operation::Split {
                base,
                cutting_plane,
                ..
            } => {
                body(base)?;
                if let Some(plane) = cutting_plane {
                    plane.remap(bodies, sketches)?;
                }
            }
            Operation::SplitRemainder { base, division } => {
                body(base)?;
                body(division)?;
            }
            Operation::Piece { base, key } => {
                body(base)?;
                for face in &mut key.faces {
                    *face = remap_face_name(face, bodies);
                }
                key.faces.sort_unstable();
                key.faces.dedup();
            }
            Operation::RepairContacts { base, contacts, .. } => {
                body(base)?;
                for face in contacts.iter_mut().flatten() {
                    *face = remap_face_name(face, bodies);
                }
                // Os nomes mudam de dono ao clonar: a ordem é parte da referência.
                contacts.iter_mut().for_each(|contact| contact.sort());
            }
            Operation::Boolean { base, tool, .. } => {
                body(base)?;
                body(tool)?;
            }
            Operation::Fillet {
                base,
                edge,
                additional_edges,
                ..
            }
            | Operation::Chamfer {
                base,
                edge,
                additional_edges,
                ..
            } => {
                body(base)?;
                for edge in std::iter::once(edge).chain(additional_edges) {
                    if let EdgeRef::Between(faces) = edge {
                        for face in faces {
                            *face = remap_face_name(face, bodies);
                        }
                    }
                }
            }
            Operation::Label { base, face, .. } => {
                body(base)?;
                if let FaceRef::Named(name) = face {
                    *name = remap_face_name(name, bodies);
                }
            }
            Operation::Shell {
                base,
                face,
                additional_faces,
                exit_face,
                ..
            } => {
                body(base)?;
                for face in std::iter::once(face)
                    .chain(additional_faces)
                    .chain(exit_face)
                {
                    if let FaceRef::Named(name) = face {
                        *name = remap_face_name(name, bodies);
                    }
                }
            }
        }
        Ok(())
    }
}
/// Assinatura conservadora para referências por índice: se a geometria mudar, a seleção
/// precisa ser refeita. Isso evita aplicar um chanfro em outra aresta após uma mudança.
pub fn signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
pub fn validate(part: &Part) -> Result<(), DocumentError> {
    if part.features.len() > 256 {
        return Err(DocumentError(
            "Histórico excede 256 operações por peça.".into(),
        ));
    }
    // O marcador no fim seria equivalente a nenhum marcador; exigir a forma canônica evita
    // dois estados diferentes para o mesmo histórico.
    if part
        .history_marker
        .is_some_and(|marker| marker >= part.features.len())
    {
        return Err(DocumentError(
            "Marcador do histórico fora das operações da peça.".into(),
        ));
    }
    let inactive = inactive_outputs(part);
    if part
        .bodies
        .iter()
        .any(|b| b.visible && inactive.contains(&b.id))
    {
        return Err(DocumentError(
            "O resultado de uma operação suprimida ou posterior ao marcador não pode ficar visível."
                .into(),
        ));
    }
    let outputs = part
        .features
        .iter()
        .map(|f| f.output)
        .collect::<HashSet<_>>();
    let mut ready = part
        .bodies
        .iter()
        .filter(|b| !outputs.contains(&b.id))
        .map(|b| b.id)
        .collect::<HashSet<_>>();
    for f in &part.features {
        validate_name(&f.name)?;
        if !part
            .bodies
            .iter()
            .any(|b| b.id == f.output && b.brep.is_some())
            || ready.contains(&f.output)
            || f.operation.references().iter().any(|b| !ready.contains(b))
            || f.operation
                .sketch()
                .is_some_and(|id| !part.sketches.iter().any(|s| s.id == id))
        {
            return Err(DocumentError(
                "Histórico contém referência ausente, repetida, futura ou de outra peça.".into(),
            ));
        }
        use parameters::Dimension::{Angle, Length};
        match &f.operation {
            Operation::ExtrudeMany {
                profiles,
                profile_keys,
                from,
                to,
                until,
                ..
            } => {
                if profile_keys.len() != profiles.len()
                    || profile_keys.iter().copied().collect::<HashSet<_>>().len()
                        != profile_keys.len()
                {
                    return Err(DocumentError(
                        "Identidades de perfis inválidas ou repetidas.".into(),
                    ));
                }
                if !(2..=64).contains(&profiles.len()) {
                    return Err(DocumentError(
                        "Selecione de duas a 64 áreas coplanares.".into(),
                    ));
                }
                for (index, profile) in profiles.iter().enumerate() {
                    profile.validate(part)?;
                    if profiles[..index].contains(profile) {
                        return Err(DocumentError("Perfil de extrusão repetido.".into()));
                    }
                }
                let start = part.parameters.evaluate(from, Length)?;
                if let Some(limit) = until {
                    limit.validate(part)?;
                } else if (start - part.parameters.evaluate(to, Length)?).abs() < 1e-7 {
                    return Err(DocumentError(
                        "Extrusão múltipla deve ter comprimento não nulo.".into(),
                    ));
                }
            }
            Operation::ExtrudeFace {
                face,
                from,
                to,
                until,
                combine,
                boundaries,
                ..
            } => {
                if let FaceRef::Named(name) = face {
                    validate_face_name(name)?;
                }
                if boundaries.len() > 64 {
                    return Err(DocumentError(
                        "Área de extrusão com delimitações em excesso.".into(),
                    ));
                }
                for boundary in boundaries {
                    validate_region(&boundary.mask.region)?;
                    if !part.sketches.iter().any(|s| s.id == boundary.mask.sketch) {
                        return Err(DocumentError(
                            "Rascunho que delimita a área inexistente.".into(),
                        ));
                    }
                }
                let a = part.parameters.evaluate(from, Length)?;
                // Só uma célula pode dividir a face em zero; a face inteira não muda nada.
                let imprint = combine.is_some() && !boundaries.is_empty();
                if let Some(limit) = until {
                    limit.validate(part)?;
                } else if (a - part.parameters.evaluate(to, Length)?).abs() < 1e-7 && !imprint {
                    return Err(DocumentError(
                        "Extrusão deve ter comprimento não nulo.".into(),
                    ));
                }
            }
            Operation::Extrude {
                from,
                to,
                region,
                until,
                combine,
                ..
            } => {
                validate_region(region)?;
                let a = part.parameters.evaluate(from, Length)?;
                match until {
                    // Com face limite, o comprimento só é conhecido no recálculo.
                    Some(limit) => {
                        limit.validate(part)?;
                    }
                    None => {
                        let b = part.parameters.evaluate(to, Length)?;
                        if (a - b).abs() < 1e-7 && combine.is_none() {
                            return Err(DocumentError(
                                "Extrusão deve ter comprimento não nulo.".into(),
                            ));
                        }
                    }
                }
            }
            Operation::Pipe {
                sketch,
                entities,
                diameter,
                thickness,
            } => {
                let source = part
                    .sketches
                    .iter()
                    .find(|s| s.id == *sketch)
                    .ok_or_else(|| {
                        DocumentError("Rascunho da trajetória do tubo inexistente.".into())
                    })?;
                let mut unique = HashSet::new();
                if entities.is_empty()
                    || entities.len() > 1024
                    || entities
                        .iter()
                        .any(|id| !unique.insert(*id) || source.sketch.entity(*id).is_err())
                {
                    return Err(DocumentError(
                        "Trajetória do tubo inexistente, repetida ou excessiva.".into(),
                    ));
                }
                for id in entities {
                    let entity = source
                        .sketch
                        .entity(*id)
                        .map_err(|e| DocumentError(e.to_string()))?;
                    if entity.infinite {
                        return Err(DocumentError(
                            "Tubo exige um círculo ou uma cadeia de linhas e arcos finitos.".into(),
                        ));
                    }
                }
                let d = part.parameters.evaluate(diameter, Length)?;
                if d <= 0. {
                    return Err(DocumentError(
                        "O diâmetro do tubo deve ser positivo.".into(),
                    ));
                }
                if let Some(thickness) = thickness {
                    let t = part.parameters.evaluate(thickness, Length)?;
                    if t <= 0. || 2. * t >= d {
                        return Err(DocumentError(
                            "A espessura do tubo deve ser positiva e menor que metade do diâmetro."
                                .into(),
                        ));
                    }
                }
            }
            Operation::Loft {
                start, end, holes, ..
            }
            | Operation::Sweep {
                start, end, holes, ..
            } => {
                if let Operation::Sweep { route, .. } = &f.operation {
                    match route {
                        SweepRoute::Automatic => {}
                        SweepRoute::Axis { sketch, line, .. } => {
                            let axis = part.sketches.iter().find(|s| s.id == *sketch).ok_or_else(
                                || {
                                    DocumentError(
                                        "Rascunho do eixo da transição inexistente.".into(),
                                    )
                                },
                            )?;
                            axis.sketch
                                .line(*line)
                                .map_err(|e| DocumentError(e.to_string()))?;
                        }
                        SweepRoute::Sketch { sketch, entities } => {
                            if entities.is_empty()
                                || entities.len() > 1024
                                || !part.sketches.iter().any(|s| {
                                    s.id == *sketch
                                        && entities.iter().all(|id| s.sketch.entity(*id).is_ok())
                                })
                            {
                                return Err(DocumentError(
                                    "Trajetória da transição inexistente ou excessiva.".into(),
                                ));
                            }
                        }
                    }
                }
                if holes.as_ref().is_some_and(|pairs| {
                    pairs.len() > 64
                        || pairs.iter().any(|p| {
                            p.start.is_empty()
                                || p.end.is_empty()
                                || p.start.len() > MAX_ELEMENTS
                                || p.end.len() > MAX_ELEMENTS
                        })
                }) {
                    return Err(DocumentError("Correspondência de furos inválida.".into()));
                }
                for profile in [start, end] {
                    validate_region(&profile.region)?;
                    if matches!(profile.region, RegionRef::Multiple(_)) {
                        return Err(DocumentError(
                            "Escolha uma única área em cada extremidade da transição.".into(),
                        ));
                    }
                }
                let a = part
                    .sketches
                    .iter()
                    .find(|s| s.id == start.sketch)
                    .ok_or_else(|| {
                        DocumentError("Perfil inicial inexistente nesta peça.".into())
                    })?;
                let b = part
                    .sketches
                    .iter()
                    .find(|s| s.id == end.sketch)
                    .ok_or_else(|| DocumentError("Perfil final inexistente nesta peça.".into()))?;
                if validate_coplanar(a.sketch.plane, b.sketch.plane).is_ok() {
                    return Err(DocumentError(
                        "Os perfis da transição precisam estar em planos diferentes.".into(),
                    ));
                }
            }
            Operation::Revolve {
                angle,
                axis_origin,
                axis_direction,
                region,
                sketch,
                axis_line,
                axis_sketch,
                additional_profiles,
            } => {
                validate_region(region)?;
                let source = part
                    .sketches
                    .iter()
                    .find(|s| s.id == *sketch)
                    .ok_or_else(|| DocumentError("Perfil de revolução inexistente.".into()))?;
                if additional_profiles.len() > MAX_OCCURRENCES {
                    return Err(DocumentError("Perfis demais na revolução.".into()));
                }
                for profile in additional_profiles {
                    validate_region(&profile.region)?;
                    let record = part
                        .sketches
                        .iter()
                        .find(|s| s.id == profile.sketch)
                        .ok_or_else(|| {
                            DocumentError("Perfil adicional inexistente nesta peça.".into())
                        })?;
                    validate_coplanar(source.sketch.plane, record.sketch.plane)?;
                }
                if let Some(id) = axis_sketch {
                    let axis = part.sketches.iter().find(|s| s.id == *id).ok_or_else(|| {
                        DocumentError("Rascunho do eixo inexistente nesta peça.".into())
                    })?;
                    validate_coplanar(source.sketch.plane, axis.sketch.plane)?;
                    if axis_line.is_none() {
                        return Err(DocumentError(
                            "Selecione a linha do eixo da revolução.".into(),
                        ));
                    }
                }
                let a = part.parameters.evaluate(angle, Angle)?;
                if !a.is_finite()
                    || a.abs() < 1e-7
                    || a.abs() > std::f64::consts::TAU + 1e-9
                    || axis_origin
                        .iter()
                        .chain(axis_direction)
                        .any(|v| !v.is_finite())
                    || nator_geometry::norm(*axis_direction) < 1e-9
                {
                    return Err(DocumentError(
                        "Revolução com eixo ou ângulo inválido.".into(),
                    ));
                }
            }
            Operation::Fillet {
                radius,
                edge,
                additional_edges,
                ..
            }
            | Operation::Chamfer {
                distance: radius,
                edge,
                additional_edges,
                ..
            } => {
                if additional_edges.len() >= MAX_OCCURRENCES {
                    return Err(DocumentError("Arestas demais na operação.".into()));
                }
                for edge in std::iter::once(edge).chain(additional_edges) {
                    if let EdgeRef::Between(faces) = edge {
                        faces.iter().try_for_each(|f| validate_face_name(f))?;
                    }
                }
                if part.parameters.evaluate(radius, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
                if let Operation::Chamfer {
                    angle: Some(angle), ..
                } = &f.operation
                {
                    let value = part.parameters.evaluate(angle, Angle)?;
                    if !(value > 0. && value < std::f64::consts::FRAC_PI_2) {
                        return Err(DocumentError(
                            "O ângulo do chanfro deve ficar entre 0° e 90°.".into(),
                        ));
                    }
                }
            }
            Operation::Label { face, spec, .. } => {
                if let FaceRef::Named(name) = face {
                    validate_face_name(name)?;
                }
                spec.validate(&part.parameters)?;
            }
            Operation::Shell {
                thickness,
                face,
                additional_faces,
                exit_face,
                ..
            } => {
                if additional_faces.len() >= MAX_OCCURRENCES {
                    return Err(DocumentError("Faces demais na casca.".into()));
                }
                for face in std::iter::once(face)
                    .chain(additional_faces)
                    .chain(exit_face)
                {
                    if let FaceRef::Named(name) = face {
                        validate_face_name(name)?;
                    }
                }
                if part.parameters.evaluate(thickness, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::ComponentCut {
                outline,
                clearance,
                floor,
                depth,
                ..
            } => {
                outline
                    .plane
                    .validate()
                    .map_err(|e| DocumentError(e.to_string()))?;
                let points = outline.contours.iter().map(Vec::len).sum::<usize>();
                if points > MAX_OUTLINE_POINTS
                    || outline.contours.is_empty()
                    || outline.contours.iter().any(|c| c.len() < 3)
                    || outline
                        .contours
                        .iter()
                        .flatten()
                        .flatten()
                        .any(|v| !v.is_finite())
                {
                    return Err(DocumentError("Silhueta do componente inválida.".into()));
                }
                if part.parameters.evaluate(clearance, Length)? < 0.
                    || part.parameters.evaluate(floor, Length)? < 0.
                {
                    return Err(DocumentError("Folga não pode ser negativa.".into()));
                }
                if let Some(depth) = depth
                    && part.parameters.evaluate(depth, Length)? <= 0.
                {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::MergeFaces { faces, .. } => {
                if faces.len() < 2 || faces.len() > 1000 {
                    return Err(DocumentError(
                        "União de áreas exige pelo menos duas faces.".into(),
                    ));
                }
                for face in faces {
                    if let FaceRef::Named(name) = face {
                        validate_face_name(name)?;
                    }
                }
            }
            Operation::RemoveFaces { faces, .. } => {
                if faces.is_empty() || faces.len() > 1000 {
                    return Err(DocumentError(
                        "Remover faces exige de 1 a 1000 faces.".into(),
                    ));
                }
                for face in faces {
                    if let FaceRef::Named(name) = face {
                        validate_face_name(name)?;
                    }
                }
            }
            Operation::Lip { face, spec, .. } => {
                if let FaceRef::Named(name) = face {
                    validate_face_name(name)?;
                }
                let length = |e: &str| part.parameters.evaluate(e, Length);
                if length(&spec.width)? <= 0. || length(&spec.height)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
                if length(&spec.inset)? < 0. || length(&spec.clearance)? < 0. {
                    return Err(DocumentError("Folga não pode ser negativa.".into()));
                }
            }
            Operation::ScrewBoss {
                points,
                sketch,
                spec,
                ..
            } => {
                validate_points(part, *sketch, points)?;
                let length = |e: &str| part.parameters.evaluate(e, Length);
                let (outer, hole) = (length(&spec.outer_diameter)?, length(&spec.hole_diameter)?);
                let height = length(&spec.height)?;
                if hole <= 0. || height <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
                if outer <= hole {
                    return Err(DocumentError(
                        "A torre precisa ser mais larga que o furo.".into(),
                    ));
                }
                let fillet = length(&spec.fillet)?;
                if fillet < 0. || fillet >= height {
                    return Err(DocumentError(
                        "A concordância deve ser menor que a altura da torre.".into(),
                    ));
                }
                if let Some(depth) = &spec.hole_depth
                    && length(depth)? <= 0.
                {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::SnapFit {
                points,
                sketch,
                spec,
                ..
            } => {
                validate_points(part, *sketch, points)?;
                let length = |e: &str| part.parameters.evaluate(e, Length);
                for value in [
                    &spec.length,
                    &spec.thickness,
                    &spec.width,
                    &spec.overhang,
                    &spec.head,
                ] {
                    if length(value)? <= 0. {
                        return Err(DocumentError("Dimensão deve ser positiva.".into()));
                    }
                }
                part.parameters.evaluate(&spec.angle, Angle)?;
                let fillet = length(&spec.fillet)?;
                if fillet < 0. || fillet >= length(&spec.length)? {
                    return Err(DocumentError(
                        "A concordância deve ser menor que a haste.".into(),
                    ));
                }
                if length(&spec.clearance)? < 0. || length(&spec.window)? < 0. {
                    return Err(DocumentError("Folga não pode ser negativa.".into()));
                }
            }
            Operation::RepairContacts {
                contacts, radius, ..
            } => {
                if contacts.is_empty() || contacts.len() >= MAX_OCCURRENCES {
                    return Err(DocumentError(
                        "O reparo precisa de 1 a 1000 contatos.".into(),
                    ));
                }
                for contact in contacts {
                    if contact.len() < 3 {
                        return Err(DocumentError("Contato do reparo inválido.".into()));
                    }
                    contact.iter().try_for_each(|f| validate_face_name(f))?;
                }
                if part.parameters.evaluate(radius, Length)? <= 0. {
                    return Err(DocumentError("Dimensão deve ser positiva.".into()));
                }
            }
            Operation::Hole {
                points,
                spec,
                sketch,
                ..
            } => {
                validate_points(part, *sketch, points)?;
                validate_hole(spec, &part.parameters)?;
            }
            Operation::Pattern { layout, .. } => validate_pattern(layout, &part.parameters)?,
            Operation::Mirror { origin, normal, .. } => validate_plane(*origin, *normal)?,
            Operation::Split {
                origin,
                normal,
                cutting_plane,
                ..
            } => {
                validate_plane(*origin, *normal)?;
                if let Some(plane) = cutting_plane {
                    plane.validate(part)?;
                }
            }
            Operation::SplitRemainder { base, division } => {
                if !part.features.iter().any(|f| {
                    f.output == *division
                        && matches!(f.operation,Operation::Split {base:source,..} if source==*base)
                }) {
                    return Err(DocumentError(
                        "A divisão complementar perdeu sua etapa primária.".into(),
                    ));
                }
            }
            Operation::Piece { key, .. } => {
                if !key.center.iter().all(|v| v.is_finite()) {
                    return Err(DocumentError("Centro da parte separada inválido.".into()));
                }
            }
            Operation::Boolean { .. } => {}
        }
        ready.insert(f.output);
    }
    Ok(())
}

fn positive(
    parameters: &parameters::Parameters,
    expression: &str,
    dimension: parameters::Dimension,
) -> Result<f64, DocumentError> {
    let value = parameters.evaluate(expression, dimension)?;
    if value.is_finite() && value > 0. {
        Ok(value)
    } else {
        Err(DocumentError(format!(
            "A medida \"{expression}\" deve ser positiva."
        )))
    }
}

fn validate_hole(
    spec: &HoleSpec,
    parameters: &parameters::Parameters,
) -> Result<(), DocumentError> {
    use parameters::Dimension::{Angle, Length};
    positive(parameters, &spec.diameter, Length)?;
    if let Some(depth) = &spec.depth {
        positive(parameters, depth, Length)?;
    }
    if let Some(angle) = &spec.tip_angle {
        positive(parameters, angle, Angle)?;
    }
    match &spec.head {
        HoleHeadSpec::Simple => {}
        HoleHeadSpec::Counterbore { diameter, depth } => {
            positive(parameters, diameter, Length)?;
            positive(parameters, depth, Length)?;
        }
        HoleHeadSpec::Countersink { diameter, angle } => {
            positive(parameters, diameter, Length)?;
            positive(parameters, angle, Angle)?;
        }
        HoleHeadSpec::Nut {
            across_flats,
            depth,
        } => {
            positive(parameters, across_flats, Length)?;
            positive(parameters, depth, Length)?;
        }
    }
    if let Some(up) = spec.teardrop {
        validate_direction(up, "A direção da gota")?;
        if spec.tip_angle.is_some()
            || matches!(
                spec.head,
                HoleHeadSpec::Counterbore { .. } | HoleHeadSpec::Countersink { .. }
            )
        {
            return Err(DocumentError(
                "O furo em gota exige fundo plano, sem rebaixo nem escareado.".into(),
            ));
        }
    }
    if let Some(thread) = &spec.thread {
        validate_name(&thread.designation)?;
        positive(parameters, &thread.nominal, Length)?;
        positive(parameters, &thread.pitch, Length)?;
        if let Some(depth) = &thread.depth {
            positive(parameters, depth, Length)?;
        }
    }
    Ok(())
}

fn validate_direction(direction: [f64; 3], name: &str) -> Result<(), DocumentError> {
    if direction.iter().any(|v| !v.is_finite()) || nator_geometry::norm(direction) < 1e-9 {
        return Err(DocumentError(format!("{name} deve ser não nula e finita.")));
    }
    Ok(())
}

fn validate_plane(origin: [f64; 3], normal: [f64; 3]) -> Result<(), DocumentError> {
    if origin.iter().any(|v| !v.is_finite()) {
        return Err(DocumentError("Origem do plano inválida.".into()));
    }
    validate_direction(normal, "A normal do plano")
}

fn validate_pattern(
    layout: &PatternLayout,
    parameters: &parameters::Parameters,
) -> Result<(), DocumentError> {
    use parameters::Dimension::{Angle, Length};
    let count = match layout {
        PatternLayout::Linear { first, second } => {
            let mut total = 1_usize;
            for axis in std::iter::once(first).chain(second) {
                validate_direction(axis.direction, "A direção do padrão")?;
                parameters.evaluate(&axis.spacing, Length)?;
                total = total.saturating_mul(axis.count as usize);
            }
            total
        }
        PatternLayout::Circular {
            axis_origin,
            axis_direction,
            count,
            angle,
        } => {
            validate_plane(*axis_origin, *axis_direction)?;
            let total = parameters.evaluate(angle, Angle)?;
            if !total.is_finite()
                || total.abs() < 1e-9
                || total.abs() > std::f64::consts::TAU + 1e-9
            {
                return Err(DocumentError(
                    "O ângulo do padrão circular deve estar entre 0° e 360°.".into(),
                ));
            }
            *count as usize
        }
    };
    if !(2..=MAX_OCCURRENCES).contains(&count) {
        return Err(DocumentError(
            "O padrão deve ter entre 2 e 1000 ocorrências.".into(),
        ));
    }
    Ok(())
}

/// Posições de um padrão em coordenadas da peça; a primeira é sempre a identidade.
pub fn pattern_placements(
    layout: &PatternLayout,
    parameters: &parameters::Parameters,
) -> Result<Vec<Placement>, DocumentError> {
    use parameters::Dimension::{Angle, Length};
    let error = |e: nator_geometry::GeometryError| DocumentError(e.to_string());
    validate_pattern(layout, parameters)?;
    match layout {
        PatternLayout::Linear { first, second } => {
            let step = |axis: &LinearAxis| -> Result<[f64; 3], DocumentError> {
                let spacing = parameters.evaluate(&axis.spacing, Length)?;
                let length = nator_geometry::norm(axis.direction);
                Ok(axis.direction.map(|v| v / length * spacing))
            };
            let a = step(first)?;
            let (b, rows) = match second {
                Some(axis) => (step(axis)?, axis.count),
                None => ([0.; 3], 1),
            };
            let mut placements = Vec::new();
            for j in 0..rows {
                for i in 0..first.count {
                    let offset = std::array::from_fn(|k| a[k] * f64::from(i) + b[k] * f64::from(j));
                    placements.push(Placement::new(offset, [0.; 3]).map_err(error)?);
                }
            }
            Ok(placements)
        }
        PatternLayout::Circular {
            axis_origin,
            axis_direction,
            count,
            angle,
        } => {
            let total = parameters.evaluate(angle, Angle)?;
            let full = (total.abs() - std::f64::consts::TAU).abs() < 1e-9;
            let step = if full {
                total / f64::from(*count)
            } else {
                total / f64::from(count - 1)
            };
            (0..*count)
                .map(|k| {
                    Placement::default()
                        .rotated_about(*axis_origin, *axis_direction, step * f64::from(k))
                        .map_err(error)
                })
                .collect()
        }
    }
}

pub(crate) fn validate_region(region: &RegionRef) -> Result<(), DocumentError> {
    if let RegionRef::Multiple(regions) = region {
        if regions.is_empty()
            || regions.len() > 1024
            || regions.iter().any(|r| matches!(r, RegionRef::Multiple(_)))
        {
            return Err(DocumentError(
                "Seleção de áreas vazia, aninhada ou excessiva.".into(),
            ));
        }
        for (i, region) in regions.iter().enumerate() {
            if regions[..i].contains(region) {
                return Err(DocumentError("Área repetida na seleção.".into()));
            }
            validate_region(region)?;
        }
    }
    if let RegionRef::Area(edges) = region
        && (edges.is_empty() || edges.len() > 8192)
    {
        return Err(DocumentError(
            "Referência de área vazia ou excessiva.".into(),
        ));
    }
    if let RegionRef::Cell { entities, .. } = region
        && (entities.is_empty() || entities.len() > nator_sketch::MAX_ENTITIES)
    {
        return Err(DocumentError("Parte de face inválida.".into()));
    }
    if let RegionRef::Boundary(ids) = region
        && (ids.is_empty() || ids.len() > nator_sketch::MAX_ENTITIES)
    {
        return Err(DocumentError(
            "Referência de região vazia ou excessiva.".into(),
        ));
    }
    Ok(())
}

/// Nomes de faces são curtos e sem NUL; vazio significa "sem nome" e não é referenciável.
/// Pontos do rascunho que posicionam furos, torres e presilhas.
fn validate_points(part: &Part, sketch: SketchId, points: &[u64]) -> Result<(), DocumentError> {
    if points.is_empty() || points.len() > MAX_OCCURRENCES {
        return Err(DocumentError(
            "A operação precisa de 1 a 1000 pontos.".into(),
        ));
    }
    let record = part.sketches.iter().find(|s| s.id == sketch);
    if points
        .iter()
        .any(|p| record.is_some_and(|r| r.sketch.point(*p).is_err()))
    {
        return Err(DocumentError(
            "Um ponto da operação não existe mais no rascunho; selecione os pontos novamente."
                .into(),
        ));
    }
    Ok(())
}

pub fn validate_face_name(name: &str) -> Result<(), DocumentError> {
    if name.is_empty() || name.len() > 256 || name.contains('\0') {
        return Err(DocumentError("Nome de face inválido.".into()));
    }
    Ok(())
}

/// Perfis/eixo precisam estar no mesmo plano geométrico, independentemente de
/// origem U/V, sinal da normal ou ocorrência escolhida na montagem.
pub fn validate_coplanar(
    a: nator_sketch::Plane,
    b: nator_sketch::Plane,
) -> Result<(), DocumentError> {
    let n = a.normal();
    if nator_geometry::norm(nator_geometry::cross(n, b.normal())) > 1e-7
        || nator_geometry::dot(nator_geometry::sub(a.origin, b.origin), n).abs() > 1e-6
    {
        return Err(DocumentError(
            "As áreas e a linha do eixo devem estar no mesmo plano.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_placements_follow_spacing_and_angles() -> Result<(), Box<dyn Error>> {
        let mut parameters = parameters::Parameters::default();
        parameters.set(
            "passo".into(),
            "12 mm".into(),
            parameters::Dimension::Length,
        )?;
        let grid = PatternLayout::Linear {
            first: LinearAxis {
                direction: [2., 0., 0.],
                count: 3,
                spacing: "passo".into(),
            },
            second: Some(LinearAxis {
                direction: [0., 1., 0.],
                count: 2,
                spacing: "20 mm".into(),
            }),
        };
        let placements = pattern_placements(&grid, &parameters)?;
        assert_eq!(placements.len(), 6);
        assert_eq!(placements[0], Placement::default());
        assert_eq!(placements[2].apply([0.; 3]), [24., 0., 0.]);
        assert_eq!(placements[5].apply([0.; 3]), [24., 20., 0.]);
        let ring = |angle: &str, count| PatternLayout::Circular {
            axis_origin: [0.; 3],
            axis_direction: [0., 0., 1.],
            count,
            angle: angle.into(),
        };
        // 360°: quatro cópias a cada 90°; 90°: três cópias com a última em 90°.
        let full = pattern_placements(&ring("360 deg", 4), &parameters)?;
        let quarter = pattern_placements(&ring("90 deg", 3), &parameters)?;
        let x = |p: &Placement| p.apply([10., 0., 0.]);
        assert!((x(&full[1])[1] - 10.).abs() < 1e-9);
        assert!((x(&quarter[2])[1] - 10.).abs() < 1e-9);
        assert!((x(&quarter[1])[0] - 10. * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        // Uma única ocorrência não é um padrão.
        assert!(pattern_placements(&ring("360 deg", 1), &parameters).is_err());
        Ok(())
    }

    /// Peça com A → B (transformação de A) → C (padrão de B) e D independente de A.
    fn chain() -> Result<Part, Box<dyn Error>> {
        let asset = Arc::new(MeshAsset::new(nator_geometry::reference_bracket()?));
        let body = |id: u64| Body {
            print_down: None,
            intrinsic_colors: Default::default(),
            id: BodyId(id),
            name: format!("Corpo {id}"),
            geometry: Arc::clone(&asset),
            brep: Some(Arc::new(vec![1])),
            face_names: Arc::default(),
            placement: Placement::default(),
            // Entradas consumidas ficam ocultas, como após criar as operações.
            visible: id > 11,
            color: None,
        };
        let transform = |base: u64| Operation::Mirror {
            base: BodyId(base),
            origin: [1., 0., 0.],
            normal: [1., 0., 0.],
            keep_original: false,
        };
        let feature = |output: u64, operation| Feature {
            name: format!("Operação {output}"),
            output: BodyId(output),
            operation,
            suppressed: false,
        };
        Ok(Part {
            id: PartId(1),
            name: "Peça".into(),
            bodies: (10..14).map(body).collect(),
            sketches: Vec::new(),
            work_planes: Vec::new(),
            parameters: parameters::Parameters::default(),
            features: vec![
                feature(11, transform(10)),
                feature(12, transform(11)),
                feature(13, transform(10)),
            ],
            history_marker: None,
            folders: PartFolders::default(),
            reference: false,
        })
    }

    #[test]
    fn listed_bodies_distinguish_history_from_manual_visibility() -> Result<(), Box<dyn Error>> {
        let mut part = chain()?;
        let mut independent = part.bodies[0].clone();
        independent.id = BodyId(14);
        independent.name = "Corpo independente".into();
        independent.visible = false;
        part.bodies.push(independent);
        let listed = |part: &Part| part.listed_bodies().map(|b| b.id).collect::<Vec<_>>();
        assert_eq!(listed(&part), [BodyId(12), BodyId(13), BodyId(14)]);
        // Ocultar um corpo atual ou a pasta não o remove da árvore: o olho deve
        // continuar disponível para reexibi-lo, sem listar todas as etapas anteriores.
        part.bodies[3].visible = false;
        part.folders.bodies = false;
        assert_eq!(listed(&part), [BodyId(12), BodyId(13), BodyId(14)]);
        assert!(part.shown_bodies().next().is_none());
        // Projetos antigos podem ter uma etapa anterior exibida explicitamente.
        // Mantê-la na lista permite ocultá-la novamente.
        part.bodies[0].visible = true;
        assert_eq!(
            listed(&part),
            [BodyId(10), BodyId(12), BodyId(13), BodyId(14)]
        );
        part.bodies[0].visible = false;

        let before = hidden_by_history(&part);
        part.history_marker = Some(1);
        sync_visibility(&mut part, &before);
        assert_eq!(listed(&part), [BodyId(11), BodyId(14)]);
        let before = hidden_by_history(&part);
        part.history_marker = Some(0);
        sync_visibility(&mut part, &before);
        assert_eq!(listed(&part), [BodyId(10), BodyId(14)]);
        Ok(())
    }

    #[test]
    fn inactive_operations_propagate_and_drive_visibility() -> Result<(), Box<dyn Error>> {
        let mut part = chain()?;
        assert_eq!(dependents(&part, BodyId(11)), [BodyId(12)]);
        let before = hidden_by_history(&part);
        assert_eq!(before, HashSet::from([BodyId(10), BodyId(11)]));
        part.features[0].suppressed = true;
        assert_eq!(
            states(&part),
            [
                FeatureState::Suppressed,
                FeatureState::Dependent,
                FeatureState::Active
            ]
        );
        // O usuário ocultou o resultado independente: a escolha é preservada.
        part.bodies[3].visible = false;
        sync_visibility(&mut part, &before);
        let shown = |part: &Part| part.bodies.iter().map(|b| b.visible).collect::<Vec<_>>();
        // A base continua consumida pela operação ativa 13; 11 e 12 ficam inativos.
        assert_eq!(shown(&part), [false, false, false, false]);
        validate(&part)?;
        // Um resultado inativo visível é rejeitado.
        part.bodies[2].visible = true;
        assert!(validate(&part).is_err());
        part.bodies[2].visible = false;
        // Marcador depois da primeira operação: as seguintes ficam fora do recálculo.
        part.features[0].suppressed = false;
        part.history_marker = Some(1);
        assert_eq!(
            states(&part),
            [
                FeatureState::Active,
                FeatureState::RolledBack,
                FeatureState::RolledBack
            ]
        );
        assert_eq!(
            inactive_outputs(&part),
            HashSet::from([BodyId(12), BodyId(13)])
        );
        // O marcador no fim não é canônico.
        part.history_marker = Some(3);
        assert!(validate(&part).is_err());
        Ok(())
    }

    #[test]
    fn cloning_moves_face_owners_to_the_new_bodies() -> Result<(), Box<dyn Error>> {
        let bodies = HashMap::from([(BodyId(12), BodyId(40)), (BodyId(3), BodyId(41))]);
        assert_eq!(remap_face_name("b12:e7", &bodies), "b40:e7");
        assert_eq!(remap_face_name("b3:e2:inner", &bodies), "b41:e2:inner");
        // Donos de fora da peça clonada e nomes de outro formato ficam intactos.
        assert_eq!(remap_face_name("b99:start", &bodies), "b99:start");
        assert_eq!(remap_face_name("box:f1", &bodies), "box:f1");
        let mut feature = Feature {
            name: "Arredondamento".into(),
            output: BodyId(3),
            operation: Operation::Fillet {
                base: BodyId(12),
                edge: EdgeRef::Between(["b12:e7".into(), "b12:end".into()]),
                radius: "2 mm".into(),
                additional_edges: vec![EdgeRef::Between(["b12:e8".into(), "b12:end".into()])],
            },
            suppressed: false,
        };
        feature.remap(&bodies, &HashMap::new())?;
        let Operation::Fillet {
            base,
            edge,
            additional_edges,
            ..
        } = feature.operation
        else {
            return Err("A operação deixou de ser um arredondamento".into());
        };
        assert_eq!(base, BodyId(40));
        assert_eq!(edge, EdgeRef::Between(["b40:e7".into(), "b40:end".into()]));
        assert_eq!(
            additional_edges,
            vec![EdgeRef::Between(["b40:e8".into(), "b40:end".into()])]
        );
        Ok(())
    }
}
