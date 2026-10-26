//! Controles da janela sem decoração nativa. O fechamento é devolvido ao editor
//! para respeitar tarefas em curso e a confirmação de projetos modificados.
use super::{
    T, egui,
    icons::{self, Icon},
    text, theme,
};

/// Três botões de 30 pontos com dois intervalos de cinco pontos.
pub(super) const CONTROLS_WIDTH: f32 = 100.;

pub(super) struct WindowButtons {
    minimize: egui::Response,
    maximize: egui::Response,
    pub close: egui::Response,
}

pub(super) fn controls(ui: &mut egui::Ui) -> WindowButtons {
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let buttons = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.;
            let minimize = icons::button(ui, Icon::Minimize, text(T::MinimizeWindow), false, false);
            let maximize = icons::button(
                ui,
                if maximized {
                    Icon::Restore
                } else {
                    Icon::Maximize
                },
                text(if maximized {
                    T::RestoreWindow
                } else {
                    T::MaximizeWindow
                }),
                false,
                false,
            );
            let close = icons::button(ui, Icon::Close, text(T::CloseWindow), false, false);
            if close.hovered() {
                ui.painter().rect_filled(close.rect, 5, theme::DANGER);
                icons::paint_with_ink(
                    ui.painter(),
                    egui::Rect::from_center_size(close.rect.center(), egui::Vec2::splat(18.)),
                    Icon::Close,
                    true,
                    egui::Color32::WHITE,
                );
            }
            WindowButtons {
                minimize,
                maximize,
                close,
            }
        })
        .inner;
    if buttons.minimize.clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    if buttons.maximize.clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    buttons
}

