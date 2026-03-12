use egui::PopupCloseBehavior::CloseOnClick;
use egui::containers::menu::MenuConfig;

use crate::app::PuzzleApp;

pub fn build_menu_bar(app: &mut PuzzleApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new()
            .config(MenuConfig::new().close_behavior(CloseOnClick))
            .ui(ui, |ui| {
                ui.menu_button("Tools", |ui| {
                    if ui.button("Measure Axis Angle").clicked() {
                        app.window_state.show_measure_axis_angle = true;
                    }
                    ui.separator();
                    ui.add_enabled_ui(!app.window_state.show_gap_console, |ui| {
                        if ui.button("Show Gap Console").clicked() {
                            app.window_state.show_gap_console = true;
                        }
                    });
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Mouse/Keyboard Controls").clicked() {
                        app.window_state.show_controls = true;
                    }
                    ui.separator();
                    if ui.button("Puzzle Explorer on GitHub").clicked()
                        && let Some(w) = web_sys::window()
                    {
                        let _ = w.open_with_url_and_target(
                            "https://github.com/cjgriscom/puzzle-explorer",
                            "_blank",
                        );
                    }
                    ui.label(format!(
                        "Version: {}-{}",
                        env!("CARGO_PKG_VERSION"),
                        app.build_hash
                    ));
                    ui.label(format!("Build: {}", env!("BUILD_DATE")));
                });
            });
    });
}
