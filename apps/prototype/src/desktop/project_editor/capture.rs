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
    if sketch_mode {
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
    }
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
    for frame in 0..3 {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
        let input = egui::RawInput {
            screen_rect: Some(rect),
            time: Some(f64::from(frame) / 60.),
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
        // aproximá-la pela normal de um triângulo e sem trocar de canvas.
        editor.start_sketch_plane();
        let rect = frame(&mut editor, &context, Vec::new())?;
        let point = sketch_scene::project(
            &editor.viewport.camera,
            editor.display_bounds(),
            rect,
            [16., 12., 8.],
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
    fn frame(
        editor: &mut Editor,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Result<egui::Rect, Box<dyn Error>> {
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
                return Err(format!("Trabalhador não terminou: {}", editor.status).into());
            }
            std::thread::sleep(Duration::from_millis(5));
            frame(editor, context, Vec::new())?;
        }
        Ok(())
    }
}
