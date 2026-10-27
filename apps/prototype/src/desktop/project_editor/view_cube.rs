//! Navegador de orientação: um poliedro fechado com seis faces, doze chanfros e
//! oito cantos. Seleção e desenho usam os mesmos polígonos projetados, sem caixas
//! sobrepostas por face. Toda a pintura fica recortada à área deste controle.
use super::{
    Camera, Point3, T, egui,
    icons::{self, Icon},
    text, theme,
};
use nator_geometry::{dot, norm};

const SIZE: egui::Vec2 = egui::vec2(208., 200.);
const BEVEL: f64 = 0.60;
const SCALE: f32 = 55.;

struct Face {
    normal: Point3,
    vertices: Vec<Point3>,
    label: Option<T>,
}
struct ProjectedFace {
    normal: Point3,
    polygon: Vec<egui::Pos2>,
    label: Option<T>,
    depth: f64,
}

pub(super) fn rect(canvas: egui::Rect) -> egui::Rect {
    if canvas.width() < SIZE.x + 16. || canvas.height() < SIZE.y + 16. {
        return egui::Rect::NOTHING;
    }
    egui::Rect::from_min_size(canvas.right_top() + egui::vec2(-SIZE.x - 8., 8.), SIZE)
}

/// Uma única região de arraste impede que a seleção mude de face durante um
/// movimento. Home e órbita têm áreas próprias; os demais controles ficam no menu.
pub(super) fn show(ui: &mut egui::Ui, camera: &mut Camera, canvas: egui::Rect) {
    let area = rect(canvas);
    if !area.is_positive() {
        return;
    }
    // Uma camada própria põe o navegador acima das anotações da cena, enquanto
    // os diálogos e modais continuam podendo receber seus próprios eventos.
    egui::Area::new(ui.make_persistent_id("view_cube_overlay"))
        .order(egui::Order::Middle)
        .fade_in(false)
        .movable(false)
        .constrain(false)
        .fixed_pos(area.min)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(SIZE);
            ui.set_max_size(SIZE);
            show_inner(ui, camera, area, canvas.aspect_ratio());
        });
}