/// Registrar antes dos botões mantém o arraste no fundo da barra: controles,
/// menus e abas interativos recebem seus próprios cliques, sem mover a janela.
pub(super) fn drag_region(ui: &egui::Ui, rect: egui::Rect) {
    let response = ui.interact(
        rect,
        egui::Id::new("window_drag_region"),
        // O movimento será assumido pelo sistema no pressionamento. Pedir também
        // o arraste ao egui faria o fundo disputar o gesto com botões só de clique.
        egui::Sense::click().difference(egui::Sense::focusable_noninteractive()),
    );
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    } else if response.is_pointer_button_down_on()
        && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
    {
        // Winit/Wayland precisam do evento de pressão recente para iniciar o arraste.
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

/// Bordas estreitas substituem a moldura de redimensionamento do sistema.
/// Os cantos têm uma área maior; os controles da barra ficam fora dessa faixa.
pub(super) fn resize_handles(ui: &egui::Ui) {
    if ui.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let rect = ui.ctx().content_rect();
    for (index, (area, direction, cursor)) in resize_regions(rect).into_iter().enumerate() {
        let response = ui
            .interact(
                area,
                egui::Id::new(("window_resize", index)),
                egui::Sense::drag().difference(egui::Sense::focusable_noninteractive()),
            )
            .on_hover_cursor(cursor);
        if response.is_pointer_button_down_on()
            && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
    }
}

fn resize_regions(rect: egui::Rect) -> [(egui::Rect, egui::ResizeDirection, egui::CursorIcon); 8] {
    use egui::{CursorIcon as C, Rect, ResizeDirection as D, pos2};
    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    let edge = 4.;
    let corner = 9.;
    [
        (
            Rect::from_min_max(pos2(l + corner, t), pos2(r - corner, t + edge)),
            D::North,
            C::ResizeNorth,
        ),
        (
            Rect::from_min_max(pos2(l + corner, b - edge), pos2(r - corner, b)),
            D::South,
            C::ResizeSouth,
        ),
        (
            Rect::from_min_max(pos2(l, t + corner), pos2(l + edge, b - corner)),
            D::West,
            C::ResizeWest,
        ),
        (
            Rect::from_min_max(pos2(r - edge, t + corner), pos2(r, b - corner)),
            D::East,
            C::ResizeEast,
        ),
        (
            Rect::from_min_max(pos2(l, t), pos2(l + corner, t + corner)),
            D::NorthWest,
            C::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(pos2(r - corner, t), pos2(r, t + corner)),
            D::NorthEast,
            C::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(pos2(l, b - corner), pos2(l + corner, b)),
            D::SouthWest,
            C::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(pos2(r - corner, b - corner), pos2(r, b)),
            D::SouthEast,
            C::ResizeSouthEast,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;
    struct Frame {
        centers: [egui::Pos2; 3],
        close_requested: bool,
        commands: Vec<egui::ViewportCommand>,
    }
    fn frame(
        context: &egui::Context,
        events: Vec<egui::Event>,
        maximized: bool,
    ) -> TestResult<Frame> {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900., 600.),
            )),
            time: Some(context.input(|i| i.time) + 0.05),
            events,
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .maximized = Some(maximized);
        let mut response = None;
        let mut output = context.run_ui(input, |ui| {
            drag_region(
                ui,
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 32.)),
            );
            let buttons = controls(ui);
            response = Some((
                [
                    buttons.minimize.rect.center(),
                    buttons.maximize.rect.center(),
                    buttons.close.rect.center(),
                ],
                buttons.close.clicked(),
            ));
            resize_handles(ui);
        });
        // Este ensaio observa eventos e comandos; a captura GPU valida os pixels.
        output.textures_delta.clear();
        let (centers, close_requested) = response.ok_or("Controles ausentes.")?;
        let commands = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.commands.clone())
            .unwrap_or_default();
        Ok(Frame {
            centers,
            close_requested,
            commands,
        })
    }
    fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    #[test]
    fn controls_minimize_maximize_and_restore_without_dragging() -> TestResult<()> {
        for (index, maximized, expected) in [
            (0, false, egui::ViewportCommand::Minimized(true)),
            (1, false, egui::ViewportCommand::Maximized(true)),
            (1, true, egui::ViewportCommand::Maximized(false)),
        ] {
            let context = egui::Context::default();
            let first = frame(&context, vec![], maximized)?;
            let pressed = frame(&context, pointer(first.centers[index], true), maximized)?;
            let released = frame(&context, pointer(first.centers[index], false), maximized)?;
            assert!(released.commands.contains(&expected));
            assert!(!pressed.commands.contains(&egui::ViewportCommand::StartDrag));
            assert!(!released.close_requested);
        }
        Ok(())
    }
    #[test]
    fn close_returns_to_the_editor_without_forcing_native_shutdown() -> TestResult<()> {
        let context = egui::Context::default();
        let first = frame(&context, vec![], false)?;
        frame(&context, pointer(first.centers[2], true), false)?;
        let released = frame(&context, pointer(first.centers[2], false), false)?;
        assert!(released.close_requested);
        assert!(!released.commands.contains(&egui::ViewportCommand::Close));
        Ok(())
    }
    #[test]
    fn title_background_drags_and_double_click_maximizes() -> TestResult<()> {
        let context = egui::Context::default();
        frame(&context, vec![], false)?;
        let position = egui::pos2(350., 15.);
        let pressed = frame(&context, pointer(position, true), false)?;
        assert!(pressed.commands.contains(&egui::ViewportCommand::StartDrag));
        frame(&context, pointer(position, false), false)?;
        frame(&context, pointer(position, true), false)?;
        let released = frame(&context, pointer(position, false), false)?;
        assert!(
            released
                .commands
                .contains(&egui::ViewportCommand::Maximized(true))
        );
        Ok(())
    }
    #[test]
    fn border_resize_stays_outside_content_and_is_disabled_when_maximized() -> TestResult<()> {
        let context = egui::Context::default();
        frame(&context, vec![], false)?;
        let pressed = frame(&context, pointer(egui::pos2(898., 300.), true), false)?;
        assert!(
            pressed
                .commands
                .contains(&egui::ViewportCommand::BeginResize(
                    egui::ResizeDirection::East
                ))
        );
        let context = egui::Context::default();
        frame(&context, vec![], true)?;
        let pressed = frame(&context, pointer(egui::pos2(898., 300.), true), true)?;
        assert!(
            !pressed
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::BeginResize(_)))
        );
        let center = egui::pos2(450., 300.);
        assert!(
            resize_regions(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900., 600.)
            ))
            .iter()
            .all(|(area, _, _)| !area.contains(center))
        );
        Ok(())
    }
}
