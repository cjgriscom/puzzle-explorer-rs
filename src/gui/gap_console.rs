use crate::app::PuzzleApp;
use crate::gap::GapState;
use crate::gui::{GAP_CONSOLE_POS, GAP_CONSOLE_WIDTH};

pub fn build_gap_console_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    let gap_title = match &app.gap_manager.state {
        GapState::Loading(_, _) => "GAP Console (loading...)",
        GapState::Error(_) => "GAP Console (error)",
        _ => "GAP Console",
    };

    egui::Window::new(gap_title)
        .id("GAP Console".into()) // unchanging ID
        .default_pos(GAP_CONSOLE_POS)
        .default_width(GAP_CONSOLE_WIDTH)
        .default_open(false)
        .show(ctx, |ui| match &app.gap_manager.state {
            GapState::NotStarted => {
                ui.label("GAP is not started.");
            }
            GapState::Loading(status, progress) => {
                ui.label(status);
                ui.add(egui::ProgressBar::new(*progress));
                ui.spinner();
            }
            GapState::Error(err) => {
                ui.label(format!("Error loading GAP: {}", err));
            }
            GapState::Ready => {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.monospace(&app.gap_manager.output_history);
                    });

                ui.horizontal(|ui| {
                    if ui
                        .button("Reset")
                        .on_hover_text("Reset GAP worker")
                        .clicked()
                    {
                        app.pending_gap_requests.clear();
                        app.gap_cache.clear();
                        app.gap_manager.reset();
                        app.gap_manager.init(ctx.clone());
                    }
                    ui.label("gap>");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.gap_input).desired_width(f32::INFINITY),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let cmd = app.gap_input.clone();
                        app.gap_input.clear();
                        app.gap_manager.send_command(&cmd);
                        response.request_focus();
                    }
                });
            }
        });
}