fn show_inner(ui: &mut egui::Ui, camera: &mut Camera, area: egui::Rect, aspect: f32) {
    let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
    let control =
        |x, y| egui::Rect::from_min_size(area.min + egui::vec2(x, y), egui::Vec2::splat(30.));
    if icon_control(
        ui,
        &painter,
        "view_home",
        control(2., 15.),
        Icon::Home,
        T::ViewHome,
    )
    .clicked()
    {
        // Home restaura orientação, roll e enquadramento, conservando a escolha
        // entre projeção ortográfica/perspectiva. A geometria não é modificada.
        camera.set_direction(Camera::default().direction());
        camera.fit(f64::from(aspect));
        ui.ctx().request_repaint();
    }
    // A referência visual usa somente duas setas largas. O gesto modificado
    // oferece o sentido inverso sem acrescentar botões permanentes ao navegador.
    let step = std::f64::consts::FRAC_PI_4 / 0.008;
    for (index, offset, size, curve, label, delta) in [
        (
            0,
            [61., 0.],
            [93., 44.],
            [[0.90, 0.74], [0.65, 0.46], [0.25, 0.46]],
            T::ViewOrbitLeft,
            [-step, 0.],
        ),
        (
            1,
            [160., 55.],
            [45., 91.],
            [[0.25, 0.10], [0.53, 0.40], [0.53, 0.75]],
            T::ViewOrbitDown,
            [0., step],
        ),
    ] {
        let rect = egui::Rect::from_min_size(
            area.min + egui::vec2(offset[0], offset[1]),
            egui::vec2(size[0], size[1]),
        );
        let response = curved_control(ui, &painter, index, rect, curve, label);
        if response.clicked() || response.clicked_by(egui::PointerButton::Secondary) {
            let reverse = response.clicked_by(egui::PointerButton::Secondary)
                || ui.input(|i| i.modifiers.shift);
            camera.orbit(delta.map(|v| if reverse { -v } else { v }));
            ui.ctx().request_repaint();
        }
    }
    let body = body_rect(area);
    let response = ui.interact(
        body,
        ui.make_persistent_id("view_cube_body"),
        egui::Sense::click_and_drag(),
    );
    let center = body.center();
    let before = projected_faces(camera, center);
    let hit = response
        .hover_pos()
        .and_then(|pointer| hit_face(&before, pointer));
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = ui.input(|i| i.pointer.delta());
        camera.orbit([f64::from(delta.x), f64::from(delta.y)]);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.clicked()
        && let Some(face) = hit
    {
        camera.set_direction(before[face].normal);
    } else if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text(T::ViewCubeHint))
    });
    let hint = hit
        .and_then(|i| before[i].label)
        .map(|label| format!("{}\n{}", text(label), text(T::ViewCubeHint)))
        .unwrap_or_else(|| text(T::ViewCubeHint).into());
    response.clone().on_hover_text(hint);
    response.context_menu(|ui| {
        ui.label(text(T::Projection));
        for (projection, label) in [
            (nator_renderer::Projection::Orthographic, T::Orthographic),
            (nator_renderer::Projection::Perspective, T::Perspective),
        ] {
            if ui
                .selectable_value(&mut camera.projection, projection, text(label))
                .clicked()
            {
                ui.close();
            }
        }
        ui.separator();
        if ui.button(text(T::Fit)).clicked() {
            camera.fit(f64::from(aspect));
            ui.close();
        }
        for (icon, label, hint, clockwise) in [
            (
                Icon::RotateCounterclockwise,
                T::ViewRollLeft,
                T::ViewRotateLeft,
                false,
            ),
            (
                Icon::RotateClockwise,
                T::ViewRollRight,
                T::ViewRotateRight,
                true,
            ),
        ] {
            if icons::entry(ui, icon, text(label), false)
                .on_hover_text(text(hint))
                .clicked()
            {
                camera.rotate_quarter_turn(clockwise);
                ui.close();
            }
        }
    });
    let faces = projected_faces(camera, center);
    let hovered = if response.dragged() {
        None
    } else {
        response
            .hover_pos()
            .and_then(|pointer| hit_face(&faces, pointer))
    };
    paint_faces(&painter, &faces, hovered);
}

fn body_rect(area: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(area.min + egui::vec2(84., 122.), egui::Vec2::splat(150.))
}

fn icon_control(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    id: impl std::hash::Hash + std::fmt::Debug,
    rect: egui::Rect,
    icon: Icon,
    label: T,
) -> egui::Response {
    let response = ui.interact(rect, ui.make_persistent_id(id), egui::Sense::click());
    if response.hovered() {
        painter.rect_filled(rect, 4, theme::BLUE_FILL);
    }
    icons::paint_with_ink(
        painter,
        rect.shrink(3.),
        icon,
        true,
        if response.hovered() {
            theme::ACCENT
        } else {
            egui::Color32::from_rgb(167, 179, 187)
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text(label)));
    response.on_hover_text(text(label))
}

