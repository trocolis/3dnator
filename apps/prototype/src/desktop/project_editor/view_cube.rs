//! Navegador de orientação: um poliedro fechado com seis faces, doze chanfros e
//! oito cantos. Seleção e desenho usam os mesmos polígonos projetados, sem caixas
//! sobrepostas por face. Toda a pintura fica recortada à área deste controle.
use super::{
    Camera, Point3, T, egui,
    icons::{self, Icon},
    text, theme,
};
use nator_geometry::{dot, norm};

const SIZE: egui::Vec2 = egui::vec2(192., 196.);
const BEVEL: f64 = 0.72;
const SCALE: f32 = 34.;

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
    label_direction: egui::Vec2,
}

pub(super) fn rect(canvas: egui::Rect) -> egui::Rect {
    if canvas.width() < SIZE.x + 16. || canvas.height() < SIZE.y + 16. {
        return egui::Rect::NOTHING;
    }
    egui::Rect::from_min_size(canvas.right_top() + egui::vec2(-SIZE.x - 8., 8.), SIZE)
}

/// Uma única região de arraste impede que a seleção mude de face durante um
/// movimento. Home, rotação e órbita têm áreas próprias ao redor do cubo.
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
        |x, y| egui::Rect::from_min_size(area.min + egui::vec2(x, y), egui::Vec2::splat(24.));
    if icon_control(
        ui,
        &painter,
        "view_home",
        control(4., 35.),
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
    for (x, icon, label, clockwise) in [
        (134., Icon::RotateCounterclockwise, T::ViewRotateLeft, false),
        (160., Icon::RotateClockwise, T::ViewRotateRight, true),
    ] {
        if icon_control(
            ui,
            &painter,
            ("view_roll", clockwise),
            control(x, 35.),
            icon,
            label,
        )
        .clicked()
        {
            camera.rotate_quarter_turn(clockwise);
            ui.ctx().request_repaint();
        }
    }
    // As setas externas orbitam em passos de 45°; as pequenas giram a tela em
    // 90°. Áreas disjuntas impedem que esses cliques atinjam faces ou o modelo.
    let step = std::f64::consts::FRAC_PI_4 / 0.008;
    for (index, offset, size, curve, label, delta) in [
        (
            0,
            [31., 4.],
            [61., 27.],
            [[0.95, 0.8], [0.55, 0.05], [0.08, 0.75]],
            T::ViewOrbitLeft,
            [-step, 0.],
        ),
        (
            1,
            [92., 4.],
            [61., 27.],
            [[0.05, 0.8], [0.45, 0.05], [0.92, 0.75]],
            T::ViewOrbitRight,
            [step, 0.],
        ),
        (
            2,
            [159., 62.],
            [29., 58.],
            [[0.25, 0.95], [0.95, 0.5], [0.3, 0.08]],
            T::ViewOrbitUp,
            [0., -step],
        ),
        (
            3,
            [159., 120.],
            [29., 58.],
            [[0.25, 0.05], [0.95, 0.5], [0.3, 0.92]],
            T::ViewOrbitDown,
            [0., step],
        ),
    ] {
        let rect = egui::Rect::from_min_size(
            area.min + egui::vec2(offset[0], offset[1]),
            egui::vec2(size[0], size[1]),
        );
        if curved_control(ui, &painter, index, rect, curve, label).clicked() {
            camera.orbit(delta);
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
    response.clone().on_hover_text(text(T::ViewCubeHint));
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
    egui::Rect::from_center_size(area.min + egui::vec2(94., 119.), egui::Vec2::splat(118.))
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
    icons::paint(painter, rect.shrink(3.), icon, true);
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
    let points: Vec<_> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.;
            let q: [f32; 2] = std::array::from_fn(|k| {
                (1. - t).powi(2) * curve[0][k]
                    + 2. * t * (1. - t) * curve[1][k]
                    + t * t * curve[2][k]
            });
            rect.min + egui::vec2(q[0] * rect.width(), q[1] * rect.height())
        })
        .collect();
    let color = if response.hovered() {
        theme::ACCENT
    } else {
        theme::MUTED
    };
    painter.add(egui::Shape::line(
        points.clone(),
        egui::Stroke::new(3., color),
    ));
    if let Some((tip, preceding)) = points
        .last()
        .zip(points.get(points.len().saturating_sub(2)))
    {
        let direction = (*tip - *preceding).normalized();
        let perpendicular = egui::vec2(-direction.y, direction.x);
        painter.add(egui::Shape::convex_polygon(
            vec![
                *tip,
                *tip - direction * 9. + perpendicular * 5.,
                *tip - direction * 9. - perpendicular * 5.,
            ],
            color,
            egui::Stroke::NONE,
        ));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text(label)));
    response.on_hover_text(text(label))
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
            let label_axis = if face.normal[0].abs() > 0.5 {
                [0., face.normal[0], 0.]
            } else if face.normal[1].abs() > 0.5 {
                [-face.normal[1], 0., 0.]
            } else {
                [1., 0., 0.]
            };
            let axis = camera.axis_projection(label_axis);
            Some(ProjectedFace {
                label_direction: egui::vec2(axis[0] as f32, axis[1] as f32),
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
                egui::Color32::from_rgb(232, 240, 249)
            } else if face.normal[0].abs() > 0.5 {
                egui::Color32::from_rgb(201, 220, 239)
            } else {
                egui::Color32::from_rgb(216, 231, 246)
            }
        } else {
            egui::Color32::from_rgb(185, 205, 226)
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
                painter.line_segment([*a, *b], egui::Stroke::new(1., theme::MUTED));
            }
        }
        if let Some(label) = face.label
            && signed_area(&face.polygon).abs() > 650.
        {
            let sum = face
                .polygon
                .iter()
                .fold(egui::Vec2::ZERO, |s, p| s + p.to_vec2());
            let center = (sum / face.polygon.len() as f32).to_pos2();
            let span = face.label_direction.length() * 2. * BEVEL as f32 * SCALE;
            let font = (span / text(label).chars().count() as f32 * 1.5).clamp(7.5, 10.);
            let galley = painter.layout_no_wrap(
                text(label).into(),
                egui::FontId::proportional(font),
                theme::INK,
            );
            // O texto acompanha a orientação da face e o roll da câmera. Não
            // corrigir o ângulo para mantê-lo horizontal: isso esconderia os giros.
            let pos = center - galley.size() * 0.5;
            let angle = face.label_direction.y.atan2(face.label_direction.x);
            painter.add(
                egui::epaint::TextShape::new(pos, galley, theme::INK)
                    .with_angle_and_anchor(angle, egui::Align2::CENTER_CENTER),
            );
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
        let allowed = egui::Rect::from_center_size(center, egui::Vec2::splat(114.));
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
    fn roll_buttons_step_through_quarter_turns_and_home_restores_the_camera() {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let area = rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(620., 400.),
        ));
        let original = camera.direction();
        for angle in [90., 180., 270., 0.] {
            let position = area.min + egui::vec2(172., 47.);
            frame(&context, &mut camera, pointer(position, true));
            let released = frame(&context, &mut camera, pointer(position, false));
            assert!(!released.canvas_clicked);
            assert!((camera.roll_angle().to_degrees() - angle).abs() < 1e-9);
            assert_eq!(camera.direction(), original);
        }
        let position = area.min + egui::vec2(146., 47.);
        frame(&context, &mut camera, pointer(position, true));
        frame(&context, &mut camera, pointer(position, false));
        assert!((camera.roll_angle().to_degrees() - 270.).abs() < 1e-9);
        camera.projection = nator_renderer::Projection::Perspective;
        camera.set_direction([1., 0., 0.]);
        camera.pan([60., -35.], 400.);
        camera.span = 0.3;
        camera.rotate_quarter_turn(true);
        let position = area.min + egui::vec2(16., 47.);
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
    fn external_arrows_orbit_45_degrees_without_clicking_the_canvas() {
        let context = egui::Context::default();
        let mut camera = Camera::default();
        for _ in 0..3 {
            frame(&context, &mut camera, vec![]);
        }
        let area = rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(620., 400.),
        ));
        for offset in [
            egui::vec2(61., 17.),
            egui::vec2(122., 17.),
            egui::vec2(173., 92.),
            egui::vec2(173., 150.),
        ] {
            let before = camera.direction();
            let position = area.min + offset;
            frame(&context, &mut camera, pointer(position, true));
            let after = frame(&context, &mut camera, pointer(position, false));
            assert!(!after.canvas_clicked);
            let angle = dot(before, camera.direction()).clamp(-1., 1.).acos();
            assert!((angle - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
        }
    }

    struct Frame {
        commands: egui::FullOutput,
        canvas_clicked: bool,
        canvas_dragged: bool,
    }
    fn frame(context: &egui::Context, camera: &mut Camera, events: Vec<egui::Event>) -> Frame {
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
