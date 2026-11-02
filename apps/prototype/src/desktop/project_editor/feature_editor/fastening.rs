//! Painéis de encaixe e fixação para impressão: recorte de componente, ressalto de tampa,
//! torres de parafuso e presilhas. Os padrões citam o perfil, para acompanhar a calibração.
use super::*;
use nator_document::features::{
    BossSpec, ComponentOutline, LipKind, LipReference, LipSpec, SnapKind, SnapSpec,
};
use nator_document::parameters::Dimension;
use nator_geometry::Placement;

/// Direções de inserção, nos eixos da peça: a primeira é entrar por cima.
const INSERTIONS: [(&str, [f64; 3]); 6] = [
    ("-Z", [0., 0., -1.]),
    ("+Z", [0., 0., 1.]),
    ("-X", [-1., 0., 0.]),
    ("+X", [1., 0., 0.]),
    ("-Y", [0., -1., 0.]),
    ("+Y", [0., 1., 0.]),
];

/// Parede da torre: quatro voltas do bico em volta do furo.
const BOSS_WALL: &str = "8 * bico";

/// Medidas como digitadas, com expressões.
pub(super) struct FasteningDraft {
    /// Corpo clicado e sua posição na montagem.
    pub component: Option<(BodyId, Placement)>,
    /// Silhueta gravada ou capturada; `None` pede nova captura.
    pub outline: Option<Result<ComponentOutline, String>>,
    pub insertion: usize,
    pub clearance: String,
    pub floor: String,
    pub through: bool,
    pub depth: String,
    pub lip: LipSpec,
    pub boss: BossSpec,
    pub snap: SnapSpec,
    /// Pontos gravados na operação em edição; mantidos enquanto o rascunho não mudar.
    pub points: Option<Vec<u64>>,
    /// Posição do fixador na tabela do perfil.
    pub fastener: usize,
}
impl Default for FasteningDraft {
    fn default() -> Self {
        Self {
            component: None,
            outline: None,
            insertion: 0,
            clearance: "folga_deslizante".into(),
            floor: "folga_justa".into(),
            through: true,
            depth: "10 mm".into(),
            lip: LipSpec {
                kind: LipKind::Lip,
                reference: LipReference::Inner,
                inset: "0 mm".into(),
                width: "parede".into(),
                height: "2 mm".into(),
                clearance: "folga_deslizante".into(),
            },
            boss: BossSpec {
                outer_diameter: "6 mm".into(),
                hole_diameter: "2.5 mm".into(),
                height: "10 mm".into(),
                hole_depth: None,
                fillet: "1 mm".into(),
                reverse: false,
            },
            snap: SnapSpec {
                kind: SnapKind::Hook,
                length: "12 mm".into(),
                thickness: "2 mm".into(),
                width: "6 mm".into(),
                overhang: "1 mm".into(),
                head: "2 mm".into(),
                fillet: "0.5 mm".into(),
                angle: "0 deg".into(),
                clearance: "folga_deslizante".into(),
                window: "3 mm".into(),
                reverse: false,
            },
            points: None,
            fastener: 2,
        }
    }
}
impl FasteningDraft {
    /// Silhueta pronta para a operação; vazia enquanto não houver captura.
    pub(super) fn captured(&self) -> ComponentOutline {
        match &self.outline {
            Some(Ok(outline)) => outline.clone(),
            _ => ComponentOutline {
                source: String::new(),
                plane: nator_sketch::Plane::default(),
                contours: Vec::new(),
            },
        }
    }
    pub(super) fn has_outline(&self) -> bool {
        matches!(&self.outline, Some(Ok(_)))
    }
    pub(super) fn keep_outline(&mut self, outline: ComponentOutline) {
        let down = outline.plane.normal().map(|v| -v);
        let same = |axis: &[f64; 3]| (0..3).all(|i| (axis[i] - down[i]).abs() < 1e-9);
        self.insertion = INSERTIONS
            .iter()
            .position(|(_, axis)| same(axis))
            .unwrap_or(0);
        self.outline = Some(Ok(outline));
    }
    /// Captura uma vez por escolha de componente e direção, não a cada quadro.
    pub(super) fn refresh_outline(&mut self, document: &Document, target: Placement) {
        if self.outline.is_some() {
            return;
        }
        if let Some((body, pose)) = self.component {
            let insertion = INSERTIONS[self.insertion].1;
            self.outline = Some(
                nator_commands::fit::capture(document, body, pose, target, insertion)
                    .map_err(|e| e.to_string()),
            );
        }
    }
    pub(super) fn load_boss(&mut self, preset: HolePreset, fastener: &Fastener) {
        let hole = preset.spec(fastener);
        self.boss.outer_diameter = format!("({}) + {BOSS_WALL}", hole.diameter);
        self.boss.hole_diameter = hole.diameter;
        self.boss.hole_depth = hole.depth;
    }
}