fn curved_control(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    index: usize,
    rect: egui::Rect,
    curve: [[f32; 2]; 3],
    label: T,
) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.make_persistent_id(("view_orbit", index)),
        egui::Sense::click(),
    );
    let color = if response.hovered() {
        theme::ACCENT
    } else {
        egui::Color32::from_rgb(167, 179, 187)
    };
    // A faixa curva usa pares de vértices; a ponta usa um triângulo separado.
    // Um leque convexo seria incorreto no pescoço côncavo da seta e preencheria o vazio.
    let mut mesh = egui::Mesh::default();
    let mut left = Vec::new();
    let mut right = Vec::new();
    for i in 0..=24 {
        let t = i as f32 / 24.;
        let at = |k: usize| {
            (1. - t).powi(2) * curve[0][k] + 2. * t * (1. - t) * curve[1][k] + t * t * curve[2][k]
        };
        let direction = egui::vec2(
            ((1. - t) * (curve[1][0] - curve[0][0]) + t * (curve[2][0] - curve[1][0]))
                * rect.width(),
            ((1. - t) * (curve[1][1] - curve[0][1]) + t * (curve[2][1] - curve[1][1]))
                * rect.height(),
        )
        .normalized();
        let center = rect.min + egui::vec2(at(0) * rect.width(), at(1) * rect.height());
        let perpendicular = egui::vec2(-direction.y, direction.x);
        let a = center + perpendicular * 10.;
        let b = center - perpendicular * 10.;
        mesh.colored_vertex(a, color);
        mesh.colored_vertex(b, color);
        left.push(a);
        right.push(b);
        if i > 0 {
            let n = (i * 2) as u32;
            mesh.add_triangle(n - 2, n - 1, n);
            mesh.add_triangle(n - 1, n + 1, n);
        }
    }
    let end = rect.min + egui::vec2(curve[2][0] * rect.width(), curve[2][1] * rect.height());
    let direction = egui::vec2(
        (curve[2][0] - curve[1][0]) * rect.width(),
        (curve[2][1] - curve[1][1]) * rect.height(),
    )
    .normalized();
    let perpendicular = egui::vec2(-direction.y, direction.x);
    let a = end + perpendicular * 18.5;
    let b = end - perpendicular * 18.5;
    let tip = end + direction * 22.;
    let n = mesh.vertices.len() as u32;
    for p in [a, tip, b] {
        mesh.colored_vertex(p, color);
    }
    mesh.add_triangle(n, n + 1, n + 2);
    painter.add(egui::Shape::mesh(mesh));
    let outline = left
        .into_iter()
        .chain([a, tip, b])
        .chain(right.into_iter().rev())
        .collect();
    painter.add(egui::Shape::closed_line(
        outline,
        egui::Stroke::new(0.6, egui::Color32::from_rgb(75, 84, 91)),
    ));

    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text(label)));
    response.on_hover_text(format!(
        "{}\n{}",
        text(label),
        text(T::ViewOrbitReverseHint)
    ))
}

fn cube_faces() -> Vec<Face> {
    let mut faces = Vec::with_capacity(26);
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let vertices = [
                [-BEVEL, -BEVEL],
                [BEVEL, -BEVEL],
                [BEVEL, BEVEL],
                [-BEVEL, BEVEL],
            ]
            .map(|v| {
                let mut point = [0.; 3];
                point[axis] = sign;
                point[a] = v[0];
                point[b] = v[1];
                point
            })
            .to_vec();
            let mut normal = [0.; 3];
            normal[axis] = sign;
            let label = match (axis, sign > 0.) {
                (0, true) => T::ViewRight,
                (0, false) => T::ViewLeft,
                (1, true) => T::ViewBack,
                (1, false) => T::Front,
                (2, true) => T::Top,
                _ => T::ViewBottom,
            };
            faces.push(Face {
                normal,
                vertices,
                label: Some(label),
            });
        }
    }
    for axis in 0..3 {
        for a in [-1., 1.] {
            for b in [-1., 1.] {
                let i = (axis + 1) % 3;
                let j = (axis + 2) % 3;
                // As quatro quinas são compartilhadas exatamente com as faces e
                // com os triângulos de canto: nenhum polígono invade o vizinho.
                let vertices = [
                    [-BEVEL, 1., BEVEL],
                    [BEVEL, 1., BEVEL],
                    [BEVEL, BEVEL, 1.],
                    [-BEVEL, BEVEL, 1.],
                ]
                .map(|v| {
                    let mut point = [0.; 3];
                    point[axis] = v[0];
                    point[i] = a * v[1];
                    point[j] = b * v[2];
                    point
                })
                .to_vec();
                let mut normal = [0.; 3];
                normal[i] = a;
                normal[j] = b;
                faces.push(Face {
                    normal,
                    vertices,
                    label: None,
                });
            }
        }
    }
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [-1., 1.] {
                faces.push(Face {
                    normal: [x, y, z],
                    vertices: vec![
                        [x, y * BEVEL, z * BEVEL],
                        [x * BEVEL, y, z * BEVEL],
                        [x * BEVEL, y * BEVEL, z],
                    ],
                    label: None,
                });
            }
        }
    }
    faces
}

