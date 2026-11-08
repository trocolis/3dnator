//! Atividades por contexto. Os cartões referenciam elementos atuais, sem simular
//! reprodução de gestos de montagem/rascunho nem guardar estado de egui no arquivo.
use super::{
    icons::{self, Icon},
    *,
};
use nator_document::activity::{
    Activity, ActivityKind as K, ActivityTarget as A, EditContext as C,
};

fn icon(kind: K) -> Icon {
    match kind {
        K::Part => Icon::Part,
        K::Sketch => Icon::Sketch,
        K::Geometry => Icon::Line,
        K::Dimension => Icon::Dimension,
        K::Relation => Icon::ConstraintCoincident,
        K::Modify => Icon::Edit,
        K::Projection => Icon::SketchProject,
        K::Feature => Icon::Extrude,
        K::Paint => Icon::Paint,
        K::Move => Icon::Move,
        K::Copy => Icon::Copy,
        K::Group => Icon::Group,
        K::WorkPlane => Icon::WorkPlane,
        K::Image => Icon::Image,
    }
}
fn label(kind: K) -> T {
    match kind {
        K::Part => T::NewPartMenu,
        K::Sketch => T::NewSketch,
        K::Geometry => T::ActivityGeometry,
        K::Dimension => T::Dimension,
        K::Relation => T::ActivityRelation,
        K::Modify => T::ActivityModify,
        K::Projection => T::Projection,
        K::Feature => T::EditFeature,
        K::Paint => T::Paint,
        K::Move => T::MoveMenu,
        K::Copy => T::CopyMenu,
        K::Group => T::GroupTool,
        K::WorkPlane => T::WorkPlaneName,
        K::Image => T::SketchImage,
    }
}
impl Editor {
    pub(super) fn context_activities(&self, context: C) -> Vec<Activity> {
        let doc = self.current().session.document();
        let mut entries: Vec<_> = doc
            .activities()
            .iter()
            .filter(|a| a.context == context && self.activity_exists(&a.target))
            .cloned()
            .collect();
        let mut fallback = vec![];
        // Arquivos anteriores ao formato 30 não guardavam atividades. Elementos
        // existentes continuam acessíveis; não inventamos uma ordem histórica.
        let mut add = |kind, target| {
            if !entries.iter().any(|a| a.target == target && a.kind == kind) {
                fallback.push(Activity {
                    id: 0,
                    context,
                    kind,
                    target,
                });
            }
        };
        match context {
            C::Assembly => {
                for p in doc.parts() {
                    add(K::Part, A::Part(p.id));
                }
                for s in doc.reference_sketches() {
                    add(
                        K::Sketch,
                        A::Sketch {
                            id: s.id,
                            entities: vec![],
                            constraints: vec![],
                        },
                    );
                }
                for g in doc.groups() {
                    add(K::Group, A::Group(g.id));
                }
            }
            C::Part(id) => {
                if let Some(p) = doc.part(id) {
                    for s in &p.sketches {
                        add(
                            K::Sketch,
                            A::Sketch {
                                id: s.id,
                                entities: vec![],
                                constraints: vec![],
                            },
                        );
                    }
                    for w in &p.work_planes {
                        add(K::WorkPlane, A::WorkPlane(w.id));
                    }
                }
            }
            C::Sketch { sketch, .. } if entries.is_empty() => {
                if let Some(s) = doc.sketch(sketch) {
                    add(
                        K::Geometry,
                        A::Sketch {
                            id: sketch,
                            entities: s.sketch.entities.iter().map(|e| e.id).collect(),
                            constraints: vec![],
                        },
                    );
                }
            }
            _ => {}
        }
        fallback.append(&mut entries);
        fallback
    }
    fn activity_exists(&self, target: &A) -> bool {
        let d = self.current().session.document();
        match target {
            A::Part(p) => d.part(*p).is_some(),
            A::Sketch { id, .. } => d.sketch(*id).is_some(),
            A::Feature { part, output } => d
                .part(*part)
                .is_some_and(|p| p.features.iter().any(|f| f.output == *output)),
            A::Body(b) => d.body(*b).is_some(),
            A::Instance(i) => d.instance(*i).is_some(),
            A::Group(g) => d.groups().iter().any(|v| v.id == *g),
            A::WorkPlane(w) => d
                .parts()
                .iter()
                .any(|p| p.work_planes.iter().any(|v| v.id == *w)),
            A::Image(i) => d.images().iter().any(|v| v.id == *i),
        }
    }
    pub(super) fn activity_cards(
        &self,
        ui: &mut egui::Ui,
        entries: &[Activity],
    ) -> Option<Activity> {
        let mut chosen = None;
        for entry in entries {
            let response =
                icons::button(ui, icon(entry.kind), text(label(entry.kind)), false, false);
            if response.clicked() && !self.busy() {
                chosen = Some(entry.clone());
            }
        }
        chosen
    }
    pub(super) fn activity_strip(&mut self, ui: &mut egui::Ui, context: C) {
        self.timeline.tiles.clear();
        let entries = self.context_activities(context);
        let mut chosen = None;
        // Como a linha do tempo das operações, abre na última alteração.
        egui::ScrollArea::horizontal()
            .id_salt("context_timeline")
            .stick_to_right(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| chosen = self.activity_cards(ui, &entries));
            });
        if let Some(entry) = chosen {
            self.edit_activity(ui.ctx(), entry);
        }
    }
    pub(super) fn edit_activity(&mut self, ctx: &egui::Context, entry: Activity) {
        if self
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.pending.is_some())
        {
            return;
        }
        match entry.target {
            A::Part(p) => self.apply_menu_action(ctx, browser::MenuAction::Activate(p)),
            A::Sketch {
                id,
                entities,
                constraints,
            } => {
                let owner = self
                    .current()
                    .session
                    .document()
                    .parts()
                    .iter()
                    .find(|p| p.sketches.iter().any(|s| s.id == id))
                    .map(|p| p.id);
                if self
                    .current()
                    .sketch_edit
                    .as_ref()
                    .is_none_or(|w| w.id != id)
                {
                    self.begin_history_sketch(owner, id);
                }
                self.shell.tab = shell::RibbonTab::Sketch;
                self.select_history_elements(&entities, &constraints);
            }
            A::WorkPlane(w) => self.edit_work_plane(w),
            A::Body(b) => {
                let owner = self
                    .current()
                    .session
                    .document()
                    .parts()
                    .iter()
                    .find(|p| p.bodies.iter().any(|v| v.id == b))
                    .map(|p| p.id);
                if let Some(part) = owner {
                    self.current_mut()
                        .select(Some(Selection::Body { part, id: b }));
                    if entry.kind == K::Paint {
                        self.open_paint();
                        self.seed_paint_body(b);
                    } else {
                        self.open_body_move(entry.kind == K::Copy);
                    }
                }
            }
            A::Instance(i) => {
                self.current_mut().select(Some(Selection::Instance(i)));
                self.open_move();
            }
            A::Group(g) => {
                let members = self
                    .current()
                    .session
                    .document()
                    .groups()
                    .iter()
                    .find(|v| v.id == g)
                    .map(|v| v.members.clone())
                    .unwrap_or_default();
                self.current_mut().select_many(
                    members.into_iter().map(Selection::Instance).collect(),
                    false,
                );
                self.open_move();
            }
            A::Feature { part, output } => {
                if let Some(f) = self
                    .current()
                    .session
                    .document()
                    .part(part)
                    .and_then(|p| p.features.iter().find(|f| f.output == output))
                    .cloned()
                {
                    self.edit_cad_feature(part, f);
                }
            }
            A::Image(i) => {
                if let Some(s) = self
                    .current()
                    .session
                    .document()
                    .images()
                    .iter()
                    .find(|v| v.id == i)
                    .map(|v| v.sketch)
                {
                    let owner = self
                        .current()
                        .session
                        .document()
                        .parts()
                        .iter()
                        .find(|p| p.sketches.iter().any(|v| v.id == s))
                        .map(|p| p.id);
                    self.begin_history_sketch(owner, s);
                }
            }
        }
        self.invalidate_scene();
        ctx.request_repaint();
    }
}