fn component_selection(ui: &mut egui::Ui, d: &mut Draft) {
    let source = match &d.fastening.outline {
        Some(Ok(outline)) => Some(outline.source.clone()),
        _ => None,
    };
    let s = panel::selection(
        ui,
        text(T::ComponentField),
        usize::from(source.is_some()),
        source.as_deref(),
        d.field == Some(Field::Component),
    );
    if s.activate {
        d.field = Some(Field::Component);
    }
    if s.clear {
        d.fastening.component = None;
        d.fastening.outline = None;
        d.field = Some(Field::Component);
    }
}

pub(super) fn component_panel(ui: &mut egui::Ui, d: &mut Draft, part: &nator_document::Part) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    component_selection(ui, d);
    let f = &mut d.fastening;
    if let Some(Err(error)) = &f.outline {
        ui.colored_label(theme::warning(), error);
    }
    // Sem o componente, a silhueta gravada não pode ser refeita em outra direção.
    ui.add_enabled_ui(f.component.is_some() || f.outline.is_none(), |ui| {
        let previous = f.insertion;
        panel::row(ui, text(T::InsertionField), |ui| {
            egui::ComboBox::from_id_salt("component_insertion")
                .width(70.)
                .selected_text(INSERTIONS[f.insertion].0)
                .show_ui(ui, |ui| {
                    for (index, (label, _)) in INSERTIONS.iter().enumerate() {
                        ui.selectable_value(&mut f.insertion, index, *label);
                    }
                });
        });
        if f.insertion != previous {
            f.outline = None;
        }
    });
    panel::row(ui, text(T::FitClearance), |ui| {
        panel::length_expression(ui, &mut f.clearance)
    });
    panel::row(ui, text(T::FloorClearance), |ui| {
        panel::length_expression(ui, &mut f.floor)
    });
    panel::row(ui, text(T::HoleExtent), |ui| {
        panel::choice(
            ui,
            &mut f.through,
            &[
                (false, Icon::Measure, T::ExtentDistance),
                (true, Icon::ToFace, T::Through),
            ],
        );
    });
    if !f.through {
        panel::row(ui, text(T::ComponentDepth), |ui| {
            panel::length_expression(ui, &mut f.depth)
        });
    }
    panel::hint(
        ui,
        text(if f.has_outline() {
            T::ComponentFrozen
        } else {
            T::ComponentHint
        }),
    );
}

fn face_selection(ui: &mut egui::Ui, d: &mut Draft, label: T) {
    let description = d.kept_topology.as_ref().map(describe_topology);
    let s = panel::selection(
        ui,
        text(label),
        usize::from(d.kept_topology.is_some()),
        description.as_deref(),
        d.field == Some(Field::Faces),
    );
    if s.activate {
        d.field = Some(Field::Faces);
    }
    if s.clear {
        d.kept_topology = None;
        d.field = Some(Field::Faces);
    }
}

pub(super) fn lip_panel(ui: &mut egui::Ui, d: &mut Draft) {
    face_selection(ui, d, T::LipFace);
    let lip = &mut d.fastening.lip;
    panel::row(ui, text(T::LipKindField), |ui| {
        panel::choice(
            ui,
            &mut lip.kind,
            &[
                (LipKind::Lip, Icon::Lip, T::LipRaised),
                (LipKind::Groove, Icon::Shell, T::LipGroove),
            ],
        );
    });
    panel::row(ui, text(T::LipReferenceField), |ui| {
        panel::choice(
            ui,
            &mut lip.reference,
            &[
                (LipReference::Inner, Icon::Hole, T::LipInner),
                (LipReference::Outer, Icon::Fit, T::LipOuter),
            ],
        );
    });
    panel::row(ui, text(T::LipInset), |ui| {
        panel::length_expression(ui, &mut lip.inset)
    });
    panel::row(ui, text(T::SnapWidth), |ui| {
        panel::length_expression(ui, &mut lip.width)
    });
    panel::row(ui, text(T::FastenerHeight), |ui| {
        panel::length_expression(ui, &mut lip.height)
    });
    if lip.kind == LipKind::Groove {
        panel::row(ui, text(T::FitClearance), |ui| {
            panel::length_expression(ui, &mut lip.clearance)
        });
    }
    panel::hint(ui, text(T::LipHint));
}

/// Rascunho dos pontos e quantos deles valem, como no furo.
fn sketch_points(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    record: Option<&nator_document::SketchRecord>,
) {
    panel::row(ui, text(T::Profile), |ui| {
        let previous = d.sketch;
        egui::ComboBox::from_id_salt("fastening_sketch")
            .width(170.)
            .selected_text(record.map(|s| s.sketch.name.as_str()).unwrap_or("—"))
            .show_ui(ui, |ui| {
                for s in &part.sketches {
                    ui.selectable_value(&mut d.sketch, Some(s.id), &s.sketch.name);
                }
            });
        if d.sketch != previous {
            d.fastening.points = None;
        }
    });
    let points = record.map(|r| hole_points(&r.sketch)).unwrap_or_default();
    match &d.fastening.points {
        Some(kept) => {
            ui.label(format!("{}: {}", text(T::KeptReference), kept.len()));
        }
        None if points.is_empty() => {
            ui.colored_label(theme::warning(), text(T::NoHolePoints));
        }
        None => {
            ui.label(format!("{}: {}", text(T::HolePoints), points.len()));
        }
    }
}