fn projected_faces(camera: &Camera, center: egui::Pos2) -> Vec<ProjectedFace> {
    let direction = camera.direction();
    let mut faces: Vec<_> = cube_faces()
        .into_iter()
        .filter_map(|face| {
            let facing = dot(face.normal, direction) / norm(face.normal);
            // Descartar o limite de perfil elimina contornos quase colineares e suas
            // zonas de clique degeneradas nos planos e nas diagonais exatas.
            if facing <= 1e-7 {
                return None;
            }
            let polygon: Vec<_> = face
                .vertices
                .iter()
                .map(|point| {
                    let p = camera.axis_projection(*point);
                    center + egui::vec2(p[0] as f32, p[1] as f32) * SCALE
                })
                .collect();
            if polygon.iter().any(|point| !point.is_finite()) || signed_area(&polygon).abs() < 0.5 {
                return None;
            }
            let depth = face
                .vertices
                .iter()
                .map(|p| dot(*p, direction))
                .sum::<f64>()
                / face.vertices.len() as f64;
            Some(ProjectedFace {
                normal: face.normal,
                polygon,
                label: face.label,
                depth,
            })
        })
        .collect();
    faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    faces
}

fn signed_area(points: &[egui::Pos2]) -> f32 {
    let Some(origin) = points.first() else {
        return 0.;
    };
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| {
            let a = *a - *origin;
            let b = *b - *origin;
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>()
        * 0.5
}

fn contains(points: &[egui::Pos2], point: egui::Pos2) -> bool {
    if !point.is_finite()
        || points.iter().any(|p| !p.is_finite())
        || points.len() < 3
        || signed_area(points).abs() < 0.5
    {
        return false;
    }
    let mut sign = 0.;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if cross.abs() > 0.02 {
            if sign == 0. {
                sign = cross.signum();
            } else if sign * cross < 0. {
                return false;
            }
        }
    }
    true
}

fn hit_face(faces: &[ProjectedFace], point: egui::Pos2) -> Option<usize> {
    faces
        .iter()
        .enumerate()
        .rev()
        .find(|(_, face)| contains(&face.polygon, point))
        .map(|(index, _)| index)
}

