use crate::{PuzzleApp, gui::CONTROLS_POS};

pub fn build_controls_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    egui::Window::new("Controls")
        .default_pos(CONTROLS_POS)
        .open(&mut app.window_state.show_controls)
        .show(ctx, |ui| {
            ui.label("Mouse controls:");
            ui.label("- Left-drag: rotate sphere");
            ui.label("- Middle-drag: pan sphere");
            ui.label("- Mouse wheel: zoom");
            ui.separator();
            ui.label("Touch controls:");
            ui.label("- One-finger drag: rotate sphere");
            ui.label("- Two-finger drag: pan sphere");
            ui.label("- Pinch: zoom");
            ui.separator();
            ui.label("Rotation shortcuts:");
            ui.label("- A: rotate axis A clockwise");
            ui.label("- Shift+A: rotate axis A counter-clockwise");
            ui.label("- B: rotate axis B clockwise");
            ui.label("- Shift+B: rotate axis B counter-clockwise");
            ui.label("- (Shift+) C thru Z: additional axes");
        });
}
