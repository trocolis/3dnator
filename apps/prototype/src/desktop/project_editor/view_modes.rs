//! Modos de câmera da barra de navegação. As ferramentas leem o botão esquerdo direto da
//! entrada do egui, então Arrastar e Rotacionar agem antes dela: o botão esquerdo pressionado
//! no canvas vira o botão do meio, que já move a câmera. Assim nenhuma ferramenta precisa
//! conhecer os modos, e a que estiver aberta fica suspensa sem perder o estado.
use super::{Editor, egui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CameraMode {
    Pan,
    Orbit,
}
impl CameraMode {
    pub fn cursor(self, dragging: bool) -> egui::CursorIcon {
        match (self, dragging) {
            (Self::Pan, false) => egui::CursorIcon::Grab,
            (Self::Pan, true) => egui::CursorIcon::Grabbing,
            (Self::Orbit, _) => egui::CursorIcon::AllScroll,
        }
    }
}

/// Lembra a pressão repassada, porque a soltura precisa seguir o mesmo caminho mesmo que o
/// modo mude ou o cursor saia do canvas durante o arraste.
#[derive(Default)]
pub(super) struct PrimaryRoute {
    pressed: bool,
}
impl PrimaryRoute {
    fn apply(&mut self, events: &mut [egui::Event], starts_here: impl Fn(egui::Pos2) -> bool) {
        for event in events {
            let egui::Event::PointerButton {
                pos,
                button,
                pressed,
                ..
            } = event
            else {
                continue;
            };
            if *button != egui::PointerButton::Primary {
                continue;
            }
            if *pressed {
                self.pressed = starts_here(*pos);
            }
            if self.pressed {
                *button = egui::PointerButton::Middle;
                self.pressed = *pressed;
            }
        }
    }
}

impl Editor {
    /// Só o canvas descoberto passa à câmera: botões, cubo e painéis ficam em camadas
    /// próprias e continuam recebendo o clique.
    pub(super) fn route_camera_input(
        &mut self,
        context: &egui::Context,
        input: &mut egui::RawInput,
    ) {
        let active = self.shell.camera_mode.is_some();
        let canvas = self.canvas_rect;
        self.shell
            .camera_press
            .apply(&mut input.events, |position| {
                active
                    && canvas.contains(position)
                    && context
                        .layer_id_at(position)
                        .is_none_or(|layer| layer.order == egui::Order::Background)
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(10., 10.),
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn canvas_press_goes_to_the_camera_until_release() {
        let mut route = PrimaryRoute::default();
        let mut events = [button(egui::PointerButton::Primary, true)];
        route.apply(&mut events, |_| true);
        assert_eq!(events[0], button(egui::PointerButton::Middle, true));
        // A soltura segue a pressão mesmo fora da área que a iniciou.
        let mut events = [button(egui::PointerButton::Primary, false)];
        route.apply(&mut events, |_| false);
        assert_eq!(events[0], button(egui::PointerButton::Middle, false));
        // A pressão seguinte volta a ser avaliada.
        let mut events = [button(egui::PointerButton::Primary, true)];
        route.apply(&mut events, |_| false);
        assert_eq!(events[0], button(egui::PointerButton::Primary, true));
    }

    #[test]
    fn other_buttons_and_presses_outside_the_canvas_are_kept() {
        let mut route = PrimaryRoute::default();
        let mut events = [
            button(egui::PointerButton::Secondary, true),
            button(egui::PointerButton::Primary, true),
            button(egui::PointerButton::Primary, false),
        ];
        let expected = events.clone();
        route.apply(&mut events, |_| false);
        assert_eq!(events, expected);
    }
}