fn paint_faces(painter: &egui::Painter, faces: &[ProjectedFace], hovered: Option<usize>) {
    for (index, face) in faces.iter().enumerate() {
        let fill = if hovered == Some(index) {
            egui::Color32::from_rgb(170, 210, 247)
        } else if face.label.is_some() {
            if face.normal[2].abs() > 0.5 {
                egui::Color32::from_rgb(153, 163, 171)
            } else if face.normal[0].abs() > 0.5 {
                egui::Color32::from_rgb(103, 113, 121)
            } else {
                egui::Color32::from_rgb(121, 131, 139)
            }
        } else if face.normal[2].abs() > 0.5 {
            egui::Color32::from_rgb(167, 179, 187)
        } else {
            egui::Color32::from_rgb(129, 139, 147)
        };
        // Preenchimento por triângulos e contornos independentes não criam junções
        // com miter ilimitado em ângulos quase planos. Mesmo uma regressão futura
        // de pintura não pode escapar do recorte aplicado pelo chamador.
        let mut mesh = egui::Mesh::default();
        for point in &face.polygon {
            mesh.colored_vertex(*point, fill);
        }
        for corner in 1..face.polygon.len().saturating_sub(1) {
            mesh.add_triangle(0, corner as u32, corner as u32 + 1);
        }
        painter.add(egui::Shape::mesh(mesh));
        for (a, b) in face
            .polygon
            .iter()
            .zip(face.polygon.iter().cycle().skip(1))
            .take(face.polygon.len())
        {
            if a.distance_sq(*b) > 0.25 {
                painter.line_segment(
                    [*a, *b],
                    egui::Stroke::new(0.75, egui::Color32::from_rgb(56, 62, 67)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nator_geometry::sub;
    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn beveled_cube_is_closed_with_shared_edges() {
        use std::collections::BTreeMap;
        let faces = cube_faces();
        assert_eq!(faces.len(), 26);
        let key = |p: Point3| p.map(|v| (v * 100.).round() as i32);
        let mut edges = BTreeMap::new();
        for face in &faces {
            for (a, b) in face
                .vertices
                .iter()
                .zip(face.vertices.iter().cycle().skip(1))
                .take(face.vertices.len())
            {
                let a = key(*a);
                let b = key(*b);
                let edge = if a < b { (a, b) } else { (b, a) };
                *edges.entry(edge).or_insert(0) += 1;
            }
        }
        assert_eq!(edges.len(), 48);
        assert!(edges.values().all(|count| *count == 2));
        // Os limites dos semiespaços demonstram que nenhuma face invade a vizinha.
        for face in &faces {
            let plane = dot(face.normal, face.vertices[0]);
            for other in &faces {
                for point in &other.vertices {
                    assert!(dot(face.normal, *point) <= plane + 1e-12);
                }
            }
        }
    }

    #[test]
    fn selection_uses_polygons_and_rejects_degenerate_regions() -> TestResult<()> {
        assert!(!contains(
            &[egui::pos2(0., 0.), egui::pos2(1., 0.), egui::pos2(2., 0.)],
            egui::pos2(100., 100.)
        ));
        let center = egui::pos2(200., 200.);
        for target in cube_faces() {
            let mut camera = Camera::default();
            camera.set_direction(target.normal);
            let projected = projected_faces(&camera, center);
            let hit =
                hit_face(&projected, center).ok_or("O centro visível não foi selecionado.")?;
            assert_eq!(projected[hit].normal, target.normal);
        }
        let camera = Camera::default();
        let faces = projected_faces(&camera, center);
        let mut rejected_boxes = 0;
        for (index, face) in faces.iter().enumerate() {
            let bounds = egui::Rect::from_points(&face.polygon);
            for point in [
                bounds.left_top(),
                bounds.right_top(),
                bounds.left_bottom(),
                bounds.right_bottom(),
            ] {
                if !contains(&face.polygon, point) {
                    assert_ne!(hit_face(&faces, point), Some(index));
                    rejected_boxes += 1;
                }
            }
        }
        assert!(rejected_boxes > 0);
        Ok(())
    }

    #[test]
    fn tessellation_stays_bounded_at_poles_diagonals_and_quarter_turns() {
        let context = egui::Context::default();
        let center = egui::pos2(260., 180.);
        let allowed = egui::Rect::from_center_size(center, egui::Vec2::splat(150.));
        for yaw in 0..16 {
            for pitch in -6..=6 {
                for turns in 0..4 {
                    let mut camera = Camera::default();
                    camera.yaw = f64::from(yaw) * std::f64::consts::PI / 8.;
                    camera.pitch = f64::from(pitch) * std::f64::consts::PI / 12.;
                    // Incluir posições quase coplanares: eram as mais sensíveis ao traço
                    // unido de polígonos degenerados, que poderia atravessar o viewport.
                    if yaw % 2 == 1 {
                        camera.pitch += 1e-10;
                    }
                    for _ in 0..turns {
                        camera.rotate_quarter_turn(true);
                    }
                    let faces = projected_faces(&camera, center);
                    let mut output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(600., 400.),
                            )),
                            ..Default::default()
                        },
                        |ui| paint_faces(&ui.painter().with_clip_rect(allowed), &faces, None),
                    );
                    output.textures_delta.clear();
                    let primitives = context.tessellate(output.shapes, output.pixels_per_point);
                    for primitive in primitives {
                        if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                            for vertex in mesh.vertices {
                                assert!(vertex.pos.is_finite());
                                assert!(
                                    allowed.contains(vertex.pos),
                                    "Vértice escapou do cubo: {:?}",
                                    vertex.pos
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn clicks_snap_visible_faces_in_both_projection_modes() -> TestResult<()> {
        for projection in [
            nator_renderer::Projection::Orthographic,
            nator_renderer::Projection::Perspective,
        ] {
            let context = egui::Context::default();
            let mut camera = Camera::default();
            camera.projection = projection;
            for _ in 0..3 {
                frame(&context, &mut camera, vec![]);
            }
            let area = rect(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620., 400.),
            ));
            let center = body_rect(area).center();
            let faces = projected_faces(&camera, center);
            let top = faces
                .iter()
                .find(|f| f.normal == [0., 0., 1.])
                .ok_or("Face superior ausente")?;
            let sum = top
                .polygon
                .iter()
                .fold(egui::Vec2::ZERO, |s, p| s + p.to_vec2());
            let position = (sum / top.polygon.len() as f32).to_pos2();
            frame(&context, &mut camera, pointer(position, true));
            let released = frame(&context, &mut camera, pointer(position, false));
            assert!(!released.canvas_clicked);
            assert!(norm(sub(camera.direction(), [0., 0., 1.])) < 1e-12);
            assert_eq!(camera.projection, projection);
        }
        Ok(())
    }

    #[test]
    fn home_restores_orientation_roll_pan_and_zoom_without_changing_projection() {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let area = rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(620., 400.),
        ));
        camera.projection = nator_renderer::Projection::Perspective;
        camera.set_direction([1., 0., 0.]);
        camera.pan([60., -35.], 400.);
        camera.span = 0.3;
        camera.rotate_quarter_turn(true);
        let position = area.min + egui::vec2(17., 30.);
        frame(&context, &mut camera, pointer(position, true));
        frame(&context, &mut camera, pointer(position, false));
        assert!(camera.roll_angle().abs() < 1e-12);
        assert!(norm(sub(camera.direction(), Camera::default().direction())) < 1e-12);
        assert_eq!(camera.projection, nator_renderer::Projection::Perspective);
        let mut expected = Camera::default();
        expected.projection = camera.projection;
        expected.fit(620. / 400.);
        assert!((camera.span - expected.span).abs() < 1e-12);
        let bounds = nator_geometry::Bounds {
            min: [0.; 3],
            max: [10., 20., 30.],
        };
        assert!(
            norm(sub(
                camera.world_on_target_plane([0.; 2], bounds, 620. / 400.),
                bounds.center()
            )) < 1e-12
        );
    }

    #[test]
    fn two_filled_arrows_orbit_in_both_directions_without_clicking_the_canvas() {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let area = rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(620., 400.),
        ));
        for offset in [egui::vec2(110., 25.), egui::vec2(183., 98.)] {
            let original = camera.direction();
            let position = area.min + offset;
            frame(&context, &mut camera, pointer(position, true));
            let released = frame(&context, &mut camera, pointer(position, false));
            assert!(!released.canvas_clicked);
            let angle = dot(original, camera.direction()).clamp(-1., 1.).acos();
            assert!((angle - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
            for pressed in [true, false] {
                let mut events = pointer(position, pressed);
                for event in &mut events {
                    if let egui::Event::PointerButton { modifiers, .. } = event {
                        modifiers.shift = true;
                    }
                }
                let released =
                    frame_with_modifiers(&context, &mut camera, events, egui::Modifiers::SHIFT);
                assert!(!released.canvas_clicked);
            }
            assert!(norm(sub(camera.direction(), original)) < 1e-10);
            for pressed in [true, false] {
                let mut events = pointer(position, pressed);
                for event in &mut events {
                    if let egui::Event::PointerButton { button, .. } = event {
                        *button = egui::PointerButton::Secondary;
                    }
                }
                let released = frame(&context, &mut camera, events);
                assert!(!released.canvas_clicked);
            }
            assert!(norm(sub(camera.direction(), original)) > 0.1);
            // Restaurar a base antes de conferir o outro eixo.
            camera.set_direction(original);
        }
    }

    #[test]
    fn cube_menu_preserves_projection_fit_and_quarter_turn_controls() -> TestResult<()> {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let area = rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(620., 400.),
        ));
        let direction = camera.direction();
        for (label, turn) in [
            (T::Perspective, 0.),
            (T::ViewRollRight, 90.),
            (T::ViewRollLeft, 0.),
            (T::Orthographic, 0.),
            (T::Fit, 0.),
        ] {
            let mut menu = frame(&context, &mut camera, vec![]);
            for pressed in [true, false] {
                let mut events = pointer(body_rect(area).center(), pressed);
                for event in &mut events {
                    if let egui::Event::PointerButton { button, .. } = event {
                        *button = egui::PointerButton::Secondary;
                    }
                }
                menu = frame(&context, &mut camera, events);
                assert!(!menu.canvas_clicked);
            }
            // O popup estabiliza o tamanho no quadro seguinte antes do clique.
            let output = frame(&context, &mut camera, vec![]);
            let position = output
                .commands
                .shapes
                .iter()
                .chain(&menu.commands.shapes)
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == text(label) => {
                        Some(t.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .ok_or("Item ausente no menu do cubo")?;
            for pressed in [true, false] {
                frame(&context, &mut camera, pointer(position, pressed));
            }
            assert_eq!(camera.direction(), direction);
            assert!((camera.roll_angle().to_degrees() - turn).abs() < 1e-9);
            if matches!(label, T::Perspective) {
                assert_eq!(camera.projection, nator_renderer::Projection::Perspective);
            }
            if matches!(label, T::Orthographic) {
                assert_eq!(camera.projection, nator_renderer::Projection::Orthographic);
            }
            if matches!(label, T::Fit) {
                let mut expected = Camera::default();
                expected.fit(620. / 400.);
                assert!((camera.span - expected.span).abs() < 1e-12);
            }
        }
        Ok(())
    }

    struct Frame {
        commands: egui::FullOutput,
        canvas_clicked: bool,
        canvas_dragged: bool,
    }
    fn frame(context: &egui::Context, camera: &mut Camera, events: Vec<egui::Event>) -> Frame {
        frame_with_modifiers(context, camera, events, egui::Modifiers::NONE)
    }
    fn frame_with_modifiers(
        context: &egui::Context,
        camera: &mut Camera,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) -> Frame {
        let mut events = events;
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let mut canvas_clicked = false;
        let mut canvas_dragged = false;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640., 420.),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let (canvas, response) =
                    ui.allocate_exact_size(egui::vec2(620., 400.), egui::Sense::click_and_drag());
                canvas_clicked = response.clicked();
                canvas_dragged = response.dragged();
                show(ui, camera, canvas);
            },
        );
        output.textures_delta.clear();
        Frame {
            commands: output,
            canvas_clicked,
            canvas_dragged,
        }
    }
    fn pointer(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    #[test]
    fn dragging_rotates_without_snapping_or_selecting_the_model() {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(620., 400.));
        let position = body_rect(rect(canvas)).center();
        let original = camera.direction();
        frame(&context, &mut camera, pointer(position, true));
        let dragging = frame(
            &context,
            &mut camera,
            vec![egui::Event::PointerMoved(position + egui::vec2(28., 17.))],
        );
        assert!(norm(sub(camera.direction(), original)) > 0.01);
        assert!(!dragging.canvas_dragged);
        let before_release = camera.direction();
        let released = frame(
            &context,
            &mut camera,
            pointer(position + egui::vec2(28., 17.), false),
        );
        assert!(norm(sub(camera.direction(), before_release)) < 1e-12);
        assert!(!released.canvas_clicked);
        assert!(
            released
                .commands
                .viewport_output
                .values()
                .all(|v| v.commands.is_empty())
        );
    }
}
