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
        | "hover-face"
        | "hover-edge"
        | "select-body"
        | "window"
        | "assembly-select"
        | "move"
        | "align"
        | "assembly-ribbon"
        | "section"
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
        | "sketch-circle-input"
        | "sketch-rectangle-input"
        | "extrude-face"
        | "status-dialog"
        | "extrude-cut" => {}
        _ => {
            return Err(
                "Cenários: timeline, timeline-menu, extrude, edit-last, failure, hover-face, \
                 hover-edge, select-body, window, assembly-select, move, align, assembly-ribbon, section, repair, recovery, insert, mesh-import, menu-new-part, menu-move, menu-copy, reference-align, reference-sketch, copy-beside, tree-folders, part-menu-move, measure, sketch-line-input, sketch-circle-input, sketch-rectangle-input, extrude-face, extrude-cut, status-dialog."
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
            let target = canvas.center() + egui::vec2(-60., 40.);
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
                        } else {
                            "40"
                        }
                        .into(),
                    )],
                    6 if pointer != "sketch-circle-input" => vec![egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    7 if pointer != "sketch-circle-input" => vec![egui::Event::Text("30".into())],
                    _ => Vec::new(),
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
                "hover-face" | "hover-edge" if frame >= 2 => {
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
        "extrude-face" | "extrude-cut" => {
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
                d.set_size_for_capture(if scenario == "extrude-cut" {
                    "-3 mm"
                } else {
                    "6 mm"
                });
            }
        }
        "sketch-line-input" | "sketch-circle-input" | "sketch-rectangle-input" => {
            open_first_sketch(editor)?;
            let work = editor
                .current_mut()
                .sketch_edit
                .as_mut()
                .ok_or("Rascunho ausente")?;
            work.select_tool(match scenario {
                "sketch-line-input" => sketch_editor::Tool::Line,
                "sketch-circle-input" => sketch_editor::Tool::Circle,
                _ => sketch_editor::Tool::Rectangle,
            });
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
        "hover-face" | "hover-edge" | "window" => {
            editor.current_mut().select(None);
            editor.pick_mode = if scenario == "hover-edge" {
                navigation::PickMode::Edge
            } else {
                navigation::PickMode::Face
            };
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

    /// Ensaio optativo porque precisa de adaptador wgpu. Usa eventos reais egui
    /// sobre o canvas completo; não exige compositor nem janela nativa.
    #[test]
    #[ignore = "Requer adaptador gráfico wgpu (físico ou software)."]
    fn sketch_workflow_keeps_the_model_and_the_same_viewport() -> Result<(), Box<dyn Error>> {
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
        let dimension = label_position(&mut editor, &context, text(T::SketchDimension))?;
        click(&mut editor, &context, dimension)?;
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
                egui::Event::Text("32 mm".into()),
            ],
        )?;
        frame(&mut editor, &context, vec![enter()])?;
        assert!(
            editor
                .current()
                .sketch_edit
                .as_ref()
                .and_then(|w| w.active_action())
                .is_some()
        );
        frame(&mut editor, &context, Vec::new())?;
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
        let field = label_position(&mut editor, &context, "32 mm")?;
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

        let circular = label_position(&mut editor, &context, text(T::SketchCircularShort))?;
        click(&mut editor, &context, circular)?;
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

        // Em 900 px, Concluir precisa continuar à direita, mesmo com rolagem dos grupos.
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900., 600.),
                )),
                ..Default::default()
            },
            |ui| editor.draw(ui),
        );
        let finish = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.job.text == text(T::FinishSketch) => {
                    Some(t.visual_bounding_rect())
                }
                _ => None,
            })
            .ok_or("Concluir escondido na janela compacta")?;
        assert!(finish.left() > 800. && finish.right() <= 900., "{finish:?}");
        output.textures_delta.clear();
        Ok(())
    }

    /// Digitação sem clicar nos campos, alternância por Tab, Enter único e cancelamento.
    /// Verifica o resultado publicado e as cotas, em vez de apenas inspecionar o formulário.
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
        wait_feature_preview(&mut editor, &context)?;
        let (guide, mode) = editor
            .feature_edit
            .as_ref()
            .and_then(|d| d.extrusion_state())
            .ok_or("Guia de face ausente")?;
        assert_eq!(mode, 1);
        let origin = project(&editor, guide.origin)?;
        let endpoint = project(
            &editor,
            std::array::from_fn(|i| guide.origin[i] + guide.normal[i] * guide.to),
        )?;
        let unit = project(
            &editor,
            std::array::from_fn(|i| guide.origin[i] + guide.normal[i]),
        )? - origin;
        let target = endpoint - unit * (guide.to as f32 + 3.);
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
            matches!(piece.features.last().map(|f|&f.operation),Some(Operation::ExtrudeFace {base:source,until:Some(until),combine:Some(combine),..}) if *source==base&&until.body==limit&&combine.mode==nator_document::features::BooleanMode::Join)
        );
        assert_eq!(piece.sketches.len(), 3);
        assert!(editor.current_mut().session.undo());
        assert!(editor.current_mut().session.redo());
        Ok(())
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
        let position = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => {
                    Some(t.visual_bounding_rect().center())
                }
                _ => None,
            });
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