pub(super) fn boss_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    record: Option<&nator_document::SketchRecord>,
    fasteners: &[Fastener],
) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    sketch_points(ui, d, part, record);
    let f = &mut d.fastening;
    let mut chosen = None;
    panel::row(ui, text(T::HolePresetField), |ui| {
        egui::ComboBox::from_id_salt("boss_fastener")
            .width(60.)
            .selected_text(fasteners.get(f.fastener).map_or("—", |f| f.name.as_str()))
            .show_ui(ui, |ui| {
                for (index, fastener) in fasteners.iter().enumerate() {
                    ui.selectable_value(&mut f.fastener, index, &fastener.name);
                }
            });
        egui::ComboBox::from_id_salt("boss_preset")
            .width(110.)
            .selected_text(text(T::HolePresetChoose))
            .show_ui(ui, |ui| {
                for (preset, label) in [
                    (HolePreset::SelfTapping, T::PresetSelfTap),
                    (HolePreset::HeatInsert, T::PresetInsert),
                ] {
                    if ui.button(text(label)).clicked() {
                        chosen = Some(preset);
                    }
                }
            });
    });
    if let (Some(preset), Some(fastener)) = (chosen, fasteners.get(f.fastener)) {
        f.load_boss(preset, fastener);
    }
    let boss = &mut f.boss;
    panel::row(ui, text(T::BossOuter), |ui| {
        panel::length_expression(ui, &mut boss.outer_diameter)
    });
    panel::row(ui, text(T::HoleDiameter), |ui| {
        panel::length_expression(ui, &mut boss.hole_diameter)
    });
    panel::row(ui, text(T::FastenerHeight), |ui| {
        panel::length_expression(ui, &mut boss.height)
    });
    let mut shallow = boss.hole_depth.is_some();
    if ui
        .checkbox(&mut shallow, text(T::BossShallowHole))
        .changed()
    {
        boss.hole_depth = shallow.then(|| "6 mm".to_owned());
    }
    if let Some(depth) = &mut boss.hole_depth {
        panel::row(ui, text(T::HoleDepth), |ui| {
            panel::length_expression(ui, depth)
        });
    }
    panel::row(ui, text(T::BaseFillet), |ui| {
        panel::length_expression(ui, &mut boss.fillet)
    });
    ui.checkbox(&mut boss.reverse, text(T::ReverseDirection));
}

pub(super) fn snap_panel(
    ui: &mut egui::Ui,
    d: &mut Draft,
    part: &nator_document::Part,
    record: Option<&nator_document::SketchRecord>,
    max_strain_pct: f64,
) {
    body_selection(ui, d, part, Field::Base, T::BodyField);
    sketch_points(ui, d, part, record);
    let snap = &mut d.fastening.snap;
    panel::row(ui, text(T::LipKindField), |ui| {
        panel::choice(
            ui,
            &mut snap.kind,
            &[
                (SnapKind::Hook, Icon::SnapFit, T::SnapHook),
                (SnapKind::Window, Icon::Shell, T::SnapWindow),
            ],
        );
    });
    for (label, value) in [
        (T::SnapLength, &mut snap.length),
        (T::SnapThickness, &mut snap.thickness),
        (T::SnapWidth, &mut snap.width),
        (T::SnapOverhang, &mut snap.overhang),
        (T::SnapHead, &mut snap.head),
    ] {
        panel::row(ui, text(label), |ui| panel::length_expression(ui, value));
    }
    panel::row(ui, text(T::Angle), |ui| {
        panel::expression(ui, &mut snap.angle)
    });
    if snap.kind == SnapKind::Hook {
        panel::row(ui, text(T::BaseFillet), |ui| {
            panel::length_expression(ui, &mut snap.fillet)
        });
        let length = |e: &str| part.parameters.evaluate(e, Dimension::Length).ok();
        if let (Some(l), Some(t), Some(o)) = (
            length(&snap.length),
            length(&snap.thickness),
            length(&snap.overhang),
        ) {
            let strain = 100. * SnapSpec::strain(t, o, l);
            panel::row(ui, text(T::SnapStrain), |ui| {
                ui.label(format!("{strain:.1} %").replace('.', ","))
            });
            if strain > max_strain_pct {
                ui.colored_label(theme::warning(), text(T::SnapStrainHigh));
            }
        }
    } else {
        panel::row(ui, text(T::FitClearance), |ui| {
            panel::length_expression(ui, &mut snap.clearance)
        });
        panel::row(ui, text(T::SnapWindowDepth), |ui| {
            panel::length_expression(ui, &mut snap.window)
        });
    }
    ui.checkbox(&mut snap.reverse, text(T::ReverseDirection));
}
