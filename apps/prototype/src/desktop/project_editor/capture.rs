//! Captura a mesma composição visual usada na janela, sem exigir servidor gráfico.
use super::*;
use std::{error::Error, path::Path};

pub(crate) fn render_project(project: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    render_project_at_size(project, output, [1280, 820], false)
}

pub(crate) fn render_project_at_size(
    project: &Path,
    output: &Path,
    size: [u32; 2],
    sketch_mode: bool,
) -> Result<(), Box<dyn Error>> {
    render_with(
        project,
        output,
        size,
        |editor| {
            if sketch_mode {
                open_first_sketch(editor)?;
            }
            Ok(())
        },
        |_, _| Vec::new(),
        |_| true,
    )
}

/// Capturas de estados da interface usados na revisão visual: linha do tempo, painéis de
/// comando com prévia e decisão sobre falhas de recálculo. Não alteram o projeto de origem.
pub(crate) fn render_ui_scenario(
    project: &Path,
    output: &Path,
    size: [u32; 2],
    scenario: &str,
) -> Result<(), Box<dyn Error>> {
    match scenario {
        "timeline"
        | "timeline-menu"
        | "extrude"
        | "edit-last"
        | "failure"
        | "cell-hover"
        | "paint-hover"
        | "hover-face"
        | "hover-edge"
        | "sketch-plane-hover"
        | "select-body"
        | "window"
        | "assembly-select"
        | "move"
        | "align"
        | "assembly-ribbon"
        | "section"
        | "offset-plane"
        | "work-plane-pick"
        | "offset-plane-edit"
        | "sketch-dimension"
        | "sketch-mirror"
        | "sketch-offset"
        | "sketch-fillet"
        | "sketch-chamfer"
        | "paint"
        | "sketch-image"
        | "repair"
        | "recovery"
        | "insert"
        | "mesh-import"
        | "menu-new-part"
        | "menu-move"
        | "menu-copy"
        | "reference-align"
        | "reference-sketch"
        | "copy-beside"
        | "tree-folders"
        | "part-menu-move"
        | "measure"
        | "sketch-line-input"
        | "sketch-construction-input"
        | "sketch-infinite-input"
        | "sketch-circle-input"
        | "sketch-rectangle-input"
        | "extrude-zero"
        | "extrude-face"
        | "status-dialog"
        | "extrude-cut" => {}
        _ => {
            return Err(
                "Cenários: timeline, timeline-menu, extrude, edit-last, failure, hover-face, \
                 hover-edge, sketch-plane-hover, select-body, window, assembly-select, move, align, assembly-ribbon, section, offset-plane, work-plane-pick, offset-plane-edit, sketch-dimension, sketch-mirror, sketch-offset, sketch-fillet, sketch-chamfer, sketch-image, paint, repair, recovery, insert, mesh-import, menu-new-part, menu-move, menu-copy, reference-align, reference-sketch, copy-beside, tree-folders, part-menu-move, measure, sketch-line-input, sketch-construction-input, sketch-infinite-input, sketch-circle-input, sketch-rectangle-input, extrude-face, extrude-cut, extrude-zero, status-dialog."
                    .into(),
            );
        }
    }
    let menu = scenario == "timeline-menu";
    let pointer = scenario.to_owned();
    render_with(
        project,
        output,
        size,
        |editor| {
            scenario_setup(editor, scenario)?;
            // A janela de recuperação lista o próprio projeto, com sua miniatura.
            if scenario == "recovery" {
                editor.recoveries = vec![project.to_path_buf()];
                editor.show_recovery = true;
            }
            Ok(())
        },
        move |editor, frame| {
            let canvas = editor.canvas_rect;
            let target = if matches!(pointer.as_str(), "cell-hover" | "paint-hover") {
                first_cell_target(editor)
                    .and_then(|point| {
                        sketch_scene::project(
                            &editor.viewport.camera,
                            editor.display_bounds(),
                            canvas,
                            point,
                        )
                    })
                    .unwrap_or(canvas.center())
            } else {
                canvas.center() + egui::vec2(-60., 40.)
            };
            if pointer.starts_with("sketch-") && pointer.ends_with("-input") {
                let start = canvas.center() + egui::vec2(-180., 100.);
                let end = start + egui::vec2(140., -75.);
                return match frame {
                    2 | 3 => vec![
                        egui::Event::PointerMoved(start),
                        egui::Event::PointerButton {
                            pos: start,
                            button: egui::PointerButton::Primary,
                            pressed: frame == 2,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    4 => vec![egui::Event::PointerMoved(end)],
                    5 => vec![egui::Event::Text(
                        if pointer == "sketch-circle-input" {
                            "12,5"
                        } else if pointer == "sketch-infinite-input" {
                            "30"
                        } else {
                            "40"
                        }
                        .into(),
                    )],
                    6 if pointer != "sketch-circle-input" && pointer != "sketch-infinite-input" => {
                        vec![egui::Event::Key {
                            key: egui::Key::Tab,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    }
                    7 if pointer != "sketch-circle-input" && pointer != "sketch-infinite-input" => {
                        vec![egui::Event::Text("30".into())]
                    }
                    _ => Vec::new(),
                };
            }
            // Cota: clique no meio da primeira linha do rascunho e cursor afastado dela,
            // com a caixa de valor e a prévia acompanhando o ponteiro.
            if pointer == "sketch-dimension"
                && let Some(middle) = first_line_middle(editor)
            {
                return match frame {
                    2 | 3 => vec![
                        egui::Event::PointerMoved(middle),
                        egui::Event::PointerButton {
                            pos: middle,
                            button: egui::PointerButton::Primary,
                            pressed: frame == 2,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    _ => vec![egui::Event::PointerMoved(middle + egui::vec2(60., -80.))],
                };
            }
            // Menus da faixa: clique no botão (Nova peça, Mover, Copiar) e cursor sobre a
            // primeira subopção.
            let ribbon_menu = match pointer.as_str() {
                "menu-new-part" => Some(0),
                "part-menu-move" => Some(0),
                "menu-move" => Some(1),
                "menu-copy" => Some(2),
                _ => None,
            };
            if let Some(index) = ribbon_menu
                && let Some(rect) = editor.shell.menu_rects.get(index)
                && (frame == 2 || frame == 3)
            {
                return vec![
                    egui::Event::PointerMoved(rect.center()),
                    egui::Event::PointerButton {
                        pos: rect.center(),
                        button: egui::PointerButton::Primary,
                        pressed: frame == 2,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
            }
            match pointer.as_str() {
                "cell-hover" | "paint-hover" | "hover-face" | "hover-edge"
                | "sketch-plane-hover" | "work-plane-pick"
                    if frame >= 2 =>
                {
                    return vec![egui::Event::PointerMoved(target)];
                }
                // Janela da direita para a esquerda, mantida aberta para a captura.
                "window" if frame >= 2 => {
                    let start = canvas.center() + egui::vec2(160., 110.);
                    let mut events = vec![egui::Event::PointerMoved(if frame == 2 {
                        start
                    } else {
                        canvas.center() + egui::vec2(-90., -60.)
                    })];
                    if frame == 2 {
                        events.push(egui::Event::PointerButton {
                            pos: start,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                    return events;
                }
                _ => {}
            }
            // Clique secundário na primeira operação abre o menu de contexto.
            match editor.timeline.tiles.first() {
                Some((_, rect)) if menu && (frame == 2 || frame == 3) => {
                    let position = rect.center();
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Secondary,
                            pressed: frame == 2,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]
                }
                _ => Vec::new(),
            }
        },
        |editor| {
            if matches!(
                scenario,
                "sketch-mirror" | "sketch-offset" | "sketch-fillet" | "sketch-chamfer"
            ) {
                return editor
                    .current()
                    .sketch_edit
                    .as_ref()
                    .is_some_and(|w| w.operation_ready());
            }
            if matches!(scenario, "cell-hover" | "paint-hover") {
                return editor.hover.as_ref().is_some_and(|h| !h.overlay.is_empty());
            }
            if scenario == "sketch-plane-hover" {
                return editor.hover.as_ref().is_some_and(|h| !h.overlay.is_empty());
            }
            if scenario == "extrude-zero" {
                return editor.preview_idle()
                    && editor.feature_edit.as_ref().is_some_and(|d| {
                        editor
                            .current()
                            .session
                            .document()
                            .part(d.part())
                            .is_some_and(|p| d.profile_plane(p).is_some())
                    });
            }
            (editor.feature_edit.is_none() && editor.assembly_panel.is_none())
                || editor.preview.shown != 0
                || (editor.assembly_panel.is_some() && editor.preview_idle())
                || {
                    let command = editor.feature_edit.as_ref().and_then(|d| {
                        editor
                            .current()
                            .session
                            .document()
                            .part(d.part())
                            .map(|p| d.preview_key(p))
                    });
                    command.is_some_and(|c| {
                        matches!(
                            editor.preview_status(&c),
                            live_preview::PreviewStatus::Failed(_)
                        )
                    })
                }
        },
    )
}

/// Meio da primeira linha finita do rascunho em edição, na tela.
fn first_line_middle(editor: &Editor) -> Option<egui::Pos2> {
    let work = editor.current().sketch_edit.as_ref()?;
    let sketch = editor.working_sketch()?;
    let (a, b) = sketch
        .entities
        .iter()
        .filter(|e| !e.infinite && !e.construction)
        .find_map(|e| sketch.line(e.id).ok())?;
    let frame = sketch_scene::Frame {
        camera: editor.viewport.camera.clone(),
        bounds: editor.display_bounds(),
        rect: editor.canvas_rect,
        placement: work.placement,
        plane: sketch.plane,
    };
    frame.screen(nator_sketch::scale(nator_sketch::add(a, b), 0.5))
}

/// Um ponto interno evita que a captura caia justamente sobre um contorno/furo.
/// Este preparo pertence ao executor de capturas, fora do fluxo interativo normal.
fn first_cell_target(editor: &Editor) -> Option<Point3> {
    let part = editor.current().active_part?;
    let piece = editor.current().session.document().part(part)?;
    let placement = sketch_scene::part_placement(editor.current(), part).ok()?;
    for record in piece.sketches.iter().filter(|s| piece.shows_sketch(s)) {
        let regions = nator_sketch::areas::regions(&record.sketch).ok()?;
        for region in regions {
            let points = region.outer.sample(32);
            let center = nator_sketch::scale(
                points.iter().fold([0.; 2], |a, p| nator_sketch::add(a, *p)),
                1. / points.len().max(1) as f64,
            );
            for point in points.iter().map(|p| {
                nator_sketch::add(
                    nator_sketch::scale(*p, 0.9),
                    nator_sketch::scale(center, 0.1),
                )
            }) {
                if region.outer.contains(point) && !region.holes.iter().any(|h| h.contains(point)) {
                    return Some(placement.apply(record.sketch.plane.world(point)));
                }
            }
        }
    }
    None
}

fn open_first_sketch(editor: &mut Editor) -> Result<(), Box<dyn Error>> {
    let sketch = editor
        .current()
        .session
        .document()
        .parts()
        .iter()
        .find_map(|p| p.sketches.first().map(|s| (p.id, s.id, s.sketch.clone())));
    if let Some((part, id, sketch)) = sketch {
        let report = nator_sketch::solver::solve(&sketch)?;
        editor.begin_sketch(part, id);
        if let Some(work) = editor.current_mut().sketch_edit.as_mut() {
            work.status = report.status;
            work.redundant = report.redundant;
        }
    }
    Ok(())
}

fn scenario_setup(editor: &mut Editor, scenario: &str) -> Result<(), Box<dyn Error>> {
    if matches!(scenario, "paint" | "paint-hover") {
        editor.shell.tab = shell::RibbonTab::Model;
        editor.open_paint();
        return Ok(());
    }
    if scenario == "sketch-image" {
        let id = editor
            .current()
            .session
            .document()
            .images()
            .first()
            .ok_or("Projeto sem imagem")?
            .id;
        editor.edit_reference_image(id);
        return Ok(());
    }
    if matches!(
        scenario,
        "sketch-mirror" | "sketch-offset" | "sketch-fillet" | "sketch-chamfer"
    ) {
        use nator_sketch::{Curve, edit::Action};
        open_first_sketch(editor)?;
        let source = editor.working_sketch().ok_or("Rascunho ausente")?;
        let first = source
            .entities
            .iter()
            .find(|e| matches!(e.curve, Curve::Line { .. }) && !e.infinite)
            .ok_or("Rascunho sem linhas")?
            .id;
        let selected = nator_sketch::connected::chain(&source, first)?;
        let action = match scenario {
            "sketch-mirror" => Action::Mirror,
            "sketch-offset" => Action::Offset,
            "sketch-fillet" => Action::Fillet,
            "sketch-chamfer" => Action::Chamfer,
            _ => return Err("Cenário de rascunho desconhecido".into()),
        };
        let axis = source
            .entities
            .iter()
            .find(|e| !selected.contains(&e.id) && matches!(e.curve, Curve::Line { .. }))
            .map(|e| e.id);
        if let Some(work) = editor.current_mut().sketch_edit.as_mut() {
            work.configure_operation_capture(action, selected, axis);
        }
        return Ok(());
    }
    let tab = editor.current().id;
    let part = editor
        .current()
        .active_part
        .or_else(|| {
            let document = editor.current().session.document();
            document
                .parts()
                .iter()
                .find(|p| !p.bodies.is_empty())
                .map(|p| p.id)
        })
        .ok_or("O projeto não tem peça para o cenário.")?;
    let features = editor
        .current()
        .session
        .document()
        .part(part)
        .map(|p| p.features.clone())
        .unwrap_or_default();
    match scenario {
        "status-dialog" => editor.status.error(text(T::Failed)),
        "extrude-zero" | "extrude-face" | "extrude-cut" => {
            use nator_document::features::{FaceRef, Operation};
            let doc = editor.current().session.document();
            let base = doc
                .part(part)
                .and_then(|p| p.shown_bodies().next())
                .ok_or("Corpo ausente")?;
            let face = base
                .face_names
                .iter()
                .filter(|name| !name.is_empty())
                .filter_map(|name| {
                    let face = FaceRef::Named(name.clone());
                    let operation = Operation::ExtrudeFace {
                        base: base.id,
                        face: face.clone(),
                        from: "0 mm".into(),
                        to: "3 mm".into(),
                        combine: None,
                        until: None,
                    };
                    let guide = nator_commands::extrusion::guide(doc, part, &operation).ok()?;
                    (guide.normal[2] > 0.5).then_some((face, guide.origin[2]))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|p| p.0)
                .ok_or("Corpo sem face superior plana")?;
            let base = base.id;
            editor.open_feature_tool(feature_editor::Tool::Extrude);
            if let Some(d) = editor.feature_edit.as_mut() {
                d.choose_face_profile(base, face);
                d.set_size_for_capture(if scenario == "extrude-zero" {
                    "0 mm"
                } else if scenario == "extrude-cut" {
                    "-3 mm"
                } else {
                    "6 mm"
                });
            }
        }
        "sketch-line-input"
        | "sketch-circle-input"
        | "sketch-rectangle-input"
        | "sketch-construction-input"
        | "sketch-infinite-input" => {
            open_first_sketch(editor)?;
            let work = editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Rascunho ausente")?;
            work.select_tool(match scenario {
                "sketch-circle-input" => sketch_editor::Tool::Circle,
                "sketch-rectangle-input" => sketch_editor::Tool::Rectangle,
                _ => sketch_editor::Tool::Line,
            });
            work.line_type = match scenario {
                "sketch-construction-input" => nator_sketch::LineType::Construction,
                "sketch-infinite-input" => nator_sketch::LineType::Infinite,
                _ => nator_sketch::LineType::Normal,
            };
        }
        "measure" => {
            // A primeira face da malha fornece uma seleção real, inclusive nos
            // sólidos com mapa triângulo → face do B-rep para medição exata.
            let document = editor.current().session.document();
            let occurrence = document
                .occurrences()?
                .into_iter()
                .find(|o| {
                    document
                        .instance(o.instance)
                        .is_some_and(|i| i.part == part)
                })
                .ok_or("Peça sem ocorrência visível")?;
            let body = document
                .part(part)
                .and_then(|p| p.shown_bodies().next())
                .ok_or("Peça sem corpo visível")?;
            let mesh = body.geometry.mesh();
            let triangle = *mesh.triangles().first().ok_or("Corpo sem triângulos")?;
            let pose = occurrence.placement.compose(body.placement)?;
            let vertices = mesh.triangle_points(triangle).map(|p| pose.apply(p));
            let normal = nator_geometry::cross(
                nator_geometry::sub(vertices[1], vertices[0]),
                nator_geometry::sub(vertices[2], vertices[0]),
            );
            let length = nator_geometry::norm(normal);
            editor.current_mut().picked = Some(nator_document::assembly::Pick {
                instance: occurrence.instance,
                part,
                body: body.id,
                triangle: 0,
                point: std::array::from_fn(|i| {
                    (vertices[0][i] + vertices[1][i] + vertices[2][i]) / 3.
                }),
                normal: normal.map(|v| v / length),
                vertices,
                placement: occurrence.placement,
            });
            editor.pick_mode = navigation::PickMode::Face;
            editor.shell.measure_open = true;
            editor.measure_current_selection(&egui::Context::default());
        }
        "timeline" | "timeline-menu" => {
            // Marcador antes da última operação e a segunda suprimida, se houver.
            if features.len() >= 2 {
                editor
                    .current_mut()
                    .session
                    .execute(Command::SetHistoryMarker {
                        part,
                        marker: Some(features.len() - 1),
                    })?;
            }
            if features.len() >= 3 {
                editor
                    .current_mut()
                    .session
                    .execute(Command::SetFeatureSuppressed {
                        part,
                        output: features[1].output,
                        suppressed: true,
                    })?;
            }
            let state = editor.current().session.state();
            editor.current_mut().session.mark_saved(state);
            editor.current_mut().changed();
            if let Some(first) = features.first() {
                editor.timeline.selected = Some((tab, part, first.output));
            }
        }
        "extrude" => {
            editor.open_feature_tool(feature_editor::Tool::Extrude);
            let document = editor.current().session.document().clone();
            let record = document
                .part(part)
                .and_then(|p| p.sketches.first())
                .ok_or("Peça sem rascunho para o perfil.")?;
            let region = nator_sketch::profiles::regions(&record.sketch)?
                .into_iter()
                .next()
                .ok_or("Rascunho sem região fechada.")?;
            if let Some(draft) = editor.feature_edit.as_mut() {
                draft.choose_region(
                    record.id,
                    nator_document::features::RegionRef::Boundary(region.boundary()),
                );
                draft.set_size_for_capture("6 mm");
            }
        }
        "edit-last" => {
            let feature = features.last().ok_or("Peça sem operações.")?.clone();
            editor.feature_edit = Some(feature_editor::Draft::edit(tab, part, feature));
        }
        "cell-hover" | "hover-face" | "hover-edge" | "sketch-plane-hover" | "window" => {
            if scenario == "cell-hover" {
                editor.shell.tab = shell::RibbonTab::Model;
            }
            editor.current_mut().select(None);
            editor.pick_mode = if scenario == "hover-edge" {
                navigation::PickMode::Edge
            } else {
                navigation::PickMode::Face
            };
            if scenario == "sketch-plane-hover" {
                editor.shell.tab = shell::RibbonTab::Model;
                editor.start_sketch_plane();
            }
        }
        "select-body" => {
            let body = editor
                .current()
                .session
                .document()
                .part(part)
                .and_then(|p| p.bodies.iter().find(|b| b.visible))
                .map(|b| b.id)
                .ok_or("Peça sem corpo visível.")?;
            editor
                .current_mut()
                .select(Some(Selection::Body { part, id: body }));
        }
        "mesh-import" => {
            // STL em polegadas: o suporte de referência dividido por 25,4.
            let mesh = reference_bracket()?;
            let inches = Mesh::new(
                mesh.vertices()
                    .iter()
                    .map(|p| p.map(|v| v / 25.4))
                    .collect(),
                mesh.triangles().to_vec(),
            )?;
            editor.mesh_import = Some(mesh_import::MeshImportDraft::new(
                tab,
                editor.current().session.revision(),
                None,
                "suporte-polegadas".into(),
                inches,
                1.,
            ));
        }
        "insert" => {
            // Do projeto auxiliar, inserir uma peça do projeto capturado.
            let source = editor.current().id;
            editor.switch(1);
            editor.shell.tab = shell::RibbonTab::Assembly;
            editor.open_insert_part();
            if let Some(draft) = editor.insert_part.as_mut() {
                draft.choose_for_capture(source, part);
            }
        }
        "repair" => {
            // Rascunho pendente com uma duplicata e uma folga de 0,3 mm, fora do perfil.
            let (id, mut sketch) = editor
                .current()
                .session
                .document()
                .part(part)
                .and_then(|p| p.sketches.first())
                .map(|s| (s.id, s.sketch.clone()))
                .ok_or("Peça sem rascunho.")?;
            editor.begin_sketch(part, id);
            let first = sketch
                .entities
                .iter()
                .find(|e| matches!(e.curve, nator_sketch::Curve::Line { .. }))
                .map(|e| e.id)
                .ok_or("Rascunho sem linhas.")?;
            let (a, b) = sketch.line(first)?;
            sketch.add_line(a, b, false)?;
            sketch.add_line([80., 0.], [90., 0.], false)?;
            sketch.add_line([90.3, 0.], [90., 10.], false)?;
            if let Some(work) = editor.current_mut().sketch_edit.as_mut() {
                work.pending = Some(sketch);
            }
            editor.open_sketch_repair();
        }
        "section" => {
            editor.current_mut().select(None);
            let middle = editor
                .current()
                .session
                .document()
                .scene_mesh()?
                .map(|m| m.bounds().center()[1])
                .unwrap_or(0.);
            editor.open_section();
            if let Some(section) = &mut editor.current_mut().section {
                // XZ pelo meio da largura: atravessa o furo do suporte de referência e
                // remove a metade da frente, deixando a tampa voltada para a câmera.
                section.base = section::Base::Principal(1);
                section.offset = middle;
                section.flip = true;
            }
        }
        "offset-plane" => {
            // Primeira peça em construção, plano deslocado 20 mm acima de XY: painel,
            // contorno do plano-base, plano novo e seta de arraste na mesma captura.
            let part = editor
                .current()
                .session
                .document()
                .parts()
                .first()
                .map(|p| p.id)
                .ok_or("Projeto sem peças.")?;
            editor.current_mut().select(None);
            editor.current_mut().active_part = Some(part);
            editor.shell.tab = shell::RibbonTab::Model;
            editor.open_offset_plane();
            editor.choose_origin_plane("XY", nator_sketch::Plane::xy());
            editor.set_offset_plane_for_capture(20.);
        }
        "sketch-dimension" => {
            open_first_sketch(editor)?;
            editor.select_sketch_tool(sketch_editor::Tool::Dimension);
        }
        "offset-plane-edit" => {
            // Plano existente a 20 mm de XY reaberto pela edição e levado a 35 mm: o
            // painel mostra o nome e o plano-base atual; a prévia substitui o plano gravado.
            let part = editor
                .current()
                .session
                .document()
                .parts()
                .first()
                .map(|p| p.id)
                .ok_or("Projeto sem peças.")?;
            editor.current_mut().select(None);
            editor.current_mut().active_part = Some(part);
            editor.shell.tab = shell::RibbonTab::Model;
            editor.execute(Command::AddWorkPlane {
                part,
                name: format!("{} 1", text(T::WorkPlaneName)),
                plane: nator_document::WorkPlane {
                    base: nator_sketch::Plane::xy(),
                    offset: 20.,
                },
            });
            let plane = editor
                .current()
                .session
                .document()
                .part(part)
                .and_then(|p| p.work_planes.last())
                .map(|w| w.id)
                .ok_or("Plano ausente.")?;
            editor.edit_work_plane(plane);
            editor.set_offset_plane_for_capture(35.);
        }
        "work-plane-pick" => {
            // Plano de trabalho 20 mm acima de XY na primeira peça, já na escolha de suporte
            // do Novo rascunho: o plano aparece clicável junto aos planos da origem.
            let part = editor
                .current()
                .session
                .document()
                .parts()
                .first()
                .map(|p| p.id)
                .ok_or("Projeto sem peças.")?;
            editor.current_mut().select(None);
            editor.current_mut().active_part = Some(part);
            editor.shell.tab = shell::RibbonTab::Model;
            editor.execute(Command::AddWorkPlane {
                part,
                name: format!("{} 1", text(T::WorkPlaneName)),
                plane: nator_document::WorkPlane {
                    base: nator_sketch::Plane::xy(),
                    offset: 20.,
                },
            });
            editor.start_sketch_plane();
        }
        "menu-new-part" | "menu-move" | "menu-copy" => {
            editor.current_mut().active_part = None;
            let instance = editor
                .current()
                .session
                .document()
                .instances()
                .iter()
                .find(|i| i.parent.is_none())
                .map(|i| i.id);
            editor
                .current_mut()
                .select(instance.map(Selection::Instance));
        }
        "part-menu-move" => {
            editor.current_mut().active_part = Some(part);
            editor.shell.tab = shell::RibbonTab::Model;
        }
        "tree-folders" => {
            // Peça da raiz em edição, com a origem dela visível e a pasta de rascunhos
            // oculta: a árvore esmaece o que a pasta esconde e a cena acompanha.
            let document = editor.current().session.document();
            let root = document
                .instances()
                .iter()
                .find(|i| i.parent.is_none())
                .and_then(|i| document.part(i.part))
                .ok_or("Projeto sem instâncias.")?;
            let (root, sketches) = (root.id, !root.sketches.is_empty());
            editor.apply_menu_action(
                &egui::Context::default(),
                browser::MenuAction::Activate(root),
            );
            editor.execute(Command::SetFolderVisibility {
                folder: nator_document::Folder::PartOrigin(root),
                visible: true,
            });
            if sketches {
                editor.execute(Command::SetFolderVisibility {
                    folder: nator_document::Folder::Sketches(root),
                    visible: false,
                });
            }
        }
        "reference-sketch" => {
            // Rascunho de referência aberto para edição, sobre a montagem.
            editor.current_mut().active_part = None;
            let mut layout =
                nator_sketch::Sketch::new(text(T::ReferenceSketch), nator_sketch::Plane::xy());
            layout.add_rectangle([-40., -30.], [120., 60.], false)?;
            layout.add_point([0., 0.])?;
            let Effect::ReferenceSketchCreated { sketch } = editor
                .current_mut()
                .session
                .execute(Command::AddReferenceSketch { sketch: layout })?
            else {
                return Err("Rascunho de referência ausente.".into());
            };
            let state = editor.current().session.state();
            editor.current_mut().session.mark_saved(state);
            editor.current_mut().changed();
            editor.begin_reference_sketch(sketch);
        }
        "reference-align" => {
            // Rascunho de referência com uma linha e pontos de destino no plano XY.
            editor.current_mut().active_part = None;
            let mut layout =
                nator_sketch::Sketch::new(text(T::ReferenceSketch), nator_sketch::Plane::xy());
            layout.add_line([-60., -40.], [60., -40.], false)?;
            layout.add_point([-60., 30.])?;
            editor
                .current_mut()
                .session
                .execute(Command::AddReferenceSketch { sketch: layout })?;
            let state = editor.current().session.state();
            editor.current_mut().session.mark_saved(state);
            editor.current_mut().changed();
            editor.open_align(navigation::PickMode::Vertex);
            let document = editor.current().session.document().clone();
            let occurrences = document.occurrences()?;
            let occurrence = occurrences
                .iter()
                .find(|o| {
                    document
                        .instance(o.instance)
                        .and_then(|i| document.part(i.part))
                        .is_some_and(|p| !p.bodies.is_empty())
                })
                .ok_or("Sem componentes com corpos.")?;
            let part = document
                .instance(occurrence.instance)
                .and_then(|i| document.part(i.part))
                .ok_or("Peça ausente")?;
            let body = &part.bodies[0];
            let corner = occurrence
                .placement
                .apply(body.placement.apply(body.geometry.mesh().bounds().min));
            editor.set_align_for_capture(
                nator_document::assembly::Pick {
                    instance: occurrence.instance,
                    part: part.id,
                    body: body.id,
                    triangle: 0,
                    point: corner,
                    normal: [0., 0., 1.],
                    vertices: [corner; 3],
                    placement: occurrence.placement,
                },
                [-60., 30., 0.],
            );
        }
        "move" | "align" | "assembly-ribbon" | "copy-beside" => {
            editor.current_mut().active_part = None;
            let document = editor.current().session.document();
            // Uma instância com corpos visíveis, a mais interna disponível.
            let instance = document
                .instances()
                .iter()
                .rev()
                .find(|i| document.part(i.part).is_some_and(|p| !p.bodies.is_empty()))
                .map(|i| i.id)
                .ok_or("Projeto sem instâncias com corpos.")?;
            editor
                .current_mut()
                .select(Some(Selection::Instance(instance)));
            editor.shell.tab = shell::RibbonTab::Assembly;
            match scenario {
                "move" => {
                    editor.open_move();
                    editor.set_move_for_capture([25., 0., 10.], [0., 0., 30.]);
                }
                "align" => editor.open_align(navigation::PickMode::Vertex),
                "copy-beside" => {
                    // Nova instância e cópia independente: ambas ao lado da original.
                    editor.execute(Command::DuplicateInstance { id: instance });
                    editor.execute(Command::CloneInstance {
                        id: instance,
                        name: text(T::CopyIndependent).into(),
                    });
                    editor
                        .current_mut()
                        .select(Some(Selection::Instance(instance)));
                }
                _ => {}
            }
        }
        "assembly-select" => {
            editor.current_mut().active_part = None;
            let instance = editor
                .current()
                .session
                .document()
                .instances()
                .iter()
                .find(|i| i.parent.is_none())
                .map(|i| i.id)
                .ok_or("Projeto sem instâncias.")?;
            editor
                .current_mut()
                .select(Some(Selection::Instance(instance)));
        }
        "failure" => {
            // Uma edição que quebra a última operação abre a decisão do usuário.
            let feature = features.last().ok_or("Peça sem operações.")?.clone();
            let mut draft = feature_editor::Draft::edit(tab, part, feature);
            draft.set_size_for_capture("80 mm");
            let piece = editor
                .current()
                .session
                .document()
                .part(part)
                .ok_or("Peça ausente")?
                .clone();
            let command = draft.preview_key(&piece);
            let revision = editor.current().session.revision();
            match PreparedCommand::new(
                editor.current().session.document().clone(),
                revision,
                command.clone(),
            ) {
                Err(error) if error.failure.is_some() => {
                    editor.reject_command(tab, command, error);
                }
                _ => return Err("A edição de teste não falhou no recálculo.".into()),
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_with(
    project: &Path,
    output: &Path,
    size: [u32; 2],
    setup: impl FnOnce(&mut Editor) -> Result<(), Box<dyn Error>>,
    events: impl Fn(&Editor, usize) -> Vec<egui::Event>,
    ready: impl Fn(&Editor) -> bool,
) -> Result<(), Box<dyn Error>> {
    if !(900..=3840).contains(&size[0]) || !(600..=2160).contains(&size[1]) {
        return Err("A captura aceita largura de 900 a 3840 e altura de 600 a 2160 pixels.".into());
    }
    let document = read_project(project)?;
    let state = gpu_state()?;
    let device = state.device.clone();
    let queue = state.queue.clone();
    let info = state.adapter.get_info();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let context = egui::Context::default();
    let mut editor =
        Editor::from_gpu(state.clone(), &context, document).map_err(|e| e.to_string())?;
    // Mostrar duas abas exercita a disposição real, sem alterar o arquivo de origem.
    editor.add_tab(Document::new(text(T::AuxiliaryProject))?, None, None)?;
    editor.switch(0);
    editor.recoveries.clear();
    editor.show_recovery = false;
    if let Some(part) = editor.current().session.document().parts().first()
        && let Some(body) = part
            .bodies
            .iter()
            .find(|b| b.visible)
            .or_else(|| part.bodies.first())
    {
        let selection = Selection::Body {
            part: part.id,
            id: body.id,
        };
        let part = part.id;
        editor.current_mut().select(Some(selection));
        editor.current_mut().active_part = Some(part);
    }
    setup(&mut editor)?;
    // Preparar a cena após escolher o contexto mantém o esmaecimento e a revisão
    // iguais aos da janela, sem iniciar uma reconstrução assíncrona durante a captura.
    let scene = prepare_scene(
        editor.current().session.document(),
        &editor.current().view,
        editor.current().active_part,
    )?;
    editor.scene = match scene {
        Some((mesh, prepared)) => {
            editor.viewport.set_prepared_mesh(&device, &prepared)?;
            editor.current_mut().scene_bounds = Some(mesh.bounds());
            Some(mesh)
        }
        None => None,
    };
    if editor.current().scene_bounds.is_none() {
        editor.current_mut().scene_bounds = Some(sketch_scene::document_bounds(
            editor.current().session.document(),
            &editor.current().view,
        ));
    }
    editor.current_mut().camera_transition = None;
    if let Some(work) = editor.current_mut().sketch_edit.as_mut() {
        work.fit_requested = true;
    }
    editor.rendered = Some(editor.scene_key());
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Tela do projeto"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let started = Instant::now();
    let deadline = started + Duration::from_secs(20);
    let mut frame = 0;
    let mut settled = 0;
    // Pelo menos três quadros; depois, até o cenário ficar pronto e estável o bastante para
    // concluir as animações de abertura de menus e diálogos.
    while frame < 3 || settled < 8 {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
        let input = egui::RawInput {
            screen_rect: Some(rect),
            time: Some(started.elapsed().as_secs_f64()),
            events: events(&editor, frame),
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| editor.draw(ui));
        let shapes = context.tessellate(output.shapes, output.pixels_per_point);
        let descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: output.pixels_per_point,
        };
        let mut renderer = state.renderer.write();
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&device, &queue, *id, delta);
            }
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        let commands = renderer.update_buffers(&device, &queue, &mut encoder, &shapes, &descriptor);
        {
            let view = texture.create_view(&Default::default());
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            renderer.render(&mut pass.forget_lifetime(), &shapes, &descriptor);
        }
        queue.submit(commands.into_iter().chain([encoder.finish()]));
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        // As alterações já foram aplicadas; informar isso antes de liberar o FullOutput.
        output.textures_delta.clear();
        drop(renderer);
        frame += 1;
        if frame >= 3 && ready(&editor) && !editor.busy() {
            settled += 1;
        } else {
            settled = 0;
        }
        if Instant::now() > deadline {
            return Err(format!("A captura não estabilizou: {}", editor.status).into());
        }
        if frame >= 3 && settled < 8 {
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    let image = crate::preview::read_texture(&device, &queue, &texture)?;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(format!("Erro de validação na captura da interface: {error}").into());
    }
    image.write_png(output)?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"preview":output,"app_version":env!("CARGO_PKG_VERSION"),"adapter":info.name,"backend":format!("{:?}",info.backend),"width":image.width,"height":image.height})
        )?
    );
    Ok(())
}

fn gpu_state() -> Result<egui_wgpu::RenderState, Box<dyn Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let renderer =
        egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, Default::default());
    let state = egui_wgpu::RenderState {
        adapter,
        available_adapters: Vec::new(),
        instance,
        device: device.clone(),
        queue: queue.clone(),
        target_format: wgpu::TextureFormat::Rgba8Unorm,
        renderer: Arc::new(egui::epaint::mutex::RwLock::new(renderer)),
        surface_config: egui_wgpu::SurfaceConfig::LOW_LATENCY,
    };
    Ok(state)
}

#[cfg(all(test, feature = "occt"))]
mod tests {
    use super::*;
    use nator_document::features::{Operation, RegionRef};
    use nator_sketch::{Plane, Sketch};

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn compact_footer_and_status_dialog_preserve_messages_and_document()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Interface compacta")?)?;
        let mut sketch = Sketch::new("Base", Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
            destination: None,
            sketch,
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        session.execute(Command::AddFeature {
            part,
            name: "Base".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "5 mm".into(),
                combine: None,
                until: None,
            },
        })?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.show_recovery = false;
        editor.recoveries.clear();
        editor.current_mut().active_part = Some(part);
        settle(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        // Este ensaio usa o desenho real e eventos de confirmação, sem a
        // leitura automática de avisos usada pelos ensaios das outras ferramentas.
        fn draw(
            editor: &mut Editor,
            context: &egui::Context,
            width: f32,
            events: Vec<egui::Event>,
        ) -> Vec<(String, egui::Rect)> {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 820.),
                    )),
                    time: Some(context.input(|i| i.time) + 1. / 60.),
                    events,
                    ..Default::default()
                },
                |ui| editor.draw(ui),
            );
            let labels = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => {
                        Some((t.galley.job.text.clone(), t.visual_bounding_rect()))
                    }
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
            labels
        }
        for width in [1280., 900.] {
            let labels = draw(&mut editor, &context, width, vec![]);
            // A câmera agora é controlada pelo cubo e por seu menu; a faixa
            // permanente de vistas/projeção não deve voltar ao topo do canvas.
            assert!(
                !labels
                    .iter()
                    .any(|(label, _)| label == text(T::Orthographic)
                        || label == text(T::Perspective)
                        || label == text(T::Fit))
            );
            assert!(editor.canvas_rect.top() < 210.);
            assert!(!labels.iter().any(|(label, _)| label == text(T::Project) || label.contains(text(T::UnitsMm))));
            let history = labels
                .iter()
                .find(|(label, _)| label == text(T::ParametricHistory))
                .ok_or("Histórico ausente")?
                .1;
            let ready = labels
                .iter()
                .find(|(label, _)| label == text(T::Ready))
                .ok_or("Status ausente")?
                .1;
            assert!(history.top() > 775. && ready.top() > 775.);
            assert!((history.center().y - ready.center().y).abs() < 6.);
            assert!(history.right() < ready.left());
            assert!(!editor.timeline.tiles.is_empty());
            assert!(
                editor
                    .timeline
                    .tiles
                    .iter()
                    .all(|(_, rect)| rect.top() > 775. && rect.right() < ready.left())
            );
        }
        editor.status.set("Projeto salvo de teste.");
        let labels = draw(&mut editor, &context, 1280., vec![]);
        assert!(!editor.status.has_pending());
        assert!(!labels.iter().any(|(label, _)| label == text(T::Ok)));
        assert!(
            labels
                .iter()
                .any(|(label, rect)| label == "Projeto salvo de teste." && rect.top() > 775.)
        );
        let warning = "Confira a operação de teste: esta mensagem longa deve permanecer na barra de status, em uma linha, sem cobrir o histórico nem abrir um diálogo.";
        editor.status.warning(warning);
        editor.status.set(text(T::Ready));
        let labels = draw(&mut editor, &context, 900., vec![]);
        assert!(!editor.status.has_pending());
        assert!(!labels.iter().any(|(label, _)| label == text(T::Ok)));
        let status = labels
            .iter()
            .find(|(label, _)| label == warning)
            .ok_or("Aviso ausente no status")?
            .1;
        assert!(status.top() > 775. && status.right() < 900.);
        assert!(
            editor
                .timeline
                .tiles
                .iter()
                .all(|(_, rect)| rect.right() < status.left())
        );
        editor.status.error("Falha de importação de teste.");
        editor.status.error("Arquivo STEP inválido de teste.");
        for _ in 0..3 {
            draw(&mut editor, &context, 1280., vec![]);
        }
        let labels = draw(&mut editor, &context, 1280., vec![]);
        assert!(
            labels
                .iter()
                .any(|(label, _)| label == "Falha de importação de teste.")
        );
        assert!(
            !labels
                .iter()
                .any(|(label, _)| label == "Arquivo STEP inválido de teste.")
        );
        let ok = labels
            .iter()
            .find(|(label, _)| label == text(T::Ok))
            .ok_or("Confirmação ausente")?
            .1
            .center();
        for pressed in [true, false] {
            draw(
                &mut editor,
                &context,
                1280.,
                vec![
                    egui::Event::PointerMoved(ok),
                    egui::Event::PointerButton {
                        pos: ok,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let labels = draw(&mut editor, &context, 1280., vec![]);
        assert!(
            labels
                .iter()
                .any(|(label, _)| label == "Arquivo STEP inválido de teste.")
        );
        assert!(
            !labels
                .iter()
                .any(|(label, _)| label == "Falha de importação de teste.")
        );
        draw(
            &mut editor,
            &context,
            1280.,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!editor.status.has_pending());
        assert_eq!(editor.current().session.revision(), revision);
        assert!(
            editor
                .current()
                .session
                .document()
                .folder_visible(nator_document::Folder::Project)?
        );
        // Enter confirma a leitura do aviso, sem confirmar o comando Mover que
        // está aberto por trás dele. A prévia precisa estar pronta para que a
        // regressão detecte uma confirmação acidental do documento.
        let body = editor
            .current()
            .session
            .document()
            .part(part)
            .and_then(|p| p.shown_bodies().next())
            .ok_or("Corpo ausente")?
            .id;
        editor
            .current_mut()
            .select(Some(Selection::Body { part, id: body }));
        editor.open_body_move(false);
        editor.set_move_for_capture([5., 0., 0.], [0.; 3]);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            draw(&mut editor, &context, 1280., vec![]);
            if editor.preview.shown != 0 && editor.preview_idle() {
                break;
            }
            if Instant::now() > deadline {
                return Err("Prévia do movimento não concluiu".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        editor.status.error("Confirme a leitura antes de mover.");
        for _ in 0..3 {
            draw(&mut editor, &context, 1280., vec![]);
        }
        draw(
            &mut editor,
            &context,
            1280.,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!editor.status.has_pending());
        assert!(editor.assembly_panel.is_some());
        assert_eq!(editor.current().session.revision(), revision);
        editor.status.error("Aviso novo no mesmo quadro do Enter.");
        draw(
            &mut editor,
            &context,
            1280.,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            editor.status.has_pending(),
            "O Enter que originou o aviso não pode fechá-lo"
        );
        assert!(editor.assembly_panel.is_some());
        assert_eq!(editor.current().session.revision(), revision);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn point_alignment_uses_edge_midpoints_circle_quadrants_and_arc_centers()
    -> Result<(), Box<dyn Error>> {
        use nator_geometry::{norm, sub};
        use nator_kernel::Solid;
        use nator_sketch::{Curve, areas};
        let mut curved = Sketch::new("Arco", Plane::xy());
        let center = curved.add_point([0., 0.])?;
        let start = curved.add_point([10., 0.])?;
        let end = curved.add_point([0., 10.])?;
        curved.add_curve(
            Curve::Arc {
                center,
                start,
                end,
                clockwise: false,
            },
            false,
        )?;
        curved.add_curve(
            Curve::Line {
                start: end,
                end: start,
            },
            false,
        )?;
        let region = areas::regions(&curved)?
            .into_iter()
            .next()
            .ok_or("Região ausente")?;
        let mut session = Session::new(Document::new("Apoios ponto a ponto")?)?;
        for (name, mut solid) in [
            ("Bloco", Solid::cuboid([0.; 3], [20., 20., 4.])?),
            ("Disco", Solid::cylinder([0.; 3], 16., 8.)?),
            ("Arco", Solid::extrude(&curved, &region, 0., 2., "arc")?),
        ] {
            let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
            session.execute(Command::ImportSolid {
                destination: None,
                name: name.into(),
                geometry: Arc::new(MeshAsset::with_faces(mesh, faces)?),
                brep: Arc::new(solid.to_brep()?),
                faces: solid.face_count()?,
            })?;
        }
        let context = egui::Context::default();
        let mut editor = Editor::from_gpu(gpu_state()?, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = None;
        editor.viewport.camera.set_direction([0., 0., 1.]);
        settle(&mut editor, &context)?;
        let instances = editor
            .current()
            .session
            .document()
            .instances()
            .iter()
            .map(|i| (i.id, i.placement))
            .collect::<Vec<_>>();
        let radial = 10. / 2_f64.sqrt();
        for (from, source, to, destination, source_label, destination_label) in [
            (
                0,
                [10., 0., 4.],
                1,
                [0., 16., 8.],
                T::EdgeMidpoint,
                T::CircleBoundaryPoint,
            ),
            (
                2,
                [0., 0., 2.],
                0,
                [10., 0., 4.],
                T::VirtualArcCenter,
                T::EdgeMidpoint,
            ),
            (
                1,
                [0., 16., 8.],
                2,
                [radial, radial, 2.],
                T::CircleBoundaryPoint,
                T::EdgeMidpoint,
            ),
        ] {
            editor.open_align(navigation::PickMode::Vertex);
            frame(&mut editor, &context, Vec::new())?;
            let revision = editor.current().session.revision();
            let bounds = editor.current().scene_bounds.ok_or("Limites ausentes")?;
            let source_world = instances[from].1.apply(source);
            let target_world = instances[to].1.apply(destination);
            for (point, label) in [
                (source_world, source_label),
                (target_world, destination_label),
            ] {
                let position = sketch_scene::project(
                    &editor.viewport.camera,
                    bounds,
                    editor.canvas_rect,
                    point,
                )
                .ok_or("Apoio fora da tela")?
                    + egui::vec2(2., 2.);
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    frame(
                        &mut editor,
                        &context,
                        vec![egui::Event::PointerMoved(position)],
                    )?;
                    if editor.hover.as_ref().is_some_and(|h| {
                        h.label == text(label)
                            && h.overlay.points.iter().any(|p| norm(sub(*p, point)) < 1e-8)
                    }) {
                        break;
                    }
                    if Instant::now() > deadline {
                        return Err(format!(
                            "Apoio {} não realçado: {}",
                            text(label),
                            editor.status
                        )
                        .into());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                click(&mut editor, &context, position)?;
            }
            assert_eq!(editor.current().session.revision(), revision);
            frame(
                &mut editor,
                &context,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            )?;
            settle(&mut editor, &context)?;
            assert!(editor.assembly_panel.is_none(), "{}", editor.status);
            let moved = |editor: &Editor| {
                editor
                    .current()
                    .session
                    .document()
                    .instance(instances[from].0)
                    .map(|i| i.placement.apply(source))
            };
            assert!(norm(sub(moved(&editor).ok_or("Origem ausente")?, target_world)) < 1e-8);
            assert!(editor.current_mut().session.undo());
            assert!(norm(sub(moved(&editor).ok_or("Origem ausente")?, source_world)) < 1e-8);
            assert!(editor.current_mut().session.redo());
            assert!(norm(sub(moved(&editor).ok_or("Origem ausente")?, target_world)) < 1e-8);
            assert!(editor.current_mut().session.undo());
            editor.current_mut().changed();
            editor.invalidate_scene();
            settle(&mut editor, &context)?;
        }
        Ok(())
    }

    /// O clique usa o mesmo centro mostrado no vazio, na origem e no destino.
    /// O resultado só entra no documento ao confirmar e mantém desfazer/refazer.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn point_to_point_captures_washer_holes_and_disk_centers() -> Result<(), Box<dyn Error>> {
        use nator_geometry::{norm, sub};
        use nator_kernel::Solid;
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Captura de centros")?)?;
        let washer =
            Solid::cylinder([0.; 3], 20., 4.)?.cut(&Solid::cylinder([0., 0., -1.], 10., 6.)?)?;
        let disk = Solid::cylinder([0.; 3], 16., 8.)?;
        for (name, mut solid) in [("Arruela", washer), ("Disco", disk)] {
            let (mesh, triangle_faces) = solid.tessellate_with_faces(0.1, 0.2)?;
            session.execute(Command::ImportSolid {
                destination: None,
                name: name.into(),
                geometry: Arc::new(MeshAsset::with_faces(mesh, triangle_faces)?),
                brep: Arc::new(solid.to_brep()?),
                faces: solid.face_count()?,
            })?;
        }
        let document = session.document().clone();
        let from_instance = document.instances()[0].id;
        let to_instance = document.instances()[1].id;
        let from = document
            .instance(from_instance)
            .ok_or("Origem ausente")?
            .placement
            .apply([0., 0., 4.]);
        let to = document
            .instance(to_instance)
            .ok_or("Destino ausente")?
            .placement
            .apply([0., 0., 8.]);
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = None;
        editor.viewport.camera.set_direction([0., 0., 1.]);
        settle(&mut editor, &context)?;
        editor.open_align(navigation::PickMode::Vertex);
        frame(&mut editor, &context, Vec::new())?;
        let revision = editor.current().session.revision();
        let bounds = editor.current().scene_bounds.ok_or("Limites ausentes")?;
        let source_screen =
            sketch_scene::project(&editor.viewport.camera, bounds, editor.canvas_rect, from)
                .ok_or("Origem fora da tela")?;
        // Fora do raio do centro e dos quadrantes da borda interna: aqui
        // exercitamos a descoberta distante, antes da atração de um apoio.
        let discovery = source_screen + egui::vec2(16., 16.);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            frame(
                &mut editor,
                &context,
                vec![egui::Event::PointerMoved(discovery)],
            )?;
            if editor
                .hover
                .as_ref()
                .is_some_and(|h| h.overlay.points.iter().any(|p| norm(sub(*p, from)) < 1e-9))
            {
                break;
            }
            if Instant::now() > deadline {
                return Err(format!("Centro não realçado: {}; cursor={discovery:?}, canvas={:?}, from={from:?}, mode={:?}, hover={:?}", editor.status, editor.canvas_rect, editor.alignment_pick_mode(), editor.hover.as_ref().map(|h| (&h.label, &h.overlay.points))).into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            editor.hover.as_ref().ok_or("Realce ausente")?.label,
            text(T::VirtualCenter)
        );
        let source_capture = source_screen + egui::vec2(8., 0.);
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                events: vec![egui::Event::PointerMoved(source_capture)],
                time: Some(context.input(|i| i.time) + 1. / 60.),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::None);
        output.textures_delta.clear();
        click(&mut editor, &context, source_capture)?;
        let target_screen =
            sketch_scene::project(&editor.viewport.camera, bounds, editor.canvas_rect, to)
                .ok_or("Destino fora da tela")?;
        let target_capture = target_screen + egui::vec2(-8., 0.);
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(target_capture)],
        )?;
        assert!(
            editor.hover.as_ref().is_some_and(|h| h
                .overlay
                .points
                .iter()
                .any(|p| norm(sub(*p, to)) < 1e-9))
        );
        click(&mut editor, &context, target_capture)?;
        assert_eq!(editor.current().session.revision(), revision);
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        settle(&mut editor, &context)?;
        assert!(editor.assembly_panel.is_none(), "{}", editor.status);
        let moved = editor
            .current()
            .session
            .document()
            .instance(from_instance)
            .ok_or("Origem movida ausente")?
            .placement;
        assert!(norm(sub(moved.apply([0., 0., 4.]), to)) < 1e-9);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .instance(to_instance)
                .ok_or("Destino ausente")?
                .placement
                .translation_mm(),
            [to[0], to[1], 0.]
        );
        assert!(editor.current_mut().session.undo());
        assert!(
            norm(sub(
                editor
                    .current()
                    .session
                    .document()
                    .instance(from_instance)
                    .ok_or("Origem ausente")?
                    .placement
                    .apply([0., 0., 4.]),
                from
            )) < 1e-9
        );
        assert!(editor.current_mut().session.redo());
        assert!(
            norm(sub(
                editor
                    .current()
                    .session
                    .document()
                    .instance(from_instance)
                    .ok_or("Origem ausente")?
                    .placement
                    .apply([0., 0., 4.]),
                to
            )) < 1e-9
        );
        Ok(())
    }

    /// Cliques Ctrl reais (sem o modificador Command) selecionam vários alvos;
    /// Mover recebe todos, Esc não altera o documento e os corpos também alimentam
    /// operações da aba Peça sem exigir nova seleção dentro do painel.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn selection_ctrl_move_and_feature_targets_follow_the_active_tab() -> Result<(), Box<dyn Error>>
    {
        use nator_kernel::Solid;
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Seleção e alvos")?)?;
        let mut parts = Vec::new();
        let mut bodies = Vec::new();
        for index in 0..3 {
            let mut solid =
                Solid::cuboid(if index == 2 { [35., 0., 0.] } else { [0.; 3] }, [20.; 3])?;
            let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
            let Effect::Imported { part, body } = session.execute(Command::ImportSolid {
                destination: (index == 2).then(|| parts[0]),
                name: format!("Bloco {index}"),
                geometry: Arc::new(MeshAsset::with_faces(mesh, faces)?),
                brep: Arc::new(solid.to_brep()?),
                faces: solid.face_count()?,
            })?
            else {
                return Err("Corpo não criado".into());
            };
            parts.push(part);
            bodies.push(body);
        }
        let mut document = session.document().clone();
        let instances = document
            .instances()
            .iter()
            .map(|instance| instance.id)
            .collect::<Vec<_>>();
        for (index, instance) in instances.iter().enumerate() {
            document.instance_mut(*instance)?.placement =
                Placement::new([if index == 0 { -50. } else { 80. }, 0., 0.], [0.; 3])?;
        }
        let mut editor = Editor::from_gpu(gpu_state()?, &context, document)
            .map_err(|error| error.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.viewport.camera.set_direction([0., 0., 1.]);
        settle(&mut editor, &context)?;
        let select = label_position(&mut editor, &context, text(T::Select))?;
        assert!(select.x < 100.);
        click(&mut editor, &context, select)?;
        assert!(editor.selection_active());
        let position = |editor: &Editor, point| -> Result<egui::Pos2, Box<dyn Error>> {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or_else(|| "Ponto sem projeção".into())
        };
        let a = position(&editor, [-40., 10., 20.])?;
        let b = position(&editor, [90., 10., 20.])?;
        click(&mut editor, &context, a)?;
        settle(&mut editor, &context)?;
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().selected.len(), 2);
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().selected.len(), 1);
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        settle(&mut editor, &context)?;
        let open_move = |editor: &mut Editor| -> Result<(), Box<dyn Error>> {
            let button = label_position(editor, &context, text(T::MoveMenu))?;
            click(editor, &context, button)?;
            let free = label_position(editor, &context, text(T::MoveFree))?;
            click(editor, &context, free)
        };
        let revision = editor.current().session.revision();
        open_move(&mut editor)?;
        assert!(!editor.selection_active());
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        settle(&mut editor, &context)?;
        assert!(editor.selection_active());
        assert_eq!(editor.current().selected.len(), 2);
        assert_eq!(editor.current().session.revision(), revision);
        open_move(&mut editor)?;
        editor.set_move_for_capture([0., 15., 0.], [0.; 3]);
        frame(&mut editor, &context, Vec::new())?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        for instance in &instances {
            assert!(
                (editor
                    .current()
                    .session
                    .document()
                    .instance(*instance)
                    .ok_or("Instância ausente")?
                    .placement
                    .translation_mm()[1]
                    - 15.)
                    .abs()
                    < 1e-6
            );
        }
        editor.current_mut().active_part = Some(parts[0]);
        editor.current_mut().select(None);
        editor.shell.tab = shell::RibbonTab::Model;
        editor.invalidate_scene();
        settle(&mut editor, &context)?;
        let select = label_position(&mut editor, &context, text(T::Select))?;
        assert!(select.x < 100.);
        let a = position(&editor, [-40., 25., 20.])?;
        let b = position(&editor, [-5., 25., 20.])?;
        click(&mut editor, &context, a)?;
        settle(&mut editor, &context)?;
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().selected.len(), 2);
        editor.open_feature_tool(feature_editor::Tool::Join);
        let part = editor
            .current()
            .session
            .document()
            .part(parts[0])
            .ok_or("Peça ausente")?;
        let command = editor
            .feature_edit
            .as_ref()
            .ok_or("Painel ausente")?
            .preview_key(part);
        assert!(
            matches!(command, Command::AddFeature { operation: Operation::Boolean { base, tool, .. }, .. } if base == bodies[0] && tool == bodies[2])
        );
        editor.activate_selection(&context);
        editor.pick_mode = navigation::PickMode::Edge;
        editor.open_feature_tool(feature_editor::Tool::Fillet);
        editor.prefill_selected_topology(&context);
        settle(&mut editor, &context)?;
        let draft = editor
            .feature_edit
            .as_ref()
            .ok_or("Arredondamento ausente")?;
        assert_eq!(draft.base(), Some(bodies[2]));
        assert_eq!(draft.references().count(), 1);
        editor.activate_selection(&context);
        assert!(editor.selection_active());
        Ok(())
    }

    /// A seleção do rascunho alimenta Cota e restrições, conserva Ctrl como
    /// alternância e retorna à seta de seleção após Esc durante uma prévia.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_selection_ctrl_prefills_dimensions_and_constraints() -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut document = Document::new("Seleção do rascunho")?;
        let mut sketch = Sketch::new("Linhas", Plane::xy());
        let first = sketch.add_line([-40., 0.], [-20., 0.], false)?;
        let second = sketch.add_line([-40., 20.], [-10., 20.], false)?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, document)
            .map_err(|error| error.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let select = label_position(&mut editor, &context, text(T::Select))?;
        assert!(select.x < 100.);
        click(&mut editor, &context, select)?;
        let position = |editor: &Editor, point| -> Result<egui::Pos2, Box<dyn Error>> {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or_else(|| "Ponto sem projeção".into())
        };
        let a = position(&editor, [-30., 0., 0.])?;
        let b = position(&editor, [-25., 20., 0.])?;
        click(&mut editor, &context, a)?;
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        editor.select_sketch_tool(sketch_editor::Tool::Dimension);
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .dimension_state()
                .1,
            1
        );
        editor.activate_selection(&context);
        click_modified(&mut editor, &context, b, egui::Modifiers::CTRL)?;
        editor.select_sketch_tool(sketch_editor::Tool::Dimension);
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .dimension_state()
                .1,
            2
        );
        frame(&mut editor, &context, Vec::new())?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert!(editor.selection_active());
        editor.open_sketch_action(&context, nator_sketch::edit::Action::Equal);
        frame(&mut editor, &context, Vec::new())?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert!(sketch.constraints.iter().any(|constraint| matches!(constraint.kind, nator_sketch::ConstraintKind::EqualLength { a, b } if a == first && b == second)));
        let line = label_position(&mut editor, &context, text(T::Line))?;
        click(&mut editor, &context, line)?;
        let start = position(&editor, [-35., -10., 0.])?;
        click(&mut editor, &context, start)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert!(editor.selection_active());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            2
        );
        Ok(())
    }

    /// Trocar de aba não deve deslocar o início do canvas. A comparação inclui
    /// a galeria de restrições e os menus do rascunho, em duas larguras de janela.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn ribbon_tabs_keep_the_same_compact_height_in_wide_and_narrow_windows()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut document = Document::new("Faixas uniformes")?;
        let mut sketch = Sketch::new("Perfil", Plane::xy());
        sketch.add_rectangle([-20., -10.], [20., 10.], false)?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, document)
            .map_err(|error| error.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        settle(&mut editor, &context)?;
        for size in [egui::vec2(1440., 900.), egui::vec2(900., 600.)] {
            let mut tops = Vec::new();
            for tab in [
                shell::RibbonTab::Assembly,
                shell::RibbonTab::Model,
                shell::RibbonTab::Inspect,
                shell::RibbonTab::Sketch,
            ] {
                editor.shell.tab = tab;
                for _ in 0..3 {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            time: Some(context.input(|input| input.time) + 0.1),
                            ..Default::default()
                        },
                        |ui| editor.draw(ui),
                    );
                    output.textures_delta.clear();
                }
                tops.push(editor.canvas_rect.top());
            }
            assert!(
                tops.iter().all(|top| (*top - tops[0]).abs() < 1.),
                "{size:?}: {tops:?}"
            );
            // O cabeçalho anterior terminava em 188–206 pontos. A versão
            // compacta precisa recuperar espaço para o canvas em todas as abas.
            assert!(tops[0] < 180., "{size:?}: {tops:?}");
        }
        Ok(())
    }

    /// Ensaio optativo porque precisa de adaptador wgpu. Usa eventos reais egui
    /// sobre o canvas completo; não exige compositor nem janela nativa.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_support_hover_and_tool_panels_follow_explicit_choices() -> Result<(), Box<dyn Error>>
    {
        use sketch_editor::Tool;
        let context = egui::Context::default();
        let mut profile = Sketch::new("Base", Plane::xy());
        profile.add_rectangle([-50., -50.], [50., 50.], false)?;
        let mut session = Session::new(Document::new("Painéis do rascunho")?)?;
        let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
            destination: None,
            sketch: profile,
        })?
        else {
            return Err("Rascunho base ausente".into());
        };
        session.execute(Command::AddFeature {
            part,
            name: "Placa".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "10 mm".into(),
                combine: None,
                until: None,
            },
        })?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        editor.shell.tab = shell::RibbonTab::Model;
        editor.viewport.camera.set_direction([0., 0., 1.]);
        editor.current_mut().camera = editor.viewport.camera.clone();
        frame(&mut editor, &context, Vec::new())?;
        settle(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        let button = label_position(&mut editor, &context, text(T::NewSketch))?;
        click(&mut editor, &context, button)?;
        assert!(editor.current().plane_pick.is_some());
        let target = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [-25., -20., 10.],
        )
        .ok_or("Face sem projeção")?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            frame(
                &mut editor,
                &context,
                vec![egui::Event::PointerMoved(target)],
            )?;
            if editor.hover.as_ref().is_some_and(|h| !h.overlay.is_empty()) {
                break;
            }
            if Instant::now() > deadline {
                return Err("Face apta não foi realçada".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(editor.current().session.revision(), revision);
        let hover = editor.hover.as_ref().ok_or("Realce ausente")?;
        assert_eq!(hover.overlay.triangles.len(), 2);
        assert!(
            hover
                .overlay
                .triangles
                .iter()
                .flatten()
                .all(|p| (p[2] - 10.).abs() < 1e-8)
        );
        let cube = view_cube::rect(editor.canvas_rect).center();
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(cube)])?;
        assert!(editor.hover.is_none());
        click(&mut editor, &context, target)?;
        settle(&mut editor, &context)?;
        editor.align_sketch_camera(false, false);
        frame(&mut editor, &context, Vec::new())?;
        assert!(editor.current().sketch_edit.is_some());
        assert!(editor.current().plane_pick.is_none());
        assert!(label_position(&mut editor, &context, text(T::Constraints)).is_err());
        assert!(label_position(&mut editor, &context, text(T::SketchLineType)).is_err());
        let revision = editor.current().session.revision();
        for tool in [
            Tool::Line,
            Tool::Rectangle,
            Tool::Circle,
            Tool::Arc,
            Tool::Trim,
        ] {
            let button = label_position(
                &mut editor,
                &context,
                text(if tool == Tool::Arc {
                    T::ArcShort
                } else {
                    tool.label()
                }),
            )?;
            click(&mut editor, &context, button)?;
            assert_eq!(
                editor.current().sketch_edit.as_ref().map(|w| w.tool()),
                Some(tool)
            );
            label_position(
                &mut editor,
                &context,
                text(if tool == Tool::Trim {
                    T::Trim
                } else {
                    T::SketchLineType
                }),
            )?;
            if tool != Tool::Trim {
                label_position(&mut editor, &context, text(T::SketchMeasurements))?;
            }
            for hint in [
                T::SketchContinuousLineHint,
                T::SketchRectangleNormalHint,
                T::SketchCircleNormalHint,
            ] {
                assert!(label_position(&mut editor, &context, text(hint)).is_err());
            }
            assert!(label_position(&mut editor, &context, text(T::Constraints)).is_err());
        }
        // Selecionar conserva os diagnósticos e a lista de restrições que antes
        // abriam automaticamente; trocar de ferramenta não edita o documento.
        editor
            .current_mut()
            .sketch_edit
            .as_mut()
            .ok_or("Editor ausente")?
            .select_tool(Tool::Select);
        frame(&mut editor, &context, Vec::new())?;
        label_position(&mut editor, &context, text(T::Constraints))?;
        let button = label_position(&mut editor, &context, text(T::OffsetField))?;
        click(&mut editor, &context, button)?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.active_action().is_some())
        );
        assert!(label_position(&mut editor, &context, text(T::Constraints)).is_err());
        let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
        click(&mut editor, &context, cancel)?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.active_action().is_none())
        );
        assert!(label_position(&mut editor, &context, text(T::Constraints)).is_err());
        assert_eq!(editor.current().session.revision(), revision);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_workflow_keeps_the_model_and_the_same_viewport() -> Result<(), Box<dyn Error>> {
        use nator_geometry::{norm, sub};
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut base = Sketch::new("Base", Plane::xy());
        base.add_rectangle([0., 0.], [20., 15.], false)?;
        let mut session = Session::new(Document::new("Fluxo do viewport")?)?;
        let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
            destination: None,
            sketch: base,
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        session.execute(Command::AddFeature {
            part,
            name: "Base".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "8 mm".into(),
                combine: None,
                until: None,
            },
        })?;
        let mut editor = Editor::from_gpu(state, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        let (mesh, prepared) = prepare_scene(
            editor.current().session.document(),
            &editor.current().view,
            Some(part),
        )?
        .ok_or("Malha ausente")?;
        editor
            .viewport
            .set_prepared_mesh(&editor.gpu.device, &prepared)?;
        editor.current_mut().scene_bounds = Some(mesh.bounds());
        editor.scene = Some(mesh);
        editor.viewport.camera.set_direction([0., 0., 1.]);
        editor.current_mut().camera = editor.viewport.camera.clone();
        editor.rendered = Some(editor.scene_key());
        let texture = editor.texture;
        frame(&mut editor, &context, Vec::new())?;
        let revision = editor.current().session.revision();
        editor.start_sketch_plane();
        assert_eq!(revision, editor.current().session.revision());
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert!(editor.current().plane_pick.is_none());
        assert_eq!(revision, editor.current().session.revision());
        editor.start_sketch_plane();
        let rect = frame(&mut editor, &context, Vec::new())?;
        let point = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            rect,
            [5., 5., 0.],
        )
        .ok_or("Plano sem projeção")?;
        click(&mut editor, &context, point)?;
        settle(&mut editor, &context)?;
        let work = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Clique não abriu a edição")?;
        let id = work.id;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .plane,
            Plane::xy()
        );
        editor.align_sketch_camera(false, false);
        let rect = frame(&mut editor, &context, Vec::new())?;
        let work = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Edição ausente")?;
        let projection = sketch_scene::Frame {
            camera: editor.viewport.camera.clone(),
            bounds: editor.display_bounds(),
            rect,
            placement: work.placement,
            plane: Plane::xy(),
        };
        editor
            .current_mut()
            .sketch_edit
            .as_mut()
            .ok_or("Edição ausente")?
            .select_tool(sketch_editor::Tool::Rectangle);
        for point in [[2., 2.], [12., 10.]] {
            click(
                &mut editor,
                &context,
                projection.screen(point).ok_or("Ponto sem projeção")?,
            )?;
        }
        settle(&mut editor, &context)?;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            4
        );
        assert_eq!(editor.texture, texture);
        assert!(editor.scene.is_some());
        // Enquadrar abaixo de Home usa o contexto do rascunho, sem substituí-lo
        // pelo enquadramento dos sólidos visíveis ou alterar o documento.
        editor.align_sketch_camera(true, false);
        let expected = editor.viewport.camera.clone();
        editor
            .viewport
            .camera
            .pan([90., -50.], f64::from(editor.canvas_rect.height()));
        editor.viewport.camera.span *= 0.3;
        let revision = editor.current().session.revision();
        let fit = view_cube::rect(editor.canvas_rect).min + egui::vec2(17., 47.) * 0.8;
        click(&mut editor, &context, fit)?;
        frame(&mut editor, &context, Vec::new())?;
        assert_eq!(editor.current().session.revision(), revision);
        assert!(
            norm(sub(
                editor.viewport.camera.direction(),
                expected.direction()
            )) < 1e-12
        );
        assert!((editor.viewport.camera.span - expected.span).abs() < 1e-12);
        let bounds = editor.display_bounds();
        let aspect = f64::from(editor.canvas_rect.aspect_ratio());
        assert!(
            norm(sub(
                editor
                    .viewport
                    .camera
                    .world_on_target_plane([0.; 2], bounds, aspect),
                expected.world_on_target_plane([0.; 2], bounds, aspect)
            )) < 1e-9
        );
        editor.finish_sketch();
        assert!(editor.current().sketch_edit.is_none());
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible
        );
        frame(&mut editor, &context, Vec::new())?;
        editor.execute(Command::AddFeature {
            part,
            name: "Perfil consumido".into(),
            operation: Operation::Extrude {
                sketch: id,
                region: RegionRef::Index(0),
                from: "8 mm".into(),
                to: "12 mm".into(),
                combine: None,
                until: None,
            },
        });
        assert!(
            !editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible,
            "{}",
            editor.status
        );
        editor.execute(Command::SetSketchVisibility { id, visible: true });
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .visible
        );
        assert_eq!(editor.texture, texture);
        settle(&mut editor, &context)?;
        // Um clique fora dos planos da origem escolhe a face do sólido, sem
        // aproximá-la pela normal de um triângulo e sem trocar de canvas. O ponto fica
        // longe do canto superior direito, onde estão o cubo e o giro da vista.
        editor.start_sketch_plane();
        let rect = frame(&mut editor, &context, Vec::new())?;
        let point = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            rect,
            [16., 3., 8.],
        )
        .ok_or("Face sem projeção")?;
        click(&mut editor, &context, point)?;
        settle(&mut editor, &context)?;
        let work = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Clique na face não abriu a edição")?;
        let plane = editor
            .current()
            .session
            .document()
            .sketch(work.id)
            .ok_or("Rascunho ausente")?
            .sketch
            .plane;
        assert!(nator_geometry::norm(nator_geometry::sub(plane.origin, [0., 0., 8.])) < 1e-8);
        assert!(nator_geometry::dot(plane.normal(), [0., 0., 1.]) > 1. - 1e-8);
        assert_eq!(editor.texture, texture);
        assert!(editor.scene.is_some());
        Ok(())
    }
    /// Digitar no campo X do Mover e confirmar com Enter: o primeiro Enter só conclui o
    /// valor; o segundo aplica o movimento com o valor digitado.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn move_panel_values_can_be_typed() -> Result<(), Box<dyn Error>> {
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Mover digitando")?)?;
        session.execute(Command::ImportMesh {
            destination: None,
            name: "Suporte".into(),
            geometry: std::sync::Arc::new(nator_document::MeshAsset::new(
                nator_geometry::reference_bracket()?,
            )),
        })?;
        let mut editor = Editor::from_gpu(state, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        let instance = editor.current().session.document().instances()[0].id;
        settle(&mut editor, &context)?;
        editor.current_mut().selected = vec![Selection::Instance(instance)];
        editor.open_move();
        frame(&mut editor, &context, Vec::new())?;
        // Campo X da translação: o primeiro texto "0,00" do painel, de cima para baixo.
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280., 820.),
            )),
            time: Some(context.input(|i| i.time) + 1. / 60.),
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| editor.draw(ui));
        let mut fields = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.job.text == "0,00" => {
                    Some(t.visual_bounding_rect().center())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        fields.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        let field = *fields.first().ok_or("Campo X ausente")?;
        click(&mut editor, &context, field)?;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("12,5".into())],
        )?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        assert!(
            editor.assembly_panel.is_some(),
            "Enter no campo não deve fechar o painel"
        );
        settle(&mut editor, &context)?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        assert!(editor.assembly_panel.is_none(), "{}", editor.status);
        let moved = editor
            .current()
            .session
            .document()
            .instance(instance)
            .ok_or("Instância ausente")?
            .placement
            .translation_mm();
        assert!((moved[0] - 12.5).abs() < 1e-9, "{moved:?}");
        Ok(())
    }

    /// A cópia só existe na prévia: Cancelar e Esc devem restaurar a malha original
    /// sem qualquer novo gesto no canvas e sem alterar revisão, salvamento ou desfazer.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn canceling_a_copy_preview_restores_the_document_scene() -> Result<(), Box<dyn Error>> {
        let state = gpu_state()?;
        for escape in [false, true] {
            let context = egui::Context::default();
            let mut document = Document::new("Cancelar cópia")?;
            let (part, body) = document.add_mesh(
                None,
                "Suporte".into(),
                Arc::new(MeshAsset::new(reference_bracket()?)),
            )?;
            let mut editor =
                Editor::from_gpu(state.clone(), &context, document).map_err(|e| e.to_string())?;
            editor.recoveries.clear();
            editor.show_recovery = false;
            editor.current_mut().active_part = Some(part);
            editor
                .current_mut()
                .select(Some(Selection::Body { part, id: body }));
            settle(&mut editor, &context)?;
            assert!(editor.requested.is_none());
            let original = editor.scene.as_ref().ok_or("Cena inicial ausente")?.clone();
            let revision = editor.current().session.revision();
            let saved_state = editor.current().session.state();
            editor.open_move();
            editor.set_move_for_capture([100., 0., 0.], [0.; 3]);
            frame(&mut editor, &context, Vec::new())?;
            let copy = label_position(&mut editor, &context, text(T::CreateCopy))?;
            click(&mut editor, &context, copy)?;

            let deadline = Instant::now() + Duration::from_secs(10);
            while editor.preview.shown == 0 || !editor.preview_idle() {
                if Instant::now() > deadline {
                    return Err(format!("Prévia da cópia não chegou: {}", editor.status).into());
                }
                std::thread::sleep(Duration::from_millis(5));
                frame(&mut editor, &context, Vec::new())?;
            }
            assert_eq!(
                editor
                    .scene
                    .as_ref()
                    .ok_or("Prévia ausente")?
                    .triangles()
                    .len(),
                2 * original.triangles().len()
            );
            if escape {
                frame(
                    &mut editor,
                    &context,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                )?;
            } else {
                let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
                click(&mut editor, &context, cancel)?;
            }
            assert!(editor.assembly_panel.is_none());
            assert_eq!(editor.preview.shown, 0);
            // Daqui em diante, só quadros sem eventos. A regressão do agendador em
            // live_preview também verifica que esse redesenho é solicitado pelo editor.
            settle(&mut editor, &context)?;
            let restored = editor.scene.as_ref().ok_or("Cena não restaurada")?;
            assert_eq!(restored.triangles().len(), original.triangles().len());
            assert_eq!(restored.bounds().min, original.bounds().min);
            assert_eq!(restored.bounds().max, original.bounds().max);
            assert_eq!(editor.rendered, Some(editor.scene_key()));
            assert!(editor.requested.is_none());
            assert_eq!(editor.current().session.revision(), revision);
            assert_eq!(editor.current().session.state(), saved_state);
            assert!(!editor.current().session.is_dirty());
            assert_eq!(
                editor
                    .current()
                    .session
                    .document()
                    .part(part)
                    .ok_or("Peça ausente")?
                    .bodies
                    .len(),
                1
            );
            assert!(!editor.current_mut().session.undo());
        }
        Ok(())
    }

    /// As duas entradas da aba Peça produzem intenções diferentes sem exigir que
    /// o usuário alterne uma opção escondida no painel. Ambas continuam reversíveis.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn part_ribbon_move_and_copy_preserve_their_distinct_intents() -> Result<(), Box<dyn Error>> {
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut document = Document::new("Ferramentas da peça")?;
        let (part, original) = document.add_mesh(
            None,
            "Suporte".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let (other_part, _) = document.add_mesh(
            None,
            "Outra peça".into(),
            Arc::new(MeshAsset::new(reference_bracket()?)),
        )?;
        let other_instance = document
            .instances()
            .iter()
            .find(|i| i.part == other_part)
            .ok_or("Instância externa ausente")?
            .id;
        document.instance_mut(other_instance)?.placement = Placement::new([300., 0., 0.], [0.; 3])?;
        let mut editor = Editor::from_gpu(state, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        editor
            .current_mut()
            .select(Some(Selection::Body { part, id: original }));
        editor.shell.tab = shell::RibbonTab::Model;
        settle(&mut editor, &context)?;
        assert_eq!(text(T::ModelTab), "Peça");

        let copy = label_position(&mut editor, &context, text(T::CopyMenu))?;
        click(&mut editor, &context, copy)?;
        assert!(editor.assembly_panel.is_some());
        editor.set_move_for_capture([100., 0., 0.], [0.; 3]);
        let enter = || egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut editor, &context, vec![enter()])?;
        settle(&mut editor, &context)?;
        let piece = editor
            .current()
            .session
            .document()
            .part(part)
            .ok_or("Peça ausente")?;
        assert_eq!(piece.bodies.len(), 2);
        let copy = piece
            .bodies
            .iter()
            .find(|b| b.id != original)
            .ok_or("Cópia ausente")?
            .id;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(original)
                .ok_or("Original ausente")?
                .placement
                .translation_mm(),
            [0.; 3]
        );
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(copy)
                .ok_or("Cópia ausente")?
                .placement
                .translation_mm(),
            [100., 0., 0.]
        );

        editor
            .current_mut()
            .select(Some(Selection::Body { part, id: copy }));
        settle(&mut editor, &context)?;
        let move_button = label_position(&mut editor, &context, text(T::MoveMenu))?;
        click(&mut editor, &context, move_button)?;
        let free = label_position(&mut editor, &context, text(T::MoveFree))?;
        click(&mut editor, &context, free)?;
        editor.set_move_for_capture([50., 0., 0.], [0.; 3]);
        frame(&mut editor, &context, vec![enter()])?;
        settle(&mut editor, &context)?;
        let piece = editor
            .current()
            .session
            .document()
            .part(part)
            .ok_or("Peça ausente")?;
        assert_eq!(piece.bodies.len(), 2);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(original)
                .ok_or("Original ausente")?
                .placement
                .translation_mm(),
            [0.; 3]
        );
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(copy)
                .ok_or("Cópia ausente")?
                .placement
                .translation_mm(),
            [150., 0., 0.]
        );
        assert!(editor.current_mut().session.undo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(copy)
                .ok_or("Cópia ausente")?
                .placement
                .translation_mm(),
            [100., 0., 0.]
        );
        assert!(editor.current_mut().session.undo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .part(part)
                .ok_or("Peça ausente")?
                .bodies
                .len(),
            1
        );
        editor.current_mut().changed();
        editor
            .current_mut()
            .select(Some(Selection::Body { part, id: original }));
        editor.invalidate_scene();
        settle(&mut editor, &context)?;
        // Cada subopção usa geometria da malha real e altera somente a posição do
        // corpo. Desfazer entre os casos mantém a mesma origem de referência.
        for (label, mode) in [
            (T::MoveFaceToFace, navigation::PickMode::Face),
            (T::MoveEdgeToEdge, navigation::PickMode::Edge),
            (T::MovePointToPoint, navigation::PickMode::Vertex),
        ] {
            let move_button = label_position(&mut editor, &context, text(T::MoveMenu))?;
            click(&mut editor, &context, move_button)?;
            let option = label_position(&mut editor, &context, text(label))?;
            click(&mut editor, &context, option)?;
            assert_eq!(editor.pick_mode, mode);
            let mut from = editor
                .current()
                .session
                .document()
                .pick(nator_geometry::query::Ray {
                    origin: [10., 10., 100.],
                    direction: [0., 0., -1.],
                })?
                .ok_or("Corpo de origem não atingido")?;
            navigation::snap_pick(&mut from, mode);
            let target = std::array::from_fn(|i| from.point[i] + [5., 6., 7.][i]);
            editor.set_align_for_capture(from.clone(), target);
            frame(&mut editor, &context, vec![enter()])?;
            settle(&mut editor, &context)?;
            let document = editor.current().session.document();
            let body = document.body(original).ok_or("Corpo alinhado ausente")?;
            assert_eq!(document.part(part).ok_or("Peça ausente")?.bodies.len(), 1);
            assert!(
                nator_geometry::norm(nator_geometry::sub(
                    body.placement.apply(from.point),
                    target
                )) < 1e-8
            );
            assert_eq!(
                document
                    .instance(from.instance)
                    .ok_or("Instância ausente")?
                    .placement
                    .translation_mm(),
                [0.; 3]
            );
            assert!(editor.current_mut().session.undo());
            editor.current_mut().changed();
            editor.invalidate_scene();
            settle(&mut editor, &context)?;
        }
        // Uma origem pertencente a outra peça nunca pode transformar esse componente
        // durante a edição da peça ativa, mesmo que chegue ao painel por uma seleção.
        let move_button = label_position(&mut editor, &context, text(T::MoveMenu))?;
        click(&mut editor, &context, move_button)?;
        let option = label_position(&mut editor, &context, text(T::MovePointToPoint))?;
        click(&mut editor, &context, option)?;
        let foreign = editor
            .current()
            .session
            .document()
            .pick(nator_geometry::query::Ray {
                origin: [310., 10., 100.],
                direction: [0., 0., -1.],
            })?
            .ok_or("Corpo externo não atingido")?;
        assert_eq!(foreign.part, other_part);
        let revision = editor.current().session.revision();
        editor.set_align_for_capture(foreign, [0.; 3]);
        frame(&mut editor, &context, vec![enter()])?;
        assert!(editor.assembly_panel.is_some());
        assert_eq!(editor.current().session.revision(), revision);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .instance(other_instance)
                .ok_or("Instância externa ausente")?
                .placement
                .translation_mm(),
            [300., 0., 0.]
        );
        Ok(())
    }

    /// Exercita os comandos promovidos da nova faixa, incluindo expressão, confirmação,
    /// cancelamento e desfazer. O rascunho continua no mesmo viewport durante todo o fluxo.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_ribbon_dimensions_offset_and_cancel_are_transactional() -> Result<(), Box<dyn Error>>
    {
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Faixa do rascunho", Plane::xy());
        let line = sketch.add_line([0., 0.], [20., 0.], false)?;
        sketch.add_line([0., -10.], [0., 10.], true)?;
        sketch.add_circle([40., 0.], 5., true)?;
        let mut document = Document::new("Rascunho pela faixa")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor = Editor::from_gpu(state, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let enter = || key(egui::Key::Enter, egui::Modifiers::NONE);
        let point = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [10., 0., 0.],
        )
        .ok_or("Linha sem projeção")?;
        click(&mut editor, &context, point)?;
        // Cota: clicar na linha já abre a caixa com a medida selecionada; digitar a
        // substitui e um Enter confirma em uma ação.
        let dimension = label_position(&mut editor, &context, text(T::SketchDimension))?;
        click(&mut editor, &context, dimension)?;
        click(&mut editor, &context, point)?;
        frame(&mut editor, &context, Vec::new())?;
        label_position(&mut editor, &context, "20 mm")?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("32 mm".into())],
        )?;
        frame(&mut editor, &context, vec![enter()])?;
        settle(&mut editor, &context)?;
        let record = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?;
        let (a, b) = record.sketch.line(line)?;
        assert!((nator_sketch::distance(a, b) - 32.).abs() < 1e-6);
        assert_eq!(record.sketch.dimension_expressions.len(), 1);
        frame(
            &mut editor,
            &context,
            vec![key(
                egui::Key::Z,
                egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            )],
        )?;
        settle(&mut editor, &context)?;
        let record = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?;
        let (a, b) = record.sketch.line(line)?;
        assert!((nator_sketch::distance(a, b) - 20.).abs() < 1e-6);
        assert!(record.sketch.constraints.is_empty());

        // Deslocar cria uma entidade; o original conserva sua geometria.
        let offset = label_position(&mut editor, &context, text(T::OffsetField))?;
        click(&mut editor, &context, offset)?;
        let field = label_position(&mut editor, &context, "20 mm")?;
        click(&mut editor, &context, field)?;
        frame(
            &mut editor,
            &context,
            vec![
                key(
                    egui::Key::A,
                    egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                ),
                egui::Event::Text("5 mm".into()),
            ],
        )?;
        frame(&mut editor, &context, vec![enter()])?;
        frame(&mut editor, &context, Vec::new())?;
        frame(&mut editor, &context, vec![enter()])?;
        settle(&mut editor, &context)?;
        let record = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?;
        assert_eq!(record.sketch.entities.len(), 4);
        assert_eq!(record.sketch.line(line)?, ([0., 0.], [20., 0.]));
        let last = record
            .sketch
            .entities
            .last()
            .ok_or("Deslocamento ausente")?
            .id;
        assert_eq!(record.sketch.line(last)?, ([0., 5.], [20., 5.]));
        frame(
            &mut editor,
            &context,
            vec![key(
                egui::Key::Z,
                egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            )],
        )?;
        settle(&mut editor, &context)?;

        assert!(label_position(&mut editor, &context, text(T::SketchCircularShort)).is_err());
        assert!(label_position(&mut editor, &context, text(T::SketchRectangularShort)).is_err());
        let mirror = label_position(&mut editor, &context, text(T::MirrorShort))?;
        click(&mut editor, &context, mirror)?;
        let center = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [45., 0., 0.],
        )
        .ok_or("Círculo sem projeção")?;
        click(&mut editor, &context, center)?;
        let revision = editor.current().session.revision();
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
        )?;
        assert_eq!(editor.current().session.revision(), revision);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            3
        );

        // Sair participa da mesma faixa rolável dos demais grupos. Em uma
        // janela compacta, rolar até o fim mantém as duas opções acessíveis.
        let mut exits = Vec::new();
        for index in 0..4 {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900., 600.),
                    )),
                    events: if index == 1 {
                        vec![
                            egui::Event::PointerMoved(egui::pos2(450., 140.)),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Point,
                                delta: egui::vec2(-2000., 0.),
                                phase: egui::TouchPhase::Move,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    } else {
                        Vec::new()
                    },
                    time: Some(context.input(|input| input.time) + 0.1),
                    ..Default::default()
                },
                |ui| editor.draw(ui),
            );
            exits = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(t)
                        if [text(T::FinishSketchShort), text(T::Cancel)]
                            .contains(&t.galley.job.text.as_str()) =>
                    {
                        let rect = t.visual_bounding_rect();
                        (rect.top() < 198. && clipped.clip_rect.contains_rect(rect))
                            .then_some(t.galley.job.text.clone())
                    }
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
        }
        assert!(
            exits
                .iter()
                .any(|label| label == text(T::FinishSketchShort)),
            "{exits:?}"
        );
        assert!(
            exits.iter().any(|label| label == text(T::Cancel)),
            "{exits:?}"
        );
        Ok(())
    }

    /// Concluir aceita o estado publicado; Cancelar restaura somente o rascunho
    /// e também funciona quando uma geometria inválida está pendente na interface.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_exit_group_confirms_or_restores_edits_without_losing_other_changes()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Saída", Plane::xy());
        sketch.add_line([0., 0.], [20., 0.], false)?;
        let mut document = Document::new("Saída do rascunho")?;
        let (part, id) = document.add_sketch(None, sketch.clone())?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, document)
            .map_err(|error| error.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.shell.tab = shell::RibbonTab::Model;
        editor.begin_sketch(part, id);
        settle(&mut editor, &context)?;
        let initial = serde_json::to_value(&sketch)?;
        sketch.add_line([20., 0.], [20., 10.], false)?;
        editor.execute(Command::UpdateSketch {
            id,
            sketch: sketch.clone(),
        });
        editor.execute(Command::RenamePart {
            id: part,
            name: "Nome preservado".into(),
        });
        settle(&mut editor, &context)?;
        // Um rascunho pendente bloqueia Concluir, mas não pode bloquear Cancelar.
        editor
            .current_mut()
            .sketch_edit
            .as_mut()
            .ok_or("Editor ausente")?
            .pending = Some(sketch.clone());
        let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
        click(&mut editor, &context, cancel)?;
        settle(&mut editor, &context)?;
        assert!(editor.current().sketch_edit.is_none());
        assert_eq!(
            serde_json::to_value(
                &editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
            )?,
            initial
        );
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .part(part)
                .ok_or("Peça ausente")?
                .name,
            "Nome preservado"
        );
        frame(&mut editor, &context, Vec::new())?;
        assert_eq!(editor.shell.tab, shell::RibbonTab::Model);
        assert!(editor.current_mut().session.undo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            2
        );
        assert!(editor.current_mut().session.redo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            1
        );

        editor.begin_sketch(part, id);
        settle(&mut editor, &context)?;
        let mut accepted = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .clone();
        accepted.add_circle([30., 10.], 5., false)?;
        editor.execute(Command::UpdateSketch {
            id,
            sketch: accepted,
        });
        settle(&mut editor, &context)?;
        let finish = label_position(&mut editor, &context, text(T::FinishSketchShort))?;
        click(&mut editor, &context, finish)?;
        settle(&mut editor, &context)?;
        assert!(editor.current().sketch_edit.is_none());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            2
        );

        // Cancelar sem alterações, inclusive em um rascunho de referência, não
        // cria uma entrada artificial no histórico nem remove o rascunho existente.
        let reference = editor
            .current_mut()
            .session
            .document()
            .reference_sketches()
            .first()
            .map(|record| record.id);
        assert!(reference.is_none());
        editor.execute(Command::AddReferenceSketch {
            sketch: Sketch::new("Referência", Plane::xy()),
        });
        settle(&mut editor, &context)?;
        let reference = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Referência não aberta")?
            .id;
        let revision = editor.current().session.revision();
        editor.cancel_sketch(&context);
        assert!(editor.current().sketch_edit.is_none());
        assert_eq!(editor.current().session.revision(), revision);
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(reference)
                .is_some()
        );
        Ok(())
    }

    /// Apoios vêm dos sólidos fora do plano editado: capturar os mesmos pontos do
    /// próprio rascunho não exercitaria a projeção que faltava na interface.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_captures_projected_model_midpoints_and_polygon_centers() -> Result<(), Box<dyn Error>>
    {
        let mut document = Document::new("Apoios projetados")?;
        let mut import = |mut solid: nator_kernel::Solid| -> Result<BodyId, Box<dyn Error>> {
            let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
            let (_, body) = document.add_mesh(
                None,
                "Fonte".into(),
                Arc::new(MeshAsset::with_faces(mesh, faces)?),
            )?;
            document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
            document.body_mut(body)?.face_names = Arc::new(nator_kernel::indexed_face_names(
                &nator_document::features::body_owner(body),
                solid.face_count()?,
            ));
            Ok(body)
        };
        let rectangle = import(nator_kernel::Solid::cuboid(
            [-30., -5., 0.],
            [20., 10., 4.],
        )?)?;
        let mut triangle = Sketch::new("Triângulo", Plane::xy());
        for (a, b) in [
            ([20., -5.], [38., -5.]),
            ([38., -5.], [20., 13.]),
            ([20., 13.], [20., -5.]),
        ] {
            triangle.add_line(a, b, false)?;
        }
        let region = nator_sketch::areas::regions(&triangle)?.remove(0);
        import(nator_kernel::Solid::extrude(
            &triangle, &region, 0., 6., "triangle",
        )?)?;
        let plane = Plane {
            origin: [0., 0., 20.],
            ..Plane::xy()
        };
        let mut sketch = Sketch::new("Projeção", plane);
        sketch.add_line([-100., -80.], [100., -80.], true)?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let context = egui::Context::default();
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let wait_projection = |editor: &mut Editor,
                               expected: [f64; 2]|
         -> Result<(), Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                frame(editor, &context, Vec::new())?;
                if editor
                    .current()
                    .sketch_edit
                    .as_ref()
                    .and_then(|w| w.projected_points())
                    .is_some_and(|points| {
                        points
                            .iter()
                            .any(|p| nator_sketch::distance(*p, expected) < 1e-8)
                    })
                {
                    break;
                }
                if Instant::now() > deadline {
                    return Err(format!("Projeção ausente {expected:?}: {}", editor.status).into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(())
        };
        let position = |editor: &Editor, point| -> Result<egui::Pos2, Box<dyn Error>> {
            sketch_scene::Frame {
                camera: editor.viewport.camera.clone(),
                bounds: editor.display_bounds(),
                rect: editor.canvas_rect,
                placement: Placement::default(),
                plane,
            }
            .screen(point)
            .ok_or_else(|| "Projeção sem posição de tela".into())
        };
        // Centro do retângulo, meio da aresta e baricentro da face triangular.
        for expected in [[-20., 0.], [-20., -5.], [26., 1.]] {
            wait_projection(&mut editor, expected)?;
            editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Editor ausente")?
                .select_tool(sketch_editor::Tool::Line);
            // A troca direta usada pelo teste deve produzir o mesmo quadro da
            // faixa: remover os campos da cadeia anterior antes do próximo clique.
            frame(&mut editor, &context, Vec::new())?;
            let first = position(&editor, expected)? + egui::vec2(2., 2.);
            click(&mut editor, &context, first)?;
            let last = position(&editor, [expected[0] + 8., expected[1] + 15.])?;
            click(&mut editor, &context, last)?;
            settle(&mut editor, &context)?;
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert!(
                sketch.entities.iter().filter(|e| !e.construction).any(|e| {
                    sketch
                        .line(e.id)
                        .is_ok_and(|(a, _)| nator_sketch::distance(a, expected) < 1e-8)
                }),
                "Ponto inicial não capturado: {expected:?}"
            );
            assert_eq!(sketch.plane.world(expected)[2], 20.);
        }
        assert!(editor.current_mut().session.undo());
        assert!(editor.current_mut().session.redo());
        editor.execute(Command::MoveBody {
            id: rectangle,
            placement: Placement::new([10., 3., 0.], [0.; 3])?,
        });
        settle(&mut editor, &context)?;
        wait_projection(&mut editor, [-10., 3.])?;
        let points = editor
            .current()
            .sketch_edit
            .as_ref()
            .and_then(|w| w.projected_points())
            .ok_or("Projeções ausentes")?;
        assert!(
            !points
                .iter()
                .any(|p| nator_sketch::distance(*p, [-20., 0.]) < 1e-8),
            "A projeção antiga ficou em cache"
        );
        assert!(!editor.status.has_pending());
        Ok(())
    }

    /// Fluxo real: referências virtuais criam círculos nos centros dos polígonos;
    /// o aparador remove apenas os trechos entre os encontros, com desfazer/refazer.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_trim_and_polygon_centers_follow_mouse_and_history() -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Aparador", Plane::xy());
        let line = sketch.add_line([-20., 0.], [20., 0.], false)?;
        let circle = sketch.add_circle([0., 0.], 10., false)?;
        sketch.add_rectangle([-50., -15.], [-30., 5.], false)?;
        for (a, b) in [
            ([-80., -15.], [-60., -15.]),
            ([-60., -15.], [-80., 15.]),
            ([-80., 15.], [-80., -15.]),
        ] {
            sketch.add_line(a, b, false)?;
        }
        sketch.add_line([-100., -80.], [100., -80.], true)?;
        let mut document = Document::new("Aparar por vértices")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            frame(&mut editor, &context, Vec::new())?;
            if editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .reference_points()
                .centers
                .len()
                == 2
            {
                break;
            }
            if Instant::now() > deadline {
                return Err("Referências não ficaram prontas".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let position = |editor: &Editor, p| -> Result<egui::Pos2, Box<dyn Error>> {
            sketch_scene::Frame {
                camera: editor.viewport.camera.clone(),
                bounds: editor.display_bounds(),
                rect: editor.canvas_rect,
                placement: Placement::default(),
                plane: Plane::xy(),
            }
            .screen(p)
            .ok_or_else(|| "Ponto sem projeção".into())
        };
        for center in [[-40., -5.], [-220. / 3., -5.]] {
            editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Editor ausente")?
                .select_tool(sketch_editor::Tool::Circle);
            let first = position(&editor, center)? + egui::vec2(2., 2.);
            click(&mut editor, &context, first)?;
            let last = position(&editor, [center[0] + 3., center[1]])?;
            click(&mut editor, &context, last)?;
            settle(&mut editor, &context)?;
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert!(
                sketch
                    .entities
                    .iter()
                    .any(|e| sketch
                        .circle(e.id)
                        .is_ok_and(|(p, r)| nator_sketch::distance(p, center) < 1e-8
                            && (r - 3.).abs() < 1e-3)),
                "Centro {center:?}; círculos {:?}; estado {}; cursores {first:?} {last:?}",
                sketch
                    .entities
                    .iter()
                    .filter_map(|e| sketch.circle(e.id).ok())
                    .collect::<Vec<_>>(),
                editor.status
            );
        }
        editor
            .current_mut()
            .sketch_edit
            .as_mut()
            .ok_or("Editor ausente")?
            .select_tool(sketch_editor::Tool::Trim);
        let revision = editor.current().session.revision();
        let at = position(&editor, [0., 0.])?;
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(at)])?;
        assert_eq!(editor.current().session.revision(), revision);
        click(&mut editor, &context, at)?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert!(sketch.entity(line).is_err());
        assert_eq!(sketch.circle(circle)?, ([0., 0.], 10.));
        let pieces = sketch
            .entities
            .iter()
            .filter_map(|e| sketch.line(e.id).ok())
            .filter(|(a, b)| a[1].abs() < 1e-8 && b[1].abs() < 1e-8)
            .collect::<Vec<_>>();
        assert_eq!(pieces.len(), 2);
        assert!(
            pieces
                .iter()
                .all(|(a, b)| (nator_sketch::distance(*a, *b) - 10.).abs() < 1e-8)
        );
        assert!(editor.current_mut().session.undo());
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entity(line)
                .is_ok()
        );
        assert!(editor.current_mut().session.redo());
        editor.current_mut().changed();
        settle(&mut editor, &context)?;
        let at = position(&editor, [0., 10.])?;
        click(&mut editor, &context, at)?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert!(sketch.entity(circle).is_err());
        let arc = sketch
            .entities
            .iter()
            .find(|e| matches!(e.curve, nator_sketch::Curve::Arc { .. }))
            .ok_or("Arco restante ausente")?;
        assert!(
            nator_sketch::profiles::segment(sketch, arc)?
                .sample(64)
                .iter()
                .all(|p| p[1] <= 1e-8)
        );
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("aparado.3dnator");
        nator_project_io::write_project(&path, editor.current().session.document())?;
        let restored = nator_project_io::read_project(&path)?;
        assert_eq!(
            restored
                .sketch(id)
                .ok_or("Rascunho perdido")?
                .sketch
                .entities
                .len(),
            sketch.entities.len()
        );
        assert!(!editor.status.has_pending());
        Ok(())
    }

    /// Digitação sem clicar nos campos, alternância por Tab, Enter único e cancelamento.
    /// Verifica o resultado publicado e as cotas, em vez de apenas inspecionar o formulário.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn line_types_relative_angle_and_floating_fields_follow_the_canvas()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, LineType, lines};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Linhas", Plane::xy());
        let reference = sketch.add_line([-40., -20.], [-10., 10.], false)?;
        let target = sketch.add_line([20., -30.], [60., -30.], false)?;
        sketch.add_point([-60., -60.])?;
        sketch.add_point([80., 60.])?;
        let mut document = Document::new("Desenho de linhas")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        frame(&mut editor, &context, Vec::new())?;
        settle(&mut editor, &context)?;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let position = |editor: &Editor, p: [f64; 2]| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                [p[0], p[1], 0.],
            )
            .ok_or("Ponto fora do canvas")
        };
        let tool = label_position(&mut editor, &context, text(T::Line))?;
        click(&mut editor, &context, tool)?;
        let start = position(&editor, [-25., -5.])?;
        click(&mut editor, &context, start)?;
        let mouse = position(&editor, [-50., 20.])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(mouse)],
        )?;
        frame(&mut editor, &context, Vec::new())?;
        let length =
            egui::AreaState::load(&context, egui::Id::new(("sketch_dynamic_input", 0_usize)))
                .ok_or("Campo de comprimento ausente")?
                .rect();
        let angle =
            egui::AreaState::load(&context, egui::Id::new(("sketch_dynamic_input", 1_usize)))
                .ok_or("Campo de ângulo ausente")?
                .rect();
        assert!(
            length.min.distance(mouse + egui::vec2(24., 16.)) < 3.,
            "{length:?}; {mouse:?}"
        );
        assert!(angle.center().distance(start) < 160.);
        assert!(
            !length.intersects(angle),
            "comprimento: {length:?}; ângulo: {angle:?}"
        );
        // O ícone da linha acompanha o ponteiro sem ocupar sua ponta de captura.
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        let badge = mouse + egui::vec2(16., -28.) + egui::vec2(4., 19.) * (22. / 24.);
        assert!(
            output.shapes.iter().any(
                |s| matches!(&s.shape,egui::Shape::Circle(c) if c.center.distance(badge) < 0.5)
            )
        );
        output.textures_delta.clear();
        frame(&mut editor, &context, vec![egui::Event::Text("12".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Tab)])?;
        frame(&mut editor, &context, vec![egui::Event::Text("90".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let line = sketch.entities.last().ok_or("Linha não criada")?.id;
        let (a, b) = sketch.line(line)?;
        assert!((nator_sketch::distance(a, b) - 12.).abs() < 1e-6);
        assert!(
            (lines::relative_angle(
                lines::angle(sketch, line)?,
                lines::angle(sketch, reference)?
            ) - std::f64::consts::FRAC_PI_2)
                .abs()
                < 1e-6
        );
        assert!(sketch.constraints.iter().any(|c| matches!(c.kind,ConstraintKind::LineAngle {line:l, reference:r,..} if l==line && r==reference)));
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        assert!(editor.selection_active());
        let tool = label_position(&mut editor, &context, text(T::Line))?;
        click(&mut editor, &context, tool)?;
        // Aproximar a outra linha termina exatamente a 90°, com incidência
        // sobre o alvo e uma restrição perpendicular publicada no histórico.
        let start = position(&editor, [35., -5.])?;
        let end = position(&editor, [35.1, -29.9])?;
        click(&mut editor, &context, start)?;
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
        click(&mut editor, &context, end)?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let perpendicular = sketch.entities.last().ok_or("Perpendicular ausente")?.id;
        let (a, b) = sketch.line(perpendicular)?;
        assert!((a[0] - b[0]).abs() < 1e-6 && (b[1] + 30.).abs() < 1e-6);
        assert!(sketch.constraints.iter().any(|c| matches!(c.kind,ConstraintKind::Perpendicular {a,b} if (a==target && b==perpendicular) || (b==target && a==perpendicular))));
        for (kind, label, a, b) in [
            (
                LineType::Construction,
                T::Construction,
                [15., 35.],
                [45., 30.],
            ),
            (
                LineType::Infinite,
                T::SketchLineInfinite,
                [-50., 35.],
                [-25., 45.],
            ),
        ] {
            let choice = label_position(&mut editor, &context, text(label))?;
            click(&mut editor, &context, choice)?;
            assert_eq!(
                editor
                    .current()
                    .sketch_edit
                    .as_ref()
                    .ok_or("Editor ausente")?
                    .line_type,
                kind
            );
            let start = position(&editor, a)?;
            let end = position(&editor, b)?;
            click(&mut editor, &context, start)?;
            frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
            click(&mut editor, &context, end)?;
            settle(&mut editor, &context)?;
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert_eq!(
                sketch.entities.last().ok_or("Linha ausente")?.line_type(),
                kind,
                "{}",
                editor.status
            );
        }
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("linhas.3dnator");
        write_project(&path, editor.current().session.document())?;
        let reopened = read_project(&path)?;
        let sketch = &reopened.sketch(id).ok_or("Rascunho ausente")?.sketch;
        let infinite = sketch.entities.last().ok_or("Reta ausente")?.id;
        // Guias de alinhamento podem deslocar a origem escolhida. A referência
        // deve continuar capturável longe dos dois apoios realmente publicados.
        let (a, b) = sketch.line(infinite)?;
        let far = nator_sketch::add(a, nator_sketch::scale(nator_sketch::sub(b, a), 10.));
        assert_eq!(
            nator_sketch::tools::pick_entity(sketch, far, 0.01)?,
            Some(infinite)
        );
        assert!(editor.current_mut().session.undo());
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .is_some_and(|s| s.sketch.entity(infinite).is_err())
        );
        assert!(editor.current_mut().session.redo());
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .is_some_and(|s| s.sketch.entity(infinite).is_ok())
        );
        Ok(())
    }

    /// Exercita os cliques reais: seleção de modo, etapas adicionais, continuidade
    /// e Esc. As regressões sem GPU verificam recálculo, persistência e dimensões.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn drawing_modes_follow_clicks_and_continuous_chains() -> Result<(), Box<dyn Error>> {
        use sketch_editor::Tool;
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Modos de desenho", Plane::xy());
        sketch.add_point([-100., -100.])?;
        sketch.add_point([100., 100.])?;
        sketch.add_line([35., 35.], [65., 35.], false)?;
        sketch.add_line([50., 20.], [50., 50.], false)?;
        let mut document = Document::new("Gestos de desenho")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let select = |editor: &mut Editor, tool| -> Result<(), Box<dyn Error>> {
            editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Editor ausente")?
                .select_tool(tool);
            frame(editor, &context, Vec::new())?;
            Ok(())
        };
        let choose = |editor: &mut Editor, label| -> Result<(), Box<dyn Error>> {
            let position = label_position(editor, &context, text(label))?;
            click(editor, &context, position)
        };
        let point = |editor: &mut Editor, p: [f64; 2]| -> Result<(), Box<dyn Error>> {
            let position = sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                [p[0], p[1], 0.],
            )
            .ok_or("Ponto fora do canvas")?;
            click(editor, &context, position)?;
            settle(editor, &context)
        };
        let drawing = |editor: &Editor| {
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|work| work.is_drawing())
        };
        select(&mut editor, Tool::Line)?;
        point(&mut editor, [-70., 0.])?;
        for p in [[-50., 0.], [-50., 20.]] {
            point(&mut editor, p)?;
            assert!(drawing(&editor));
        }
        point(&mut editor, [-70., 0.])?;
        assert!(!drawing(&editor));
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            5
        );
        choose(&mut editor, T::Symmetric)?;
        point(&mut editor, [-70., -30.])?;
        point(&mut editor, [-60., -20.])?;
        assert!(drawing(&editor));
        let revision = editor.current().session.revision();
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert!(!drawing(&editor));
        assert_eq!(editor.current().session.revision(), revision);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            7
        );

        select(&mut editor, Tool::Rectangle)?;
        choose(&mut editor, T::SketchRectangleCenter)?;
        point(&mut editor, [-25., -40.])?;
        point(&mut editor, [-15., -35.])?;
        assert!(!drawing(&editor));
        let rectangle = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let corners = rectangle.entities[7..11]
            .iter()
            .map(|entity| rectangle.line(entity.id).map(|line| line.0))
            .collect::<nator_sketch::Result<Vec<_>>>()?;
        // O clique passa por coordenadas f32 da tela. A tolerância de 0,00002 mm
        // cobre esse arredondamento ao mudar a altura do canvas, sem folga geométrica.
        assert!(
            nator_sketch::distance(
                nator_sketch::scale(nator_sketch::add(corners[0], corners[2]), 0.5),
                [-25., -40.]
            ) < 2e-5,
            "Cantos obtidos: {corners:?}"
        );
        choose(&mut editor, T::SketchRectangleInclined)?;
        point(&mut editor, [-70., 55.])?;
        point(&mut editor, [-50., 65.])?;
        assert!(drawing(&editor));
        point(&mut editor, [-55., 75.])?;
        assert!(!drawing(&editor));
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            15
        );

        select(&mut editor, Tool::Circle)?;
        assert!(label_position(&mut editor, &context, text(T::SketchLineInfinite)).is_err());
        choose(&mut editor, T::SketchCircleDiameter)?;
        point(&mut editor, [0., -40.])?;
        point(&mut editor, [20., -40.])?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let (center, radius) =
            sketch.circle(sketch.entities.last().ok_or("Círculo ausente")?.id)?;
        assert!(nator_sketch::distance(center, [10., -40.]) < 1e-5);
        assert!((radius - 10.).abs() < 1e-5);
        choose(&mut editor, T::SketchCircleTangent)?;
        point(&mut editor, [40., 35.])?;
        point(&mut editor, [50., 25.])?;
        assert!(drawing(&editor));
        point(&mut editor, [55., 40.])?;
        assert!(!drawing(&editor));
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let (center, radius) =
            sketch.circle(sketch.entities.last().ok_or("Círculo ausente")?.id)?;
        assert!((center[0] - 50. - radius).abs() < 1e-5);
        assert!((center[1] - 35. - radius).abs() < 1e-5);
        assert_eq!(
            sketch
                .constraints
                .iter()
                .filter(|constraint| matches!(
                    constraint.kind,
                    nator_sketch::ConstraintKind::Tangent { .. }
                ))
                .count(),
            2
        );
        // Preset digitado antes da origem e edição do ângulo no painel durante a
        // prévia devem confirmar a mesma cota por Enter, sem roubar o foco.
        select(&mut editor, Tool::Line)?;
        let field = |index: usize| -> Result<egui::Pos2, Box<dyn Error>> {
            context
                .read_response(egui::Id::new(("sketch_measurement_field", index)))
                .map(|response| response.rect.center())
                .ok_or_else(|| "Campo de medida ausente".into())
        };
        let length_field = field(0)?;
        click(&mut editor, &context, length_field)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("12 mm".into())],
        )?;
        point(&mut editor, [-75., -75.])?;
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields()[0],
            ("12 mm".into(), true)
        );
        let mouse = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [-55., -65., 0.],
        )
        .ok_or("Ponto sem projeção")?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(mouse)],
        )?;
        let angle_field = field(1)?;
        click(&mut editor, &context, angle_field)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("30 deg".into())],
        )?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let line = sketch.entities[sketch.entities.len() - 2].id;
        let (a, b) = sketch.line(line)?;
        assert!(
            (nator_sketch::distance(a, b) - 12.).abs() < 1e-6,
            "{a:?}; {b:?}; {}",
            editor.status
        );
        assert!((nator_sketch::lines::angle(sketch, line)?.to_degrees() - 30.).abs() < 1e-6);

        // A área pode se apoiar nas retas anteriores, sem voltar a um ponto da
        // cadeia recém-iniciada. A resposta do trabalhador também deve encerrá-la.
        select(&mut editor, Tool::Line)?;
        point(&mut editor, [-60., 0.])?;
        point(&mut editor, [-60., 10.])?;
        assert!(!drawing(&editor));

        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn drawing_dimensions_follow_typing_tab_enter_and_escape() -> Result<(), Box<dyn Error>> {
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Entrada durante o desenho", Plane::xy());
        sketch.add_line([-100., -100.], [100., 100.], true)?;
        let mut document = Document::new("Cotas dinâmicas")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor = Editor::from_gpu(state, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let start = egui::pos2(
            editor.canvas_rect.left() + 140.,
            editor.canvas_rect.top() + 120.,
        );
        let end = start + egui::vec2(100., -70.);
        let select = |editor: &mut Editor, tool| -> Result<(), Box<dyn Error>> {
            editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Editor ausente")?
                .select_tool(tool);
            Ok(())
        };
        select(&mut editor, sketch_editor::Tool::Line)?;
        click(&mut editor, &context, start)?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.is_drawing())
        );
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
        let before = editor.current().session.revision();
        frame(&mut editor, &context, vec![egui::Event::Text("40".into())])?;
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields()[0],
            ("40".into(), true)
        );
        frame(&mut editor, &context, vec![key(egui::Key::Tab)])?;
        frame(&mut editor, &context, vec![egui::Event::Text("30".into())])?;
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields()[1],
            ("30".into(), true)
        );
        // Shift+Tab volta ao comprimento; limpar libera a medida para seguir o mouse.
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
        )?;
        frame(&mut editor, &context, vec![key(egui::Key::Backspace)])?;
        assert!(
            !editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields()[0]
                .1,
            "{:?}; foco: {:?}",
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields(),
            context.memory(|m| m.focused())
        );
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(end + egui::vec2(10., 0.))],
        )?;
        let live = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Editor ausente")?
            .drawing_fields()[0]
            .0
            .clone();
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(end + egui::vec2(60., 0.))],
        )?;
        assert_ne!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .drawing_fields()[0]
                .0,
            live
        );
        frame(&mut editor, &context, vec![egui::Event::Text("40".into())])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(end + egui::vec2(40., 50.))],
        )?;
        assert_eq!(editor.current().session.revision(), before);
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let s = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(s.entities.len(), 2, "{}", editor.status);
        let (a, b) = s.line(s.entities[1].id)?;
        assert!(
            (nator_sketch::distance(a, b) - 40.).abs() < 1e-6,
            "{a:?} {b:?}"
        );
        assert!(((b[1] - a[1]).atan2(b[0] - a[0]).to_degrees() - 30.).abs() < 1e-6);
        assert_eq!(s.dimension_expressions.len(), 2);

        select(&mut editor, sketch_editor::Tool::Circle)?;
        click(&mut editor, &context, start + egui::vec2(0., 140.))?;
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("12,5".into())],
        )?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let s = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(s.entities.len(), 3, "{}", editor.status);
        assert!((s.circle(s.entities[2].id)?.1 - 12.5).abs() < 1e-6);

        select(&mut editor, sketch_editor::Tool::Rectangle)?;
        click(&mut editor, &context, start + egui::vec2(320., 70.))?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(start + egui::vec2(240., 140.))],
        )?;
        frame(&mut editor, &context, vec![egui::Event::Text("30".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Tab)])?;
        frame(&mut editor, &context, vec![egui::Event::Text("20".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let s = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(s.entities.len(), 7, "{}", editor.status);
        let (a, b) = s.line(s.entities[3].id)?;
        assert!((b[0] - a[0] + 30.).abs() < 1e-6);
        let (a, b) = s.line(s.entities[4].id)?;
        assert!((b[1] - a[1] + 20.).abs() < 1e-6);

        select(&mut editor, sketch_editor::Tool::Arc)?;
        click(&mut editor, &context, start + egui::vec2(280., 220.))?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(start + egui::vec2(350., 220.))],
        )?;
        frame(&mut editor, &context, vec![egui::Event::Text("10".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        frame(&mut editor, &context, Vec::new())?;
        frame(&mut editor, &context, vec![egui::Event::Text("90".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let s = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(s.entities.len(), 8, "{}", editor.status);
        let arc = s.entities[7].id;
        assert!((s.radial(arc)?.1 - 10.).abs() < 1e-6);
        let points = nator_sketch::profiles::segment(s, s.entity(arc)?)?.sample(4);
        assert_eq!(points.len(), 3);

        // Valor inválido não publica geometria nem encerra a etapa; Esc descarta a prévia.
        select(&mut editor, sketch_editor::Tool::Circle)?;
        click(&mut editor, &context, start)?;
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
        let before = editor.current().session.revision();
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("-5".into()), key(egui::Key::Enter)],
        )?;
        assert_eq!(editor.current().session.revision(), before);
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.is_drawing())
        );
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        assert!(
            !editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.is_drawing())
        );
        assert_eq!(editor.current().session.revision(), before);
        for (modifiers, expected) in [
            (egui::Modifiers::CTRL, 7),
            (egui::Modifiers::CTRL | egui::Modifiers::SHIFT, 8),
        ] {
            frame(
                &mut editor,
                &context,
                vec![egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            )?;
            settle(&mut editor, &context)?;
            assert_eq!(
                editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
                    .entities
                    .len(),
                expected
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn extrusion_mouse_faces_and_automatic_modes_keep_preview_transactional()
    -> Result<(), Box<dyn Error>> {
        let state = gpu_state()?;
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Extrusão pelo mouse")?)?;
        let mut base_profile = Sketch::new("Placa", Plane::xy());
        base_profile.add_rectangle([0., 0.], [40., 30.], false)?;
        let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
            destination: None,
            sketch: base_profile,
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        let Effect::Imported { body: base, .. } = session.execute(Command::AddFeature {
            part,
            name: "Placa".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "10 mm".into(),
                combine: None,
                until: None,
            },
        })?
        else {
            return Err("Placa ausente".into());
        };
        let mut tower = Sketch::new("Limite", Plane::xy());
        tower.add_rectangle([70., 0.], [80., 10.], false)?;
        let Effect::SketchCreated { sketch, .. } = session.execute(Command::AddSketch {
            destination: Some(part),
            sketch: tower,
        })?
        else {
            return Err("Rascunho ausente".into());
        };
        let Effect::Imported { body: limit, .. } = session.execute(Command::AddFeature {
            part,
            name: "Limite".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "20 mm".into(),
                combine: None,
                until: None,
            },
        })?
        else {
            return Err("Limite ausente".into());
        };
        let mut free = Sketch::new("Livre", Plane::xy());
        free.add_circle([45., 15.], 2., false)?;
        session.execute(Command::AddSketch {
            destination: Some(part),
            sketch: free,
        })?;
        let mut editor = Editor::from_gpu(state, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        editor.viewport.camera.span = 2.5;
        settle(&mut editor, &context)?;
        let project = |editor: &Editor, point| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or("Ponto sem projeção")
        };
        let initial_revision = editor.current().session.revision();
        editor.open_feature_tool(feature_editor::Tool::Extrude);
        frame(&mut editor, &context, Vec::new())?;
        let point = project(&editor, [45., 15., 0.])?;
        click(&mut editor, &context, point)?;
        editor
            .feature_edit
            .as_mut()
            .ok_or("Painel ausente")?
            .set_size_for_capture("10 mm");
        wait_feature_preview(&mut editor, &context)?;
        assert_eq!(
            editor
                .feature_edit
                .as_ref()
                .and_then(|d| d.extrusion_state())
                .ok_or("Guia ausente")?
                .1,
            0
        );
        let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
        click(&mut editor, &context, cancel)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), initial_revision);

        // Uma face do corpo pode iniciar o perfil; não há criação auxiliar de rascunho.
        editor.open_feature_tool(feature_editor::Tool::Extrude);
        frame(&mut editor, &context, Vec::new())?;
        let point = project(&editor, [5., 5., 10.])?;
        click(&mut editor, &context, point)?;
        settle(&mut editor, &context)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .is_none()
        {
            if Instant::now() > deadline {
                return Err("Guia de comprimento zero ausente".into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let (guide, mode) = editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .ok_or("Guia de face ausente")?;
        assert_eq!(mode, 0);
        assert_eq!(guide.to, 0.);
        assert_eq!(editor.current().session.revision(), initial_revision);
        let origin = project(&editor, guide.origin)?;
        let endpoint = project(
            &editor,
            std::array::from_fn(|i| guide.origin[i] + guide.normal[i] * guide.to),
        )?;
        let unit = project(
            &editor,
            std::array::from_fn(|i| guide.origin[i] + guide.normal[i]),
        )? - origin;
        let endpoint = endpoint + unit.normalized() * 32.;
        let target = endpoint - unit * 3.;
        for (position, pressed) in [
            (endpoint, Some(true)),
            (target, None),
            (target, Some(false)),
        ] {
            let mut events = vec![egui::Event::PointerMoved(position)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            frame(&mut editor, &context, events)?;
        }
        wait_feature_preview(&mut editor, &context)?;
        let (guide, mode) = editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .ok_or("Guia ausente")?;
        assert_eq!(mode, 2);
        assert!((guide.to + 3.).abs() < 1e-4, "{}", guide.to);
        assert_eq!(editor.current().session.revision(), initial_revision);
        // O resultado restante e a malha do volume removido recompõem o volume original.
        assert!(
            (editor
                .scene
                .as_ref()
                .ok_or("Prévia ausente")?
                .signed_volume()
                .abs()
                - 14000.)
                .abs()
                < 1e-3
        );

        // O clique na torre troca o limite automaticamente, sem ativar um campo manual.
        let point = project(&editor, [75., 5., 20.])?;
        click(&mut editor, &context, point)?;
        settle(&mut editor, &context)?;
        wait_feature_preview(&mut editor, &context)?;
        let (guide, mode) = editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .ok_or("Guia ausente")?;
        assert_eq!(mode, 1);
        assert!((guide.to - 10.).abs() < 1e-5);
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        assert!(editor.feature_edit.is_none(), "{}", editor.status);
        let piece = editor
            .current()
            .session
            .document()
            .part(part)
            .ok_or("Peça ausente")?;
        assert!(
            matches!(piece.features.last().map(|f|&f.operation),Some(Operation::ExtrudeFace {base:source,until:Some(until),combine:Some(combine),..}) if *source==base&&matches!(until,nator_document::features::ExtrusionLimit::Face(face) if face.body==limit)&&combine.mode==nator_document::features::BooleanMode::Join)
        );
        assert_eq!(piece.sketches.len(), 3);
        assert!(editor.current_mut().session.undo());
        assert!(editor.current_mut().session.redo());
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn offset_plane_follows_the_face_drag_and_typing_and_hides_after_a_sketch()
    -> Result<(), Box<dyn Error>> {
        use sketch_scene::PlanePurpose;
        let context = egui::Context::default();
        let mut profile = Sketch::new("Base", Plane::xy());
        profile.add_rectangle([-50., -50.], [50., 50.], false)?;
        // O nome do projeto aparece na aba: não pode coincidir com o botão procurado.
        let mut session = Session::new(Document::new("Planos de trabalho")?)?;
        let Effect::SketchCreated { part, sketch } = session.execute(Command::AddSketch {
            destination: None,
            sketch: profile,
        })?
        else {
            return Err("Rascunho base ausente".into());
        };
        session.execute(Command::AddFeature {
            part,
            name: "Placa".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "10 mm".into(),
                combine: None,
                until: None,
            },
        })?;
        let mut editor = Editor::from_gpu(gpu_state()?, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.shell.tab = shell::RibbonTab::Model;
        // A peça ativa no primeiro quadro deixa o nó dela aberto na árvore.
        editor.current_mut().active_part = Some(part);
        // Vista superior: a normal da face aponta para quem observa, e o arraste da alça
        // passa a ser vertical na escala do plano.
        editor.viewport.camera.set_direction([0., 0., 1.]);
        editor.current_mut().camera = editor.viewport.camera.clone();
        frame(&mut editor, &context, Vec::new())?;
        settle(&mut editor, &context)?;
        let project = |editor: &Editor, point: Point3| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or("Ponto sem projeção")
        };
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        // Sem peça em construção não há onde guardar o plano.
        editor.current_mut().active_part = None;
        settle(&mut editor, &context)?;
        let button = label_position(&mut editor, &context, text(T::OffsetPlane))?;
        click(&mut editor, &context, button)?;
        assert!(editor.work_plane_edit.is_none());
        editor.current_mut().active_part = Some(part);
        settle(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        // Esc sem plano-base encerra a ferramenta.
        click(&mut editor, &context, button)?;
        assert!(editor.work_plane_edit.is_some());
        assert!(
            editor
                .current()
                .plane_pick
                .as_ref()
                .is_some_and(|p| p.purpose == PlanePurpose::OffsetBase)
        );
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        assert!(editor.work_plane_edit.is_none() && editor.current().plane_pick.is_none());
        // A face superior da placa (z = 10) vira o plano-base pela consulta ao B-rep.
        click(&mut editor, &context, button)?;
        let face = project(&editor, [-25., -20., 10.])?;
        click(&mut editor, &context, face)?;
        settle(&mut editor, &context)?;
        frame(&mut editor, &context, Vec::new())?;
        assert!(editor.current().plane_pick.is_none());
        assert!(editor.offset_plane_has_base());
        label_position(
            &mut editor,
            &context,
            &format!("{} (Corpo 1)", text(T::PickedFace)),
        )?;
        // Com a normal voltada para a câmera, a alça fica acima do centro, a 10 mm.
        let start = project(&editor, [0., 0., 10.])?;
        let pixels_per_mm = start.distance(project(&editor, [1., 0., 10.])?);
        let handle = start + egui::vec2(0., -10. * pixels_per_mm);
        let moved = handle + egui::vec2(0., -40.);
        for (position, pressed) in [(handle, Some(true)), (moved, None), (moved, Some(false))] {
            let mut events = vec![egui::Event::PointerMoved(position)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            frame(&mut editor, &context, events)?;
        }
        let dragged = editor
            .work_plane_edit
            .as_ref()
            .ok_or("Painel ausente")?
            .offset;
        let expected = 10. + f64::from(40. / pixels_per_mm);
        assert!((dragged - expected).abs() < 0.05, "{dragged} ≠ {expected}");
        assert_eq!(editor.current().session.revision(), revision);
        // A distância digitada substitui a do arraste; Enter conclui o campo e depois o OK.
        let field = label_position(
            &mut editor,
            &context,
            &nator_drawing::format_fixed(dragged, 2),
        )?;
        click(&mut editor, &context, field)?;
        frame(&mut editor, &context, vec![egui::Event::Text("25".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        assert!(
            editor.work_plane_edit.is_some(),
            "Enter no campo não confirma"
        );
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        assert!(editor.work_plane_edit.is_none(), "{}", editor.status);
        assert_eq!(editor.current().session.revision(), revision + 1);
        let document = editor.current().session.document();
        let planes = &document.part(part).ok_or("Peça ausente")?.work_planes;
        assert_eq!(planes.len(), 1);
        let plane = planes[0].clone();
        assert_eq!(plane.name, format!("{} 1", text(T::WorkPlaneName)));
        assert!(plane.visible);
        assert!(
            nator_geometry::norm(nator_geometry::sub(
                plane.plane.plane().origin,
                [0., 0., 35.]
            )) < 1e-9
        );
        // A árvore lista a pasta e o plano.
        label_position(
            &mut editor,
            &context,
            &format!("{} (1)", text(T::WorkPlanesFolder)),
        )?;
        label_position(&mut editor, &context, &plane.name)?;
        // No Novo rascunho, o plano visível da peça é clicável acima da placa.
        let button = label_position(&mut editor, &context, text(T::NewSketch))?;
        click(&mut editor, &context, button)?;
        assert!(
            editor
                .current()
                .plane_pick
                .as_ref()
                .is_some_and(|p| p.purpose == PlanePurpose::Sketch)
        );
        let target = project(&editor, [-25., -25., 35.])?;
        click(&mut editor, &context, target)?;
        settle(&mut editor, &context)?;
        let work = editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Rascunho não abriu")?;
        let document = editor.current().session.document();
        let support = document
            .sketch(work.id)
            .ok_or("Rascunho ausente")?
            .sketch
            .plane;
        assert!(nator_geometry::norm(nator_geometry::sub(support.origin, [0., 0., 35.])) < 1e-9);
        assert!(
            document.work_plane(plane.id).is_some_and(|w| !w.visible),
            "O plano usado deve ficar oculto"
        );
        assert_eq!(editor.current().session.revision(), revision + 2);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn editing_a_work_plane_from_the_tree_moves_its_extrusion() -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut session = Session::new(Document::new("Edição de planos")?)?;
        let Effect::PartCreated { part } = session.execute(Command::CreatePart {
            name: "Suporte".into(),
        })?
        else {
            return Err("Peça ausente".into());
        };
        let Effect::WorkPlaneCreated { plane, .. } = session.execute(Command::AddWorkPlane {
            part,
            name: format!("{} 1", text(T::WorkPlaneName)),
            plane: nator_document::WorkPlane {
                base: Plane::xy(),
                offset: 20.,
            },
        })?
        else {
            return Err("Plano ausente".into());
        };
        let Effect::SketchCreated { sketch, .. } =
            session.execute(Command::AddSketchOnWorkPlane {
                id: plane,
                name: "Perfil".into(),
            })?
        else {
            return Err("Rascunho ausente".into());
        };
        let mut profile = session
            .document()
            .sketch(sketch)
            .ok_or("Rascunho ausente")?
            .sketch
            .clone();
        profile.add_rectangle([0., 0.], [20., 10.], false)?;
        session.execute(Command::UpdateSketch {
            id: sketch,
            sketch: profile,
        })?;
        let Effect::Imported { body, .. } = session.execute(Command::AddFeature {
            part,
            name: "Placa".into(),
            operation: Operation::Extrude {
                sketch,
                region: RegionRef::Index(0),
                from: "0 mm".into(),
                to: "5 mm".into(),
                combine: None,
                until: None,
            },
        })?
        else {
            return Err("Extrusão ausente".into());
        };
        let mut editor = Editor::from_gpu(gpu_state()?, &context, session.document().clone())
            .map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.shell.tab = shell::RibbonTab::Model;
        editor.current_mut().active_part = Some(part);
        frame(&mut editor, &context, Vec::new())?;
        settle(&mut editor, &context)?;
        let heights = |editor: &Editor| -> Result<[f64; 2], Box<dyn Error>> {
            let bounds = editor
                .current()
                .session
                .document()
                .body(body)
                .ok_or("Corpo ausente")?
                .geometry
                .mesh()
                .bounds();
            Ok([bounds.min[2], bounds.max[2]])
        };
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let name = format!("{} 1", text(T::WorkPlaneName));
        let double_click = |editor: &mut Editor| -> Result<(), Box<dyn Error>> {
            // Meio segundo de quadros vazios: o par anterior não vira clique triplo.
            for _ in 0..30 {
                frame(editor, &context, Vec::new())?;
            }
            let entry = label_position(editor, &context, &name)?;
            click(editor, &context, entry)?;
            click(editor, &context, entry)
        };
        assert!((heights(&editor)?[0] - 20.).abs() < 1e-6);
        let revision = editor.current().session.revision();
        // Duplo clique na árvore reabre o plano com a distância gravada; OK sem mudança
        // não cria ação no desfazer.
        double_click(&mut editor)?;
        assert_eq!(editor.editing_work_plane(), Some(plane));
        label_position(
            &mut editor,
            &context,
            &format!("{} · {name}", text(T::OffsetPlane)).to_uppercase(),
        )?;
        label_position(&mut editor, &context, text(T::CurrentBase))?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        assert!(editor.work_plane_edit.is_none());
        assert_eq!(editor.current().session.revision(), revision);
        // Nova distância: o rascunho apoiado e a extrusão acompanham o plano.
        double_click(&mut editor)?;
        let field = label_position(&mut editor, &context, "20,00")?;
        click(&mut editor, &context, field)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("32,5".into())],
        )?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        assert!(
            editor.work_plane_edit.is_some(),
            "Enter no campo não confirma"
        );
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        assert!(editor.work_plane_edit.is_none(), "{}", editor.status);
        assert_eq!(editor.current().session.revision(), revision + 1);
        let document = editor.current().session.document();
        assert_eq!(
            document.work_plane(plane).map(|w| w.plane.offset),
            Some(32.5)
        );
        assert!(document.work_plane(plane).is_some_and(|w| !w.visible));
        let [low, high] = heights(&editor)?;
        assert!((low - 32.5).abs() < 1e-6 && (high - 37.5).abs() < 1e-6);
        // Uma única ação de desfazer devolve plano, rascunho e extrusão.
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
        )?;
        settle(&mut editor, &context)?;
        let [low, high] = heights(&editor)?;
        assert!((low - 20.).abs() < 1e-6 && (high - 25.).abs() < 1e-6);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn dimension_tool_types_follows_places_and_cancels_like_solidcraft()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, Curve};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Cotas", Plane::xy());
        let line = sketch.add_line([0., 0.], [12., 16.], false)?;
        let circle = sketch.add_circle([40., 0.], 5., false)?;
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        let mut document = Document::new("Ferramenta Cota")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let at = |editor: &Editor, x: f64, y: f64| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                [x, y, 0.],
            )
            .ok_or("Ponto sem projeção")
        };
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let plain = egui::Modifiers::NONE;
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let value_id = egui::Id::new("sketch_dimension_value");
        let sketch_of = |editor: &Editor| -> Result<Sketch, Box<dyn Error>> {
            Ok(editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .clone())
        };
        let live = |editor: &Editor, context: &egui::Context| {
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.dimension_state().0)
                && context.memory(|m| m.focused()) == Some(value_id)
        };
        // D ativa a Cota.
        frame(&mut editor, &context, vec![key(egui::Key::D, plain)])?;
        assert_eq!(
            editor.current().sketch_edit.as_ref().map(|w| w.tool()),
            Some(sketch_editor::Tool::Dimension)
        );
        let revision = editor.current().session.revision();
        // Clique na linha: a caixa recebe o teclado na hora, sem mudar o documento.
        let target = at(&editor, 6., 8.)?;
        click(&mut editor, &context, target)?;
        frame(&mut editor, &context, Vec::new())?;
        assert!(live(&editor, &context));
        assert_eq!(editor.current().session.revision(), revision);
        // Acima da linha inclinada, o texto que segue o cursor mede a horizontal.
        let target = at(&editor, 6., 25.)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(target)],
        )?;
        frame(&mut editor, &context, vec![egui::Event::Text("10".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter, plain)])?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision + 1);
        let solved = sketch_of(&editor)?;
        let dimension = solved
            .constraints
            .iter()
            .find(|c| c.kind.is_dimension())
            .ok_or("Cota ausente")?;
        assert!(
            matches!(dimension.kind, ConstraintKind::DeltaX { value, .. } if (value - 10.).abs() < 1e-9),
            "{:?}",
            dimension.kind
        );
        assert_eq!(
            solved
                .dimension_expressions
                .get(&dimension.id)
                .map(String::as_str),
            Some("10")
        );
        assert!(solved.dimension_labels.contains_key(&dimension.id));
        let span = (solved.point(end)?[0] - solved.point(start)?[0]).abs();
        assert!((span - 10.).abs() < 1e-6, "{span}");
        // A ferramenta continua ativa e sem escolhas, pronta para a próxima cota.
        let work = editor.current().sketch_edit.as_ref().ok_or("Edição")?;
        assert_eq!(work.tool(), sketch_editor::Tool::Dimension);
        assert_eq!(work.dimension_state().1, 0);
        frame(&mut editor, &context, vec![key(egui::Key::Z, ctrl)])?;
        settle(&mut editor, &context)?;
        assert!(sketch_of(&editor)?.constraints.is_empty());
        let revision = editor.current().session.revision();

        // Esc descarta escolhas e valor, sem tocar no documento.
        let target = at(&editor, 6., 8.)?;
        click(&mut editor, &context, target)?;
        frame(&mut editor, &context, Vec::new())?;
        assert!(live(&editor, &context));
        frame(&mut editor, &context, vec![egui::Event::Text("25".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Escape, plain)])?;
        frame(&mut editor, &context, Vec::new())?;
        assert_eq!(editor.current().session.revision(), revision);
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .map(|w| w.dimension_state().1),
            Some(0)
        );

        assert!(editor.selection_active());
        frame(&mut editor, &context, vec![key(egui::Key::D, plain)])?;
        // Lugar primeiro: o clique ao lado fixa o texto e a cota passa a medir a vertical.
        let target = at(&editor, 6., 8.)?;
        click(&mut editor, &context, target)?;
        frame(&mut editor, &context, Vec::new())?;
        let target = at(&editor, -10., 1.)?;
        click(&mut editor, &context, target)?;
        frame(&mut editor, &context, Vec::new())?;
        assert!(
            live(&editor, &context),
            "o clique no vazio devolve o teclado"
        );
        let target = at(&editor, 30., 30.)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(target)],
        )?;
        frame(&mut editor, &context, vec![egui::Event::Text("18".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter, plain)])?;
        settle(&mut editor, &context)?;
        let solved = sketch_of(&editor)?;
        let vertical = solved
            .constraints
            .iter()
            .find(|c| matches!(c.kind, ConstraintKind::DeltaY { .. }))
            .ok_or("Cota vertical ausente")?;
        let span = (solved.point(end)?[1] - solved.point(start)?[1]).abs();
        assert!((span - 18.).abs() < 1e-6, "{span}");
        // O texto ficou onde foi fixado, não onde o cursor parou.
        let offset = solved
            .dimension_labels
            .get(&vertical.id)
            .ok_or("Texto sem posição")?;
        assert!(offset[0] < 0., "{offset:?}");

        // Um círculo completo recebe diâmetro.
        let target = at(&editor, 45., 0.)?;
        click(&mut editor, &context, target)?;
        frame(&mut editor, &context, Vec::new())?;
        label_position(&mut editor, &context, "10 mm")?;
        frame(&mut editor, &context, vec![egui::Event::Text("12".into())])?;
        frame(&mut editor, &context, vec![key(egui::Key::Enter, plain)])?;
        settle(&mut editor, &context)?;
        let solved = sketch_of(&editor)?;
        assert!(solved.constraints.iter().any(|c| matches!(
            c.kind,
            ConstraintKind::Diameter { circle: target, value } if target == circle && (value - 12.).abs() < 1e-9
        )));
        assert!((solved.radial(circle)?.1 - 6.).abs() < 1e-6);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn project_tool_clicks_create_lilac_references_with_undo_and_escape()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut solid = nator_kernel::Solid::cuboid([-20., -10., 0.], [40., 20., 5.])?;
        let (mesh, faces) = solid.tessellate_with_faces(0.1, 0.2)?;
        let mut document = Document::new("Projetar na interface")?;
        let (part, body) = document.add_mesh(
            None,
            "Bloco".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
        let plane = Plane {
            origin: [0., 0., 20.],
            ..Plane::xy()
        };
        document.body_mut(body)?.face_names = Arc::new(nator_kernel::indexed_face_names(
            &nator_document::features::body_owner(body),
            solid.face_count()?,
        ));
        let (_, id) = document.add_sketch(Some(part), Sketch::new("Destino", plane))?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let tool = label_position(&mut editor, &context, text(T::SketchProject))?;
        click(&mut editor, &context, tool)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !editor
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.projection_ready())
        {
            if Instant::now() > deadline {
                return Err("Projetar não terminou a consulta".into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let at = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [0., 0., 20.],
        )
        .ok_or("Ponto sem projeção")?;
        let revision = editor.current().session.revision();
        let wait_selection = |editor: &mut Editor, count: usize| -> Result<(), Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(10);
            while editor
                .current()
                .sketch_edit
                .as_ref()
                .is_none_or(|w| w.projection_selected() != count)
            {
                if Instant::now() > deadline {
                    return Err("Seleção projetada não foi atualizada".into());
                }
                frame(editor, &context, vec![])?;
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(())
        };
        click(&mut editor, &context, at)?;
        wait_selection(&mut editor, 1)?;
        assert_eq!(editor.current().session.revision(), revision);
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .is_empty()
        );
        let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
        click(&mut editor, &context, cancel)?;
        frame(&mut editor, &context, vec![])?;
        assert!(editor.selection_active());
        assert_eq!(editor.current().session.revision(), revision);
        let tool = label_position(&mut editor, &context, text(T::SketchProject))?;
        click(&mut editor, &context, tool)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !editor
            .current()
            .sketch_edit
            .as_ref()
            .is_some_and(|w| w.projection_ready())
        {
            if Instant::now() > deadline {
                return Err("Projetar não reiniciou".into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        click(&mut editor, &context, at)?;
        wait_selection(&mut editor, 1)?;
        click(&mut editor, &context, at)?;
        wait_selection(&mut editor, 0)?;
        click(&mut editor, &context, at)?;
        wait_selection(&mut editor, 1)?;
        let finish = label_position(&mut editor, &context, text(T::ProjectionFinish))?;
        click(&mut editor, &context, finish)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while editor.current().session.revision() == revision {
            if Instant::now() > deadline {
                return Err(format!("Projetar não finalizou: {}", editor.status).into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision + 1);
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(sketch.entities.len(), 4);
        assert!(sketch.entities.iter().all(|e| e.projected));
        while editor
            .current()
            .sketch_edit
            .as_ref()
            .is_none_or(|w| w.projected_region_count() != 1)
        {
            if Instant::now() > deadline {
                return Err("Preenchimento projetado não apareceu".into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        let primitives =
            context.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
        let colors = primitives
            .iter()
            .filter_map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => Some(m),
                _ => None,
            })
            .flat_map(|m| &m.vertices)
            .map(|v| v.color)
            .collect::<Vec<_>>();
        assert!(colors.contains(&theme::PROJECTED));
        assert!(colors.contains(&theme::PROJECTED_FILL));
        output.textures_delta.clear();
        let revision = editor.current().session.revision();
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert!(editor.selection_active());
        assert_eq!(editor.current().session.revision(), revision);
        assert!(editor.current_mut().session.undo());
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .is_empty()
        );
        assert!(editor.current_mut().session.redo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            4
        );
        // Artefato opcional para revisar o mesmo traço/preenchimento que o ensaio
        // verificou. O caminho vem do executor; a rotina normal não grava imagens.
        if let Some(directory) = std::env::var_os("NATOR_TEST_CAPTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)?;
            let project = directory.join("projected-sketch.3dnator");
            write_project(&project, editor.current().session.document())?;
            render_with(
                &project,
                &directory.join("projected-sketch.png"),
                [1280, 820],
                |editor| {
                    open_first_sketch(editor)?;
                    editor.select_sketch_tool(sketch_editor::Tool::Project);
                    Ok(())
                },
                |_, _| vec![],
                |editor| {
                    editor
                        .current()
                        .sketch_edit
                        .as_ref()
                        .is_some_and(|w| w.projected_region_count() == 1)
                },
            )?;
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn junction_f_key_and_vertex_padlocks_publish_the_selected_strength()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{Curve, junctions};
        let gpu = gpu_state()?;
        // F também precisa funcionar quando as medidas flutuantes têm teclado.
        for (crossing, toggle, expected) in [
            (false, false, true),
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            let context = egui::Context::default();
            let mut sketch = Sketch::new("Junções", Plane::xy());
            let horizontal = sketch.add_line([-30., 8.], [30., 8.], false)?;
            sketch.add_line([10., -30.], [10., 30.], false)?;
            let Curve::Line { start: target, .. } = sketch.entity(horizontal)?.curve else {
                return Err("Linha ausente".into());
            };
            let mut document = Document::new("Força das junções")?;
            let (part, id) = document.add_sketch(None, sketch)?;
            let mut editor =
                Editor::from_gpu(gpu.clone(), &context, document).map_err(|e| e.to_string())?;
            editor.recoveries.clear();
            editor.show_recovery = false;
            editor.begin_sketch(part, id);
            editor.align_sketch_camera(true, false);
            settle(&mut editor, &context)?;
            editor.select_sketch_tool(sketch_editor::Tool::Line);
            frame(&mut editor, &context, vec![])?;
            let screen = |editor: &Editor, p: [f64; 2]| {
                sketch_scene::project(
                    &editor.viewport.camera,
                    editor.display_bounds(),
                    editor.canvas_rect,
                    [p[0], p[1], 0.],
                )
                .ok_or("Ponto sem projeção")
            };
            let first = screen(&editor, [-20., 25.])?;
            click(&mut editor, &context, first)?;
            let position = if crossing { [10., 8.] } else { [-30., 8.] };
            let end = screen(&editor, position)?;
            frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
            if toggle {
                frame(
                    &mut editor,
                    &context,
                    vec![
                        egui::Event::Key {
                            key: egui::Key::F,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::Text("f".into()),
                    ],
                )?;
            }
            click(&mut editor, &context, end)?;
            settle(&mut editor, &context)?;
            let current = || -> Result<Sketch, Box<dyn Error>> {
                Ok(editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
                    .clone())
            };
            let sketch = current()?;
            assert_eq!(sketch.entities.len(), 3, "{}", editor.status);
            let created = sketch.entities.last().ok_or("Linha ausente")?;
            let Curve::Line { end: endpoint, .. } = created.curve else {
                return Err("Linha ausente".into());
            };
            let link = sketch.constraints.iter().find(|c| if crossing { matches!(c.kind, nator_sketch::ConstraintKind::PointOnLine { point,.. } if point == endpoint) }
                else { matches!(c.kind,nator_sketch::ConstraintKind::Coincident { a,b } if (a == endpoint && b == target) || (b == endpoint && a == target)) }).ok_or("Ligação não foi criada")?;
            assert_eq!(!link.reference, expected);
            assert!(editor.current().sketch_edit.as_ref().is_some_and(|w| {
                w.drawing_fields()
                    .iter()
                    .all(|(text, _)| !text.eq_ignore_ascii_case("f"))
            }));
            editor.activate_selection(&context);
            frame(&mut editor, &context, vec![])?;
            let at = screen(&editor, position)?;
            click(&mut editor, &context, at)?;
            frame(&mut editor, &context, vec![])?;
            let before = editor.current().session.revision();
            if !crossing && let Some(directory) = std::env::var_os("NATOR_JUNCTION_CAPTURE_DIR") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory)?;
                let name = if expected {
                    "strong-junction"
                } else {
                    "weak-junction"
                };
                let project = directory.join(format!("{name}.3dnator"));
                write_project(&project, editor.current().session.document())?;
                render_with(
                    &project,
                    &directory.join(format!("{name}.png")),
                    [1280, 820],
                    |editor| {
                        open_first_sketch(editor)?;
                        if let Some(work) = editor.current_mut().sketch_edit.as_mut() {
                            work.select_vertex_for_capture(position);
                        }
                        Ok(())
                    },
                    |_, _| vec![],
                    |_| true,
                )?;
            }
            // O botão mostra a ação oposta ao estado: abrir o forte, fechar o fraco.
            let button = at + egui::vec2(26., -18.);
            click(&mut editor, &context, button)?;
            settle(&mut editor, &context)?;
            let sketch = editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .clone();
            assert_eq!(editor.current().session.revision(), before + 1);
            assert_eq!(
                junctions::at(&sketch, position)?
                    .ok_or("Junção ausente")?
                    .strong,
                !expected
            );
            assert!(editor.current_mut().session.undo());
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert_eq!(
                junctions::at(sketch, position)?
                    .ok_or("Junção ausente")?
                    .strong,
                expected
            );
            assert!(editor.current_mut().session.redo());
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn existing_sketch_dimensions_drag_edit_and_cancel_without_recreating_constraints()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, Curve};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Cota editável", Plane::xy());
        let line = sketch.add_line([-20., 0.], [20., 0.], false)?;
        let Curve::Line { start, end } = sketch.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: start,
                position: [-20., 0.],
            },
            false,
        )?;
        sketch.add_constraint(ConstraintKind::Horizontal { line }, false)?;
        let dimension = sketch.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 40.,
            },
            false,
        )?;
        sketch
            .dimension_expressions
            .insert(dimension, "40 mm".into());
        sketch.dimension_labels.insert(dimension, [0., 12.]);
        let original_points = sketch.points.iter().map(|p| p.position).collect::<Vec<_>>();
        let mut document = Document::new("Editar e mover cotas")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let at = label_position(&mut editor, &context, "40")?;
        let destination = at + egui::vec2(65., 30.);
        let button = |position, pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(at), button(at, true)],
        )?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(destination)],
        )?;
        frame(&mut editor, &context, vec![button(destination, false)])?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_ne!(sketch.dimension_labels.get(&dimension), Some(&[0., 12.]));
        assert_eq!(
            sketch.points.iter().map(|p| p.position).collect::<Vec<_>>(),
            original_points
        );
        let offset = sketch.dimension_labels[&dimension];
        let at = label_position(&mut editor, &context, "40")?;
        click(&mut editor, &context, at)?;
        click(&mut editor, &context, at)?;
        frame(&mut editor, &context, vec![])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("60 mm".into())],
        )?;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(sketch.constraints.len(), 3);
        assert!(
            (nator_sketch::distance(sketch.point(start)?, sketch.point(end)?) - 60.).abs() < 1e-6,
            "{}",
            editor.status
        );
        assert_eq!(sketch.dimension_labels[&dimension], offset);
        let revision = editor.current().session.revision();
        let at = label_position(&mut editor, &context, "60")?;
        click(&mut editor, &context, at)?;
        click(&mut editor, &context, at)?;
        frame(&mut editor, &context, vec![])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("80 mm".into())],
        )?;
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        assert_eq!(editor.current().session.revision(), revision);
        assert!(editor.current_mut().session.undo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .constraints
                .iter()
                .find(|c| c.id == dimension)
                .and_then(|c| c.kind.dimension_value()),
            Some(40.)
        );
        assert!(editor.current_mut().session.redo());
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_mirror_fields_accept_multiple_origins_and_offsets_offer_two_sides()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Ferramentas", Plane::xy());
        sketch.add_line([-25., -10.], [-5., -10.], false)?;
        sketch.add_line([-5., -10.], [-5., 5.], false)?;
        sketch.add_line([0., -20.], [0., 20.], true)?;
        let mut document = Document::new("Espelhar e deslocar")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        editor.open_sketch_action(&context, nator_sketch::edit::Action::Mirror);
        frame(&mut editor, &context, vec![])?;
        let at = |editor: &Editor, p: [f64; 3]| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                p,
            )
            .ok_or("Ponto sem projeção")
        };
        for p in [[-15., -10., 0.], [-5., 0., 0.]] {
            let position = at(&editor, p)?;
            click(&mut editor, &context, position)?;
        }
        let axis_label = label_position(&mut editor, &context, text(T::SketchMirrorAxis))?;
        click(&mut editor, &context, axis_label + egui::vec2(125., 0.))?;
        let position = at(&editor, [0., 10., 0.])?;
        click(&mut editor, &context, position)?;
        let construction = label_position(&mut editor, &context, text(T::Construction))?;
        click(&mut editor, &context, construction)?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(sketch.entities.len(), 5, "{}", editor.status);
        assert!(sketch.entities[3..].iter().all(|e| e.construction));
        editor.activate_selection(&context);
        let position = at(&editor, [-15., -10., 0.])?;
        click(&mut editor, &context, position)?;
        editor.open_sketch_action(&context, nator_sketch::edit::Action::Offset);
        frame(&mut editor, &context, vec![])?;
        let double = label_position(&mut editor, &context, text(T::SketchOffsetDouble))?;
        click(&mut editor, &context, double)?;
        label_position(&mut editor, &context, text(T::SketchOffsetSecond))?;
        let value = label_position(&mut editor, &context, "20 mm")?;
        click(&mut editor, &context, value)?;
        frame(
            &mut editor,
            &context,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                },
                egui::Event::Text("2 mm".into()),
            ],
        )?;
        wait_sketch_operation(&mut editor, &context)?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            9,
            "{}",
            editor.status
        );
        assert!(editor.current_mut().session.undo());
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            5
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn dimension_equal_connections_keep_the_second_clicked_center_stationary()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, dimensions::Target};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Ordem das escolhas", Plane::xy());
        let first = sketch.add_circle([-15., 0.], 4., false)?;
        let second = sketch.add_circle([15., 0.], 4., false)?;
        let a = sketch.entity(first)?.point_ids()[0];
        let b = sketch.entity(second)?.point_ids()[0];
        let mut document = Document::new("Empate nas conexões")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        editor.select_sketch_tool(sketch_editor::Tool::Dimension);
        frame(&mut editor, &context, vec![])?;
        for point in [[-15., 0., 0.], [15., 0., 0.]] {
            let at = sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or("Ponto sem projeção")?;
            click(&mut editor, &context, at)?;
        }
        frame(&mut editor, &context, vec![])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Text("40 mm".into())],
        )?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let dimension = sketch
            .constraints
            .iter()
            .find(|c| matches!(c.kind, ConstraintKind::Distance { .. }))
            .ok_or("Cota ausente")?;
        assert_eq!(
            sketch.dimension_targets.get(&dimension.id),
            Some(&[Target::Point(a), Target::Point(b)])
        );
        assert_eq!(sketch.point(b)?, [15., 0.]);
        assert!((sketch.point(a)?[0] + 25.).abs() < 1e-6);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_context_shortcuts_delete_connected_selection_and_clear_undo_targets()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::edit::Action as Edit;
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Seleção e atalhos", Plane::xy());
        sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        sketch.add_circle([35., 5.], 3., false)?;
        let mut document = Document::new("Atalhos por contexto")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let at = |editor: &Editor, p| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                p,
            )
            .ok_or("Ponto sem projeção")
        };
        let line = at(&editor, [10., 0., 0.])?;
        click(&mut editor, &context, line)?;
        click(&mut editor, &context, line)?;
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .selected_entities()
                .len(),
            4
        );
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::X, egui::Modifiers::NONE)],
        )?;
        settle(&mut editor, &context)?;
        assert!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities[..4]
                .iter()
                .all(|e| e.construction)
        );
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::Delete, egui::Modifiers::NONE)],
        )?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            1
        );
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::Z, egui::Modifiers::CTRL)],
        )?;
        settle(&mut editor, &context)?;
        let mut next = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .clone();
        let added = next.add_line([25., -10.], [35., -10.], false)?;
        editor
            .current_mut()
            .session
            .execute(Command::UpdateSketch { id, sketch: next })?;
        editor.current_mut().changed();
        editor.invalidate_scene();
        settle(&mut editor, &context)?;
        let point = at(&editor, [30., -10., 0.])?;
        click(&mut editor, &context, point)?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .selected_entities()
                .contains(&added)
        );
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::Z, egui::Modifiers::CTRL)],
        )?;
        settle(&mut editor, &context)?;
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::E, egui::Modifiers::NONE)],
        )?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .selected_entities()
                .is_empty()
        );
        editor.activate_selection(&context);
        for (letter, tool) in [
            (egui::Key::L, sketch_editor::Tool::Line),
            (egui::Key::R, sketch_editor::Tool::Rectangle),
            (egui::Key::C, sketch_editor::Tool::Circle),
            (egui::Key::A, sketch_editor::Tool::Arc),
            (egui::Key::P, sketch_editor::Tool::Project),
            (egui::Key::G, sketch_editor::Tool::Trim),
            (egui::Key::D, sketch_editor::Tool::Dimension),
        ] {
            frame(
                &mut editor,
                &context,
                vec![key(letter, egui::Modifiers::NONE)],
            )?;
            assert_eq!(
                editor
                    .current()
                    .sketch_edit
                    .as_ref()
                    .ok_or("Editor ausente")?
                    .tool(),
                tool
            );
        }
        for (letter, action) in [
            (egui::Key::S, Edit::Offset),
            (egui::Key::O, Edit::Fillet),
            (egui::Key::H, Edit::Chamfer),
            (egui::Key::E, Edit::Mirror),
        ] {
            frame(
                &mut editor,
                &context,
                vec![key(letter, egui::Modifiers::NONE)],
            )?;
            assert_eq!(
                editor
                    .current()
                    .sketch_edit
                    .as_ref()
                    .ok_or("Editor ausente")?
                    .active_action(),
                Some(action)
            );
            assert!(!editor.shell.search_open);
        }
        editor.activate_selection(&context);
        editor.finish_sketch();
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::S, egui::Modifiers::NONE)],
        )?;
        assert!(editor.shell.search_open);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_edge_drag_and_offset_arrow_preview_cancel_and_confirm() -> Result<(), Box<dyn Error>>
    {
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Mover contorno", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let first = lines[0];
        let mut document = Document::new("Prévias e mouse")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let at = |editor: &Editor, p| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                p,
            )
            .ok_or("Ponto sem projeção")
        };
        let point = at(&editor, [10., 0., 0.])?;
        click(&mut editor, &context, point)?;
        click(&mut editor, &context, point)?;
        let revision = editor.current().session.revision();
        let destination = at(&editor, [13., 2., 0.])?;
        drag_between(&mut editor, &context, point, destination)?;
        wait_sketch_drag(&mut editor, &context)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision + 1);
        let (a, b) = editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch
            .line(first)?;
        assert!(
            nator_sketch::distance(a, [3., 2.]) < 1e-4
                && nator_sketch::distance(b, [23., 2.]) < 1e-4
        );
        editor.activate_selection(&context);
        let point = at(&editor, [13., 2., 0.])?;
        click(&mut editor, &context, point)?;
        frame(&mut editor, &context, vec![key(egui::Key::S)])?;
        assert_eq!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .selected_entities()
                .len(),
            4
        );
        frame(&mut editor, &context, vec![egui::Event::Text("-2".into())])?;
        let p = at(&editor, [0., 0., 0.])?;
        let q = at(&editor, [0., -1., 0.])?;
        drag_operation_arrow(&mut editor, &context, 0, q - p)?;
        frame(
            &mut editor,
            &context,
            vec![key(egui::Key::X), egui::Event::Text("x".into())],
        )?;
        wait_sketch_operation(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .entities
                .len(),
            4
        );
        frame(&mut editor, &context, vec![key(egui::Key::Enter)])?;
        for _ in 0..4 {
            frame(&mut editor, &context, vec![])?;
        }
        settle(&mut editor, &context)?;
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(
            editor.current().session.revision(),
            revision + 1,
            "{}",
            editor.status
        );
        assert_eq!(sketch.entities.len(), 8);
        assert!(sketch.entities[4..].iter().all(|e| e.construction));
        editor.activate_selection(&context);
        frame(&mut editor, &context, vec![key(egui::Key::S)])?;
        wait_sketch_operation(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        assert_eq!(editor.current().session.revision(), revision);
        assert!(
            !editor
                .current()
                .sketch_edit
                .as_ref()
                .ok_or("Editor ausente")?
                .operation_ready()
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn fillet_chamfer_and_mirror_previews_are_translucent_and_cancel_without_edits()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{Curve, edit::Action as Edit};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Prévias", Plane::xy());
        let a = sketch.add_point([-10., 0.])?;
        let joint = sketch.add_point([0., 0.])?;
        let b = sketch.add_point([0., 10.])?;
        sketch.add_curve(
            Curve::Line {
                start: a,
                end: joint,
            },
            false,
        )?;
        sketch.add_curve(
            Curve::Line {
                start: joint,
                end: b,
            },
            false,
        )?;
        sketch.add_line([-20., -15.], [-20., 15.], true)?;
        let mut document = Document::new("Prévias sem publicação")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let at = |editor: &Editor, p| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                p,
            )
            .ok_or("Ponto sem projeção")
        };
        let point = at(&editor, [-5., 0., 0.])?;
        click(&mut editor, &context, point)?;
        click(&mut editor, &context, point)?;
        let revision = editor.current().session.revision();
        for action in [Edit::Fillet, Edit::Chamfer, Edit::Mirror] {
            editor.open_sketch_action(&context, action);
            frame(&mut editor, &context, vec![])?;
            if action == Edit::Mirror {
                let label = label_position(&mut editor, &context, text(T::SketchMirrorAxis))?;
                click(&mut editor, &context, label + egui::vec2(125., 0.))?;
                let point = at(&editor, [-20., 5., 0.])?;
                click(&mut editor, &context, point)?;
            } else {
                let value = label_position(
                    &mut editor,
                    &context,
                    if action == Edit::Fillet {
                        "10 mm"
                    } else {
                        "2 mm"
                    },
                )?;
                click(&mut editor, &context, value)?;
                frame(
                    &mut editor,
                    &context,
                    vec![
                        egui::Event::Key {
                            key: egui::Key::A,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers {
                                ctrl: true,
                                command: true,
                                ..Default::default()
                            },
                        },
                        egui::Event::Text("2 mm".into()),
                    ],
                )?;
            }
            wait_sketch_operation(&mut editor, &context)?;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280., 820.),
                    )),
                    ..Default::default()
                },
                |ui| editor.draw(ui),
            );
            let primitives =
                context.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
            let preview_color = egui::Color32::from_rgba_unmultiplied(75, 105, 210, 145);
            assert!(primitives.iter().any(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(mesh) =>
                    mesh.vertices.iter().any(|v| v.color == preview_color),
                _ => false,
            }));
            output.textures_delta.clear();
            assert_eq!(editor.current().session.revision(), revision);
            assert_eq!(
                editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
                    .entities
                    .len(),
                3
            );
            editor.activate_selection(&context);
            frame(&mut editor, &context, vec![])?;
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn double_offset_arrows_adjust_both_sides_before_one_commit() -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Dois lados", Plane::xy());
        sketch.add_line([0., 0.], [20., 0.], false)?;
        let mut document = Document::new("Distâncias pelo mouse")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let at = |editor: &Editor, p| {
            sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                p,
            )
            .ok_or("Ponto sem projeção")
        };
        let p = at(&editor, [10., 0., 0.])?;
        click(&mut editor, &context, p)?;
        editor.open_sketch_action(&context, nator_sketch::edit::Action::Offset);
        frame(&mut editor, &context, vec![])?;
        let double = label_position(&mut editor, &context, text(T::SketchOffsetDouble))?;
        click(&mut editor, &context, double)?;
        // A medida só muda ao arrastar a ponta, sem confirmar por clique no vazio.
        // A primeira entrada recebe 2 mm para manter ambas as setas no canvas.
        let value_field = egui::Id::new("sketch_offset_distance");
        context.memory_mut(|m| m.request_focus(value_field));
        frame(
            &mut editor,
            &context,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                },
                egui::Event::Text("2".into()),
            ],
        )?;
        let unit = at(&editor, [0., 1., 0.])? - at(&editor, [0., 0., 0.])?;
        drag_operation_arrow(&mut editor, &context, 0, unit)?;
        wait_sketch_operation(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), 0);
        drag_operation_arrow(&mut editor, &context, 1, unit * 5.)?;
        wait_sketch_operation(&mut editor, &context)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        for _ in 0..4 {
            frame(&mut editor, &context, vec![])?;
        }
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), 1);
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!(sketch.entities.len(), 3);
        let ys = sketch.entities[1..]
            .iter()
            .map(|e| sketch.line(e.id).map(|(a, _)| a[1]))
            .collect::<Result<Vec<_>, _>>()?;
        assert!(
            ys.iter().any(|v| (*v - 3.).abs() < 1e-4) && ys.iter().any(|v| (*v + 5.).abs() < 1e-4),
            "Deslocamentos: {ys:?}"
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn selected_vertex_f_indicators_and_direct_drag_respect_history_and_fixed_points()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::{ConstraintKind, junctions};
        let context = egui::Context::default();
        let mut sketch = Sketch::new("Apoios", Plane::xy());
        let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
        let fixed_line = sketch.add_line([-15., 0.], [-5., 0.], false)?;
        let fixed = sketch.entity(fixed_line)?.point_ids()[0];
        sketch.add_constraint(
            ConstraintKind::Fixed {
                point: fixed,
                position: [-15., 0.],
            },
            false,
        )?;
        let mut document = Document::new("Arraste e indicadores")?;
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let at = |e: &Editor, p| {
            sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                .ok_or("Ponto sem projeção")
        };
        let visible = |e: &Editor| {
            e.current()
                .sketch_edit
                .as_ref()
                .map(|w| w.visible_indicator_count())
                .unwrap_or(0)
        };
        assert_eq!(visible(&editor), 0);
        let vertex = at(&editor, [0., 0., 0.])?;
        click(&mut editor, &context, vertex)?;
        frame(&mut editor, &context, vec![])?;
        assert_eq!(visible(&editor), 1);
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for strong in [false, true] {
            let revision = editor.current().session.revision();
            frame(
                &mut editor,
                &context,
                vec![key(egui::Key::F), egui::Event::Text("f".into())],
            )?;
            settle(&mut editor, &context)?;
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert_eq!(
                junctions::at(sketch, [0., 0.])?
                    .ok_or("Junção ausente")?
                    .strong,
                strong
            );
            assert_eq!(editor.current().session.revision(), revision + 1);
        }
        let center = at(&editor, [10., 5., 0.])?;
        click(&mut editor, &context, center)?;
        frame(&mut editor, &context, vec![])?;
        assert_eq!(visible(&editor), 1);
        let blank = at(&editor, [-10., 15., 0.])?;
        click(&mut editor, &context, blank)?;
        frame(&mut editor, &context, vec![])?;
        assert_eq!(visible(&editor), 0);
        editor.select_sketch_tool(sketch_editor::Tool::Line);
        for _ in 0..3 {
            frame(&mut editor, &context, vec![])?;
        }
        assert!(visible(&editor) >= 4);
        editor.activate_selection(&context);
        let revision = editor.current().session.revision();
        let to = at(&editor, [2., 3., 0.])?;
        drag_between(&mut editor, &context, vertex, to)?;
        wait_sketch_drag(&mut editor, &context)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor.current().session.revision(),
            revision + 1,
            "{}",
            editor.status
        );
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        let corner = sketch.entity(lines[0])?.point_ids()[0];
        assert!(nator_sketch::distance(sketch.point(corner)?, [2., 3.]) < 1e-4);
        assert!(nator_sketch::solver::max_error(sketch)? < 1e-6);
        assert!(editor.current_mut().session.undo());
        editor.invalidate_scene();
        frame(&mut editor, &context, vec![])?;
        let from = at(&editor, [-15., 0., 0.])?;
        let to = at(&editor, [-14., 2., 0.])?;
        let revision = editor.current().session.revision();
        drag_between(&mut editor, &context, from, to)?;
        wait_sketch_drag(&mut editor, &context)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision);
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .point(fixed)?,
            [-15., 0.]
        );
        // Esc descarta uma prévia em andamento, mesmo com o botão ainda pressionado.
        let from = at(&editor, [0., 0., 0.])?;
        let to = at(&editor, [3., 2., 0.])?;
        frame(
            &mut editor,
            &context,
            vec![
                egui::Event::PointerMoved(from),
                egui::Event::PointerButton {
                    pos: from,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )?;
        frame(&mut editor, &context, vec![egui::Event::PointerMoved(to)])?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.drag_pending())
        );
        frame(&mut editor, &context, vec![key(egui::Key::Escape)])?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )?;
        assert_eq!(editor.current().session.revision(), revision);
        frame(&mut editor, &context, vec![key(egui::Key::M)])?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.active_action().is_none())
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn multiple_corner_refinements_drag_the_arrow_and_commit_atomically()
    -> Result<(), Box<dyn Error>> {
        use nator_sketch::edit::Action;
        for action in [Action::Fillet, Action::Chamfer] {
            let context = egui::Context::default();
            let mut sketch = Sketch::new("Quatro cantos", Plane::xy());
            let lines = sketch.add_rectangle([0., 0.], [20., 10.], false)?;
            let mut document = Document::new("Acabamentos")?;
            let (part, id) = document.add_sketch(None, sketch)?;
            let mut editor =
                Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
            editor.recoveries.clear();
            editor.show_recovery = false;
            editor.begin_sketch(part, id);
            editor.align_sketch_camera(true, false);
            settle(&mut editor, &context)?;
            editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Editor ausente")?
                .configure_operation_capture(action, lines, None);
            frame(&mut editor, &context, vec![])?;
            wait_sketch_operation(&mut editor, &context)?;
            let at = |e: &Editor, p| {
                sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                    .ok_or("Ponto sem projeção")
            };
            let unit = at(&editor, [1., 1., 0.])? - at(&editor, [0., 0., 0.])?;
            let delta = if action == Action::Fillet {
                unit
            } else {
                unit * 0.5
            };
            drag_operation_arrow(&mut editor, &context, 0, delta)?;
            wait_sketch_operation(&mut editor, &context)?;
            // Prévia de 3 mm: nenhuma revisão foi publicada ao soltar a seta.
            assert_eq!(editor.current().session.revision(), 0);
            assert_eq!(
                editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
                    .entities
                    .len(),
                4
            );
            frame(
                &mut editor,
                &context,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            )?;
            for _ in 0..4 {
                frame(&mut editor, &context, vec![])?;
            }
            settle(&mut editor, &context)?;
            assert_eq!(editor.current().session.revision(), 1, "{}", editor.status);
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert_eq!(sketch.entities.len(), 8);
            assert_eq!(nator_sketch::profiles::regions(sketch)?.len(), 1);
            if action == Action::Fillet {
                for e in &sketch.entities[4..] {
                    assert!(
                        (sketch.radial(e.id)?.1 - 3.).abs() < 1e-4,
                        "Raio: {}",
                        sketch.radial(e.id)?.1
                    );
                }
            } else {
                let area = nator_sketch::profiles::regions(sketch)?[0].area();
                assert!((area - 182.).abs() < 1e-3, "{area}");
            }
            assert!(editor.current_mut().session.undo());
            assert_eq!(
                editor
                    .current()
                    .session
                    .document()
                    .sketch(id)
                    .ok_or("Rascunho ausente")?
                    .sketch
                    .entities
                    .len(),
                4
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn arc_mouse_motion_selects_clockwise_and_continues_beyond_half_a_turn()
    -> Result<(), Box<dyn Error>> {
        for angles in [vec![-15_f64, -90., -170., -190., -270.], vec![15_f64, 90.]] {
            let context = egui::Context::default();
            let mut document = Document::new("Sentido do arco")?;
            let (part, id) = document.add_sketch(None, Sketch::new("Arco", Plane::xy()))?;
            let mut editor =
                Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
            editor.recoveries.clear();
            editor.show_recovery = false;
            editor.begin_sketch(part, id);
            editor.align_sketch_camera(true, false);
            settle(&mut editor, &context)?;
            editor.select_sketch_tool(sketch_editor::Tool::Arc);
            frame(&mut editor, &context, vec![])?;
            let at = |e: &Editor, p| {
                sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                    .ok_or("Ponto sem projeção")
            };
            let center = at(&editor, [0., 0., 0.])?;
            click(&mut editor, &context, center)?;
            let start = at(&editor, [20., 0., 0.])?;
            click(&mut editor, &context, start)?;
            let mut end = start;
            for angle in &angles {
                let angle = angle.to_radians();
                end = at(&editor, [20. * angle.cos(), 20. * angle.sin(), 0.])?;
                frame(&mut editor, &context, vec![egui::Event::PointerMoved(end)])?;
            }
            click(&mut editor, &context, end)?;
            settle(&mut editor, &context)?;
            let sketch = &editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch;
            assert_eq!(sketch.entities.len(), 1, "{}", editor.status);
            let nator_sketch::Curve::Arc { clockwise, .. } = sketch.entities[0].curve else {
                return Err("Arco ausente".into());
            };
            assert_eq!(clockwise, angles[0] < 0.);
            let points = nator_sketch::profiles::segment(sketch, &sketch.entities[0])?.sample(96);
            let length = points
                .windows(2)
                .map(|p| nator_sketch::distance(p[0], p[1]))
                .sum::<f64>();
            let expected = if clockwise {
                30. * std::f64::consts::PI
            } else {
                10. * std::f64::consts::PI
            };
            assert!((length - expected).abs() < 0.1, "{length}");
        }
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn image_reference_places_two_corners_drags_resizes_and_keeps_pixels_in_the_project()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut document = Document::new("Imagem no plano")?;
        let (part, sketch) = document.add_sketch(None, Sketch::new("Rascunho", Plane::xy()))?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, sketch);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let asset = Arc::new(nator_document::RasterAsset {
            width: 2,
            height: 2,
            rgba: Arc::new(vec![
                180, 70, 20, 255, 40, 160, 90, 255, 80, 120, 200, 255, 240, 220, 140, 255,
            ]),
        });
        editor.receive_reference_image(
            editor.current().id,
            editor.current().session.revision(),
            sketch,
            asset,
        );
        frame(&mut editor, &context, vec![])?;
        assert_eq!(
            editor
                .image_edit
                .as_ref()
                .ok_or("Painel ausente")?
                .image
                .transparency,
            0.5
        );
        let at = |e: &Editor, p| {
            sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                .ok_or("Ponto sem projeção")
        };
        let first = at(&editor, [0., 0., 0.])?;
        let second = at(&editor, [20., 10., 0.])?;
        click(&mut editor, &context, first)?;
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(second)],
        )?;
        click(&mut editor, &context, second)?;
        assert!(editor.current().session.document().images().is_empty());
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        let image = editor
            .current()
            .session
            .document()
            .images()
            .first()
            .ok_or("Imagem ausente")?
            .clone();
        assert!(nator_sketch::distance(image.size, [20., 10.]) < 1e-4);
        assert_eq!(image.name, "Imagem 1");
        assert_eq!(image.transparency, 0.5);
        let revision = editor.current().session.revision();
        let from = at(&editor, [10., 5., 0.])?;
        let to = at(&editor, [15., 7., 0.])?;
        drag_between(&mut editor, &context, from, to)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor.current().session.revision(),
            revision + 1,
            "{}",
            editor.status
        );
        let image = editor.current().session.document().images()[0].clone();
        assert!(
            nator_sketch::distance(image.origin, [5., 2.]) < 1e-4,
            "{:?}",
            image.origin
        );
        let from = at(&editor, [25., 12., 0.])?;
        let to = at(&editor, [30., 15., 0.])?;
        drag_between(&mut editor, &context, from, to)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision + 2);
        assert!(
            nator_sketch::distance(
                editor.current().session.document().images()[0].size,
                [25., 13.]
            ) < 1e-4
        );
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("imagem.3dnator");
        write_project(&path, editor.current().session.document())?;
        let reopened = read_project(&path)?;
        assert_eq!(reopened.images()[0].asset.rgba.len(), 16);
        assert!(nator_sketch::distance(reopened.images()[0].origin, [5., 2.]) < 1e-4);
        if let Some(directory) = std::env::var_os("NATOR_IMAGE_CAPTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)?;
            write_project(
                &directory.join("image.3dnator"),
                editor.current().session.document(),
            )?;
        }
        // Desfazer/refazer também funciona com as propriedades da imagem abertas.
        for key in [egui::Key::Z, egui::Key::Y] {
            frame(
                &mut editor,
                &context,
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::CTRL,
                }],
            )?;
            settle(&mut editor, &context)?;
            let expected = if key == egui::Key::Z {
                [20., 10.]
            } else {
                [25., 13.]
            };
            assert!(
                nator_sketch::distance(
                    editor.current().session.document().images()[0].size,
                    expected
                ) < 1e-4
            );
            assert!(
                nator_sketch::distance(
                    editor
                        .image_edit
                        .as_ref()
                        .ok_or("Painel ausente")?
                        .image
                        .size,
                    expected
                ) < 1e-4
            );
        }
        let select = label_position(&mut editor, &context, text(T::Select))?;
        click(&mut editor, &context, select)?;
        assert!(editor.image_edit.is_none());
        assert!(editor.selection_active());
        editor.finish_sketch();
        settle(&mut editor, &context)?;
        editor.current_mut().camera_transition = None;
        let revision = editor.current().session.revision();
        let from = at(&editor, [17., 8., 0.])?;
        let to = at(&editor, [20., 10., 0.])?;
        drag_between(&mut editor, &context, from, to)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor.current().session.revision(),
            revision + 1,
            "{}",
            editor.status
        );
        assert!(editor.current().sketch_edit.is_none());
        assert!(
            nator_sketch::distance(
                editor.current().session.document().images()[0].origin,
                [8., 4.]
            ) < 1e-4
        );
        editor.activate_selection(&context);
        let from = at(&editor, [33., 17., 0.])?;
        let to = at(&editor, [38., 20., 0.])?;
        drag_between(&mut editor, &context, from, to)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor.current().session.revision(),
            revision + 2,
            "{}",
            editor.status
        );
        assert!(
            nator_sketch::distance(
                editor.current().session.document().images()[0].size,
                [30., 16.]
            ) < 1e-4
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn paint_panel_picks_faces_previews_hex_and_masks_and_cancels_atomically()
    -> Result<(), Box<dyn Error>> {
        use nator_commands::painting::Target;
        let context = egui::Context::default();
        let mut solid = nator_kernel::Solid::cuboid([0.; 3], [20., 20., 5.])?;
        let (mesh, faces) = solid.tessellate_with_faces(0.05, 0.2)?;
        let mut document = Document::new("Balde de tinta")?;
        let (part, body) = document.add_mesh(
            None,
            "Placa".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
        document.body_mut(body)?.face_names = Arc::new(nator_kernel::indexed_face_names(
            &nator_document::features::body_owner(body),
            solid.face_count()?,
        ));
        let mut mask = Sketch::new(
            "Delimitação",
            Plane {
                origin: [0., 0., 5.],
                ..Plane::xy()
            },
        );
        mask.add_rectangle([2., 2.], [10., 10.], false)?;
        document.add_sketch(Some(part), mask)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        editor.shell.tab = shell::RibbonTab::Model;
        editor.shell.browser_visible = false;
        settle(&mut editor, &context)?;
        let paint = label_position(&mut editor, &context, text(T::Paint))?;
        click(&mut editor, &context, paint)?;
        let revision = editor.current().session.revision();
        let at = |e: &Editor, p| {
            sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                .ok_or("Superfície fora da tela")
        };
        let inside = at(&editor, [7., 7., 5.])?;
        click(&mut editor, &context, inside)?;
        let wait_selection = |e: &mut Editor, count: usize| -> Result<(), Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                frame(e, &context, vec![])?;
                if e.paint_edit
                    .as_ref()
                    .is_some_and(|d| d.targets.len() == count && d.selection_ready())
                {
                    return Ok(());
                }
                if Instant::now() > deadline {
                    return Err(format!(
                        "Seleção de pintura não terminou ({}): {}; alvos: {:?}",
                        count,
                        e.status,
                        e.paint_edit.as_ref().map(|d| &d.targets)
                    )
                    .into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wait_selection(&mut editor, 1).map_err(|e| format!("primeiro clique: {e}"))?;
        assert!(
            matches!(&editor.paint_edit.as_ref().ok_or("Painel ausente")?.targets[0],Target::Face{boundaries,..} if boundaries.len()==1 && boundaries[0].inside)
        );
        let custom = label_position(&mut editor, &context, "#3EAF66")?;
        click(&mut editor, &context, custom)?;
        frame(
            &mut editor,
            &context,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                },
                egui::Event::Text("#1248AC".into()),
            ],
        )?;
        assert_eq!(editor.paint_palette.custom, [18, 72, 172]);
        let wait_preview = |e: &mut Editor| -> Result<(), Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                frame(e, &context, vec![])?;
                let command = e
                    .paint_edit
                    .as_ref()
                    .and_then(|d| d.command())
                    .ok_or("Comando ausente")?;
                match e.preview_status(&command) {
                    live_preview::PreviewStatus::Ready => return Ok(()),
                    live_preview::PreviewStatus::Failed(error) => {
                        return Err(error.to_string().into());
                    }
                    _ => {}
                }
                if Instant::now() > deadline {
                    return Err("Prévia não terminou".into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wait_preview(&mut editor)?;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        let red = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_rgb(220, 65, 65)
                        && (25. ..=28.).contains(&rect.rect.width()) =>
                {
                    Some(rect.rect.center())
                }
                _ => None,
            })
            .ok_or("Cor predefinida ausente")?;
        output.textures_delta.clear();
        click(&mut editor, &context, red)?;
        assert_eq!(editor.paint_palette.custom, [18, 72, 172]);
        assert!(
            matches!(editor.paint_edit.as_ref().and_then(|d|d.command()),Some(Command::PaintSurfaces{color,..}) if color.channels()==[220,65,65])
        );
        assert_eq!(editor.current().session.revision(), revision);
        let cancel = label_position(&mut editor, &context, text(T::Cancel))?;
        click(&mut editor, &context, cancel)?;
        settle(&mut editor, &context)?;
        assert!(editor.current().session.document().appearances().is_empty());
        editor.open_paint();
        frame(&mut editor, &context, vec![])?;
        assert_eq!(
            editor.paint_edit.as_ref().ok_or("Painel ausente")?.hex,
            "#1248AC"
        );
        let outside = at(&editor, [1., 1., 5.])?;
        click(&mut editor, &context, outside)?;
        wait_selection(&mut editor, 1).map_err(|e| format!("região exterior: {e}"))?;
        assert!(
            matches!(&editor.paint_edit.as_ref().ok_or("Painel ausente")?.targets[0],Target::Face{boundaries,..} if boundaries.len()==1 && !boundaries[0].inside)
        );
        let side = at(&editor, [10., 0., 2.5])?;
        click(&mut editor, &context, side)?;
        wait_selection(&mut editor, 2).map_err(|e| format!("segunda superfície: {e}"))?;
        wait_preview(&mut editor)?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor.current().session.revision(),
            revision + 1,
            "{}",
            editor.status
        );
        let appearance = editor
            .current()
            .session
            .document()
            .body_appearance(body)
            .ok_or("Pintura ausente")?;
        assert_eq!(appearance.layers.len(), 2);
        assert_eq!(appearance.layers[0].color.channels(), [18, 72, 172]);
        editor.open_paint();
        frame(&mut editor, &context, vec![])?;
        let inside = at(&editor, [7., 7., 5.])?;
        // Os dois cliques entram sem aguardar a primeira consulta geométrica.
        click(&mut editor, &context, inside)?;
        click(&mut editor, &context, inside)?;
        wait_selection(&mut editor, 1).map_err(|e| format!("duplo clique: {e}"))?;
        assert_eq!(
            editor.paint_edit.as_ref().ok_or("Painel ausente")?.targets,
            vec![Target::Body(body)]
        );
        wait_preview(&mut editor)?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        assert_eq!(editor.current().session.revision(), revision + 2);
        assert!(
            editor
                .current()
                .session
                .document()
                .body_appearance(body)
                .is_some_and(|a| a.layers.is_empty())
        );
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .body(body)
                .ok_or("Corpo ausente")?
                .color
                .map(|c| c.channels()),
            Some([18, 72, 172])
        );
        assert!(editor.current_mut().session.undo());
        assert!(
            editor
                .current()
                .session
                .document()
                .body_appearance(body)
                .is_some()
        );
        assert!(editor.current_mut().session.redo());
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn part_and_paint_hover_only_the_visible_sketch_cell_and_respect_holes_and_occlusion()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut solid = nator_kernel::Solid::cuboid([0.; 3], [20., 20., 5.])?;
        let (mesh, faces) = solid.tessellate_with_faces(0.02, 0.2)?;
        let mut document = Document::new("Realce de células")?;
        let (part, body) = document.add_mesh(
            None,
            "Placa".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        document.body_mut(body)?.brep = Some(Arc::new(solid.to_brep()?));
        // Nomes repetidos também precisam de realce preciso, como na pintura.
        document.body_mut(body)?.face_names =
            Arc::new(vec!["shared:face".into(); solid.face_count()?]);
        let mut sketch = Sketch::new(
            "Divisões",
            Plane {
                origin: [0., 0., 5.],
                ..Plane::xy()
            },
        );
        sketch.add_rectangle([2., 2.], [10., 10.], false)?;
        sketch.add_circle([6., 6.], 2., false)?;
        let (_, id) = document.add_sketch(Some(part), sketch)?;
        let mut free = Sketch::new(
            "Perfil livre",
            Plane {
                origin: [0., 0., 20.],
                ..Plane::xy()
            },
        );
        free.add_rectangle([-20., -20.], [-10., -10.], false)?;
        document.add_sketch(Some(part), free)?;
        let mut cover = nator_kernel::Solid::cuboid([2., 2., 8.], [8., 8., 2.])?;
        let (mesh, faces) = cover.tessellate_with_faces(0.02, 0.2)?;
        let (_, blocker) = document.add_mesh(
            Some(part),
            "Cobertura".into(),
            Arc::new(MeshAsset::with_faces(mesh, faces)?),
        )?;
        document.body_mut(blocker)?.brep = Some(Arc::new(cover.to_brep()?));
        document.body_mut(blocker)?.face_names = Arc::new(nator_kernel::indexed_face_names(
            "cover",
            cover.face_count()?,
        ));
        document.body_mut(blocker)?.visible = false;
        if let Some(directory) = std::env::var_os("NATOR_CELL_HOVER_CAPTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory)?;
            write_project(&directory.join("cell-hover.3dnator"), &document)?;
        }
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        editor.shell.browser_visible = false;
        editor.viewport.camera.set_direction([0., 0., 1.]);
        settle(&mut editor, &context)?;
        let revision = editor.current().session.revision();
        let area = |overlay: &highlight::Overlay| {
            overlay
                .triangles
                .iter()
                .map(|[a, b, c]| {
                    nator_geometry::norm(nator_geometry::cross(
                        nator_geometry::sub(*b, *a),
                        nator_geometry::sub(*c, *a),
                    )) / 2.
                })
                .sum::<f64>()
        };
        let hover_at = |editor: &mut Editor, point, expected: f64| -> Result<(), Box<dyn Error>> {
            let position = sketch_scene::project(
                &editor.viewport.camera,
                editor.display_bounds(),
                editor.canvas_rect,
                point,
            )
            .ok_or("Área fora da tela")?;
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                frame(editor, &context, vec![egui::Event::PointerMoved(position)])?;
                if editor.hover.as_ref().is_some_and(|h| {
                    h.position.distance(position) < 0.1 && (area(&h.overlay) - expected).abs() < 0.1
                }) {
                    return Ok(());
                }
                if Instant::now() > deadline {
                    return Err(format!(
                        "Área de realce incorreta: esperado {expected}, recebido {:?}",
                        editor.hover.as_ref().map(|h| area(&h.overlay))
                    )
                    .into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        for paint in [false, true] {
            if paint {
                editor.open_paint();
                frame(&mut editor, &context, vec![])?;
            }
            hover_at(&mut editor, [3., 3., 5.], 64. - 4. * std::f64::consts::PI)?;
            hover_at(&mut editor, [6., 6., 5.], 4. * std::f64::consts::PI)?;
            hover_at(&mut editor, [1., 1., 5.], 336.)?;
            assert_eq!(editor.current().session.revision(), revision);
        }
        editor.execute(Command::SetSketchVisibility { id, visible: false });
        settle(&mut editor, &context)?;
        hover_at(&mut editor, [3., 3., 5.], 400.)?;
        editor.execute(Command::SetSketchVisibility { id, visible: true });
        editor.execute(Command::SetBodyVisibility {
            id: blocker,
            visible: true,
        });
        settle(&mut editor, &context)?;
        hover_at(&mut editor, [6., 6., 10.], 64.)?;
        // Sem corpo atrás, Peça oferece o perfil livre. Pintar exige uma superfície.
        editor.activate_selection(&context);
        editor.viewport.camera.span = 4.;
        frame(&mut editor, &context, vec![])?;
        let position = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [-15., -15., 20.],
        )
        .ok_or("Perfil fora da tela")?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            frame(
                &mut editor,
                &context,
                vec![egui::Event::PointerMoved(position)],
            )?;
            if editor
                .hover
                .as_ref()
                .is_some_and(|h| !h.overlay.contours.is_empty())
            {
                break;
            }
            if Instant::now() > deadline {
                return Err(format!(
                    "Perfil livre não realçado: pos={position:?}; canvas={:?}; hover={:?}",
                    editor.canvas_rect,
                    editor
                        .hover
                        .as_ref()
                        .map(|h| (&h.label, h.overlay.contours.len()))
                )
                .into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        frame(
            &mut editor,
            &context,
            vec![egui::Event::PointerMoved(egui::pos2(1., 1.))],
        )?;
        assert!(editor.hover.is_none());
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn imaginary_centers_appear_while_drawing_without_creating_points() -> Result<(), Box<dyn Error>>
    {
        let context = egui::Context::default();
        let mut document = Document::new("Centros imaginários")?;
        let mut sketch = Sketch::new("Centros", Plane::xy());
        sketch.add_circle([0., 0.], 10., false)?;
        sketch.add_rectangle([20., 0.], [40., 20.], false)?;
        let before = (sketch.points.len(), sketch.entities.len());
        let (part, id) = document.add_sketch(None, sketch)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let diamonds = |editor: &mut Editor| -> Vec<egui::Pos2> {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280., 820.),
                    )),
                    ..Default::default()
                },
                |ui| editor.draw(ui),
            );
            let points = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Path(path)
                        if path.closed && path.points.len() == 4 && path.stroke.width == 1.3 =>
                    {
                        Some(egui::pos2(
                            path.points.iter().map(|p| p.x).sum::<f32>() / 4.,
                            path.points.iter().map(|p| p.y).sum::<f32>() / 4.,
                        ))
                    }
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
            points
        };
        editor.activate_selection(&context);
        let found = diamonds(&mut editor);
        let center = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [0., 0., 0.],
        )
        .ok_or("Centro fora da tela")?;
        assert!(!found.iter().any(|p| p.distance(center) < 0.1));
        editor.select_sketch_tool(sketch_editor::Tool::Line);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let found = diamonds(&mut editor);
            let at = |p| {
                sketch_scene::project(
                    &editor.viewport.camera,
                    editor.display_bounds(),
                    editor.canvas_rect,
                    p,
                )
            };
            if [[0., 0., 0.], [30., 10., 0.]]
                .iter()
                .all(|p| at(*p).is_some_and(|p| found.iter().any(|q| p.distance(*q) < 0.1)))
            {
                break;
            }
            if Instant::now() > deadline {
                return Err(format!("Centros ausentes: {found:?}").into());
            }
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        editor.activate_selection(&context);
        let found = diamonds(&mut editor);
        let center = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            editor.canvas_rect,
            [0., 0., 0.],
        )
        .ok_or("Centro fora da tela")?;
        assert!(!found.iter().any(|p| p.distance(center) < 0.1));
        let sketch = &editor
            .current()
            .session
            .document()
            .sketch(id)
            .ok_or("Rascunho ausente")?
            .sketch;
        assert_eq!((sketch.points.len(), sketch.entities.len()), before);
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn extrusion_clicks_toggle_multiple_projected_cells_in_one_profile()
    -> Result<(), Box<dyn Error>> {
        use nator_document::features::RegionRef;
        let context = egui::Context::default();
        let mut document = Document::new("Áreas projetadas")?;
        let mut sketch = Sketch::new("Projetado", Plane::xy());
        for x in [0., 25.] {
            let points = [[x, 0.], [x + 10., 0.], [x + 10., 10.], [x, 10.]];
            let curves = (0..4)
                .map(|i| nator_sketch::profiles::Segment::Line {
                    a: points[i],
                    b: points[(i + 1) % 4],
                })
                .collect::<Vec<_>>();
            nator_sketch::projection::insert(&mut sketch, &curves, &[])?;
        }
        let (part, _) = document.add_sketch(None, sketch)?;
        let mut upper = Sketch::new(
            "Plano limite",
            Plane {
                origin: [0., 0., 20.],
                ..Plane::xy()
            },
        );
        upper.add_rectangle([-30., -20.], [-20., -10.], false)?;
        let (_, limit) = document.add_sketch(Some(part), upper)?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.current_mut().active_part = Some(part);
        settle(&mut editor, &context)?;
        editor.open_feature_tool(feature_editor::Tool::Extrude);

        for _ in 0..15 {
            frame(&mut editor, &context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let at = |e: &Editor, p| {
            sketch_scene::project(&e.viewport.camera, e.display_bounds(), e.canvas_rect, p)
                .ok_or("Ponto sem projeção")
        };
        for p in [[5., 5., 0.], [30., 5., 0.]] {
            let point = at(&editor, p)?;
            click(&mut editor, &context, point)?;
            wait_extrusion_guide(&mut editor, &context)?;
        }
        let reference = editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.region_reference())
            .ok_or("Perfil ausente")?;
        assert!(
            matches!(reference,RegionRef::Multiple(regions) if regions.len()==2),
            "{reference:?}"
        );
        let point = at(&editor, [30., 5., 0.])?;
        click(&mut editor, &context, point)?;
        wait_extrusion_guide(&mut editor, &context)?;
        assert_eq!(
            editor
                .feature_edit
                .as_ref()
                .and_then(|d| d.region_reference())
                .ok_or("Perfil ausente")?
                .selections()
                .len(),
            1
        );
        let point = at(&editor, [30., 5., 0.])?;
        click(&mut editor, &context, point)?;
        wait_extrusion_guide(&mut editor, &context)?;
        assert_eq!(
            editor
                .feature_edit
                .as_ref()
                .and_then(|d| d.extrusion_state())
                .ok_or("Guia ausente")?
                .0
                .to,
            0.
        );
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .part(part)
                .ok_or("Peça ausente")?
                .features
                .len(),
            0
        );
        let point = at(&editor, [-25., -15., 20.])?;
        click(&mut editor, &context, point)?;
        wait_feature_preview(&mut editor, &context)?;
        assert!(
            (editor
                .feature_edit
                .as_ref()
                .and_then(|d| d.extrusion_state())
                .ok_or("Guia ausente")?
                .0
                .to
                - 20.)
                .abs()
                < 1e-5
        );
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        let part = editor
            .current()
            .session
            .document()
            .part(part)
            .ok_or("Peça ausente")?;
        assert_eq!(part.features.len(), 1);
        assert_eq!(part.bodies.len(), 1);
        assert!(
            matches!(&part.features[0].operation,nator_document::features::Operation::Extrude{region:RegionRef::Multiple(regions),until:Some(nator_document::features::ExtrusionLimit::Sketch{sketch}),..} if regions.len()==2 && *sketch==limit)
        );
        Ok(())
    }

    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_tree_menus_rename_delete_and_create_with_monotonic_names()
    -> Result<(), Box<dyn Error>> {
        let context = egui::Context::default();
        let mut document = Document::new("Árvore")?;
        let (part, id) = document.add_sketch(None, Sketch::new("Rascunho", Plane::xy()))?;
        let mut editor =
            Editor::from_gpu(gpu_state()?, &context, document).map_err(|e| e.to_string())?;
        editor.recoveries.clear();
        editor.show_recovery = false;
        editor.begin_sketch(part, id);
        editor.align_sketch_camera(true, false);
        settle(&mut editor, &context)?;
        let right = |editor: &mut Editor, position| -> Result<(), Box<dyn Error>> {
            for pressed in [true, false] {
                frame(
                    editor,
                    &context,
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Secondary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                )?;
            }
            Ok(())
        };
        let node = label_position(&mut editor, &context, "Rascunho 1")?;
        right(&mut editor, node)?;
        let rename = label_position(&mut editor, &context, text(T::RenameFeature))?;
        click(&mut editor, &context, rename)?;
        assert!(editor.rename.is_some(), "Renomear não abriu");
        frame(&mut editor, &context, vec![])?;
        frame(
            &mut editor,
            &context,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                },
                egui::Event::Text("Principal".into()),
            ],
        )?;
        let ok = label_position(&mut editor, &context, text(T::Ok))?;
        click(&mut editor, &context, ok)?;
        settle(&mut editor, &context)?;
        assert_eq!(
            editor
                .current()
                .session
                .document()
                .sketch(id)
                .ok_or("Rascunho ausente")?
                .sketch
                .name,
            "Principal"
        );
        let node = label_position(&mut editor, &context, "Principal")?;
        right(&mut editor, node)?;
        let delete = label_position(&mut editor, &context, text(T::DeleteFeature))?;
        click(&mut editor, &context, delete)?;
        assert!(editor.current().session.document().sketch(id).is_none());
        assert!(editor.current().sketch_edit.is_none());
        let folder = label_position(
            &mut editor,
            &context,
            &format!("{} (0)", text(T::SketchesGroup)),
        )?;
        // Primeiro nó é o de referência; escolher o da peça pelo rótulo contextual
        // usa o mesmo fluxo sem alterar geometria antes da escolha de suporte.
        right(&mut editor, folder)?;
        let new = label_position(&mut editor, &context, text(T::NewSketch))?;
        click(&mut editor, &context, new)?;
        assert!(editor.current().plane_pick.is_some());
        editor.choose_origin_plane("XY", Plane::xy());
        settle(&mut editor, &context)?;
        let doc = editor.current().session.document();
        let latest = doc
            .reference_sketches()
            .last()
            .or_else(|| doc.part(part).and_then(|p| p.sketches.last()))
            .ok_or("Rascunho novo ausente")?;
        assert!(latest.sketch.name.ends_with(" 2"), "{}", latest.sketch.name);
        Ok(())
    }

    fn drag_between(
        editor: &mut Editor,
        context: &egui::Context,
        from: egui::Pos2,
        to: egui::Pos2,
    ) -> Result<(), Box<dyn Error>> {
        frame(
            editor,
            context,
            vec![
                egui::Event::PointerMoved(from),
                egui::Event::PointerButton {
                    pos: from,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )?;
        frame(editor, context, vec![egui::Event::PointerMoved(to)])?;
        frame(
            editor,
            context,
            vec![
                egui::Event::PointerMoved(to),
                egui::Event::PointerButton {
                    pos: to,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )?;
        frame(editor, context, vec![])?;
        Ok(())
    }
    fn drag_operation_arrow(
        editor: &mut Editor,
        context: &egui::Context,
        side: usize,
        delta: egui::Vec2,
    ) -> Result<(), Box<dyn Error>> {
        frame(editor, context, vec![])?;
        let tip = *editor
            .current()
            .sketch_edit
            .as_ref()
            .ok_or("Editor ausente")?
            .operation_handle_tips()
            .get(side)
            .ok_or("Seta ausente")?;
        drag_between(editor, context, tip, tip + delta)
    }
    fn wait_sketch_drag(
        editor: &mut Editor,
        context: &egui::Context,
    ) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            frame(editor, context, vec![])?;
            if !editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.drag_pending())
            {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err("Arraste não terminou".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_sketch_operation(
        editor: &mut Editor,
        context: &egui::Context,
    ) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            frame(editor, context, vec![])?;
            if editor
                .current()
                .sketch_edit
                .as_ref()
                .is_some_and(|w| w.operation_ready())
            {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!(
                    "Prévia do rascunho não terminou: {}; {}",
                    editor.status,
                    editor
                        .current()
                        .sketch_edit
                        .as_ref()
                        .map(|w| w.operation_debug())
                        .unwrap_or_default()
                )
                .into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_feature_preview(
        editor: &mut Editor,
        context: &egui::Context,
    ) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            frame(editor, context, Vec::new())?;
            let command = editor
                .feature_edit
                .as_ref()
                .and_then(|d| {
                    editor
                        .current()
                        .session
                        .document()
                        .part(d.part())
                        .map(|p| d.preview_key(p))
                })
                .ok_or("Painel ausente")?;
            match editor.preview_status(&command) {
                live_preview::PreviewStatus::Ready => return Ok(()),
                live_preview::PreviewStatus::Failed(error) => return Err(error.to_string().into()),
                _ => {}
            }
            if Instant::now() > deadline {
                return Err(format!(
                    "Prévia não concluiu: {}; comando: {command:?}",
                    editor.status
                )
                .into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_extrusion_guide(
        editor: &mut Editor,
        context: &egui::Context,
    ) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .is_none()
        {
            if Instant::now() > deadline {
                return Err(format!("Guia ausente: {}", editor.status).into());
            }
            frame(editor, context, vec![])?;
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn label_position(
        editor: &mut Editor,
        context: &egui::Context,
        label: &str,
    ) -> Result<egui::Pos2, Box<dyn Error>> {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280., 820.),
                )),
                time: Some(context.input(|i| i.time) + 1. / 60.),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        let position_for = |clipped: &egui::epaint::ClippedShape| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.job.text == label => {
                Some(t.visual_bounding_rect().center())
            }
            _ => None,
        };
        let position = if label == text(T::Cancel) {
            output.shapes.iter().rev().find_map(position_for)
        } else {
            output.shapes.iter().find_map(position_for)
        };
        output.textures_delta.clear();
        position.ok_or_else(|| format!("Rótulo ausente: {label}").into())
    }

    fn frame(
        editor: &mut Editor,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Result<egui::Rect, Box<dyn Error>> {
        // Estes ensaios exercitam ferramentas do CAD. O diálogo de erros
        // tem um ensaio próprio; nos demais, sua leitura é considerada
        // concluída para que ele não intercepte o próximo gesto da ferramenta.
        while editor.status.has_pending() {
            editor.status.dismiss();
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280., 820.),
            )),
            events,
            time: Some(context.input(|i| i.time) + 1. / 60.),
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| editor.draw(ui));
        while editor.status.has_pending() {
            editor.status.dismiss();
        }
        output.textures_delta.clear();
        let primitives = context.tessellate(output.shapes, output.pixels_per_point);
        let images: Vec<_> = primitives
            .iter()
            .filter_map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == editor.texture => {
                    Some(mesh.calc_bounds())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            images.len(),
            1,
            "Deve haver exatamente um canvas CAD, inclusive em edição."
        );
        images
            .first()
            .copied()
            .ok_or_else(|| "Viewport ausente".into())
    }
    fn click(
        editor: &mut Editor,
        context: &egui::Context,
        position: egui::Pos2,
    ) -> Result<(), Box<dyn Error>> {
        for pressed in [true, false] {
            frame(
                editor,
                context,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )?;
        }
        Ok(())
    }
    fn click_modified(
        editor: &mut Editor,
        context: &egui::Context,
        position: egui::Pos2,
        modifiers: egui::Modifiers,
    ) -> Result<(), Box<dyn Error>> {
        for pressed in [true, false] {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280., 820.),
                    )),
                    events: vec![
                        egui::Event::ModifiersChanged(modifiers),
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers,
                        },
                    ],
                    time: Some(context.input(|input| input.time) + 1. / 60.),
                    ..Default::default()
                },
                |ui| editor.draw(ui),
            );
            output.textures_delta.clear();
        }
        frame(
            editor,
            context,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)],
        )?;
        Ok(())
    }

    fn settle(editor: &mut Editor, context: &egui::Context) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while editor.busy() || editor.rendered != Some(editor.scene_key()) {
            if Instant::now() > deadline {
                return Err(format!(
                    "Trabalhador não terminou: {}; exibida: {:?}; pedida: {:?}; atual: {:?}",
                    editor.status,
                    editor.rendered,
                    editor.requested,
                    editor.scene_key()
                )
                .into());
            }
            std::thread::sleep(Duration::from_millis(5));
            frame(editor, context, Vec::new())?;
        }
        Ok(())
    }
}
