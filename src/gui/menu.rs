use egui::PopupCloseBehavior::CloseOnClick;
use egui::containers::menu::MenuConfig;
use egui::{Button, TextEdit};

use crate::app::PuzzleApp;
use crate::examples::EXAMPLES;
use crate::puzzle_io::{
    PUZZLE_FORMAT_VERSION, PuzzleExport, trigger_download, trigger_file_picker,
};

pub fn build_menu_bar(app: &mut PuzzleApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::MenuBar::new()
            .config(MenuConfig::new().close_behavior(CloseOnClick))
            .ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import Puzzle...").clicked() {
                        trigger_file_picker(app.pending_import.clone());
                        ui.close();
                    }
                    if ui.button("Export Puzzle...").clicked() {
                        let export = PuzzleExport {
                            version: PUZZLE_FORMAT_VERSION,
                            puzzle_name: app.puzzle_name.clone(),
                            params: app.params.clone(),
                            axis_defs: app.axis_defs.clone(),
                            orbit_state: app.orbit_state.clone(),
                        };
                        if let Ok(yaml) = export.to_yaml() {
                            let base_name = app
                                .puzzle_name
                                .clone()
                                .unwrap_or_else(|| "puzzle".to_string());
                            trigger_download(&yaml, &base_name);
                            app.puzzle_name = Some(base_name);
                        }
                        ui.close();
                    }
                });
                ui.menu_button("Examples", |ui| {
                    for example in EXAMPLES.iter() {
                        if ui.button(example.name).clicked() {
                            if let Some(yaml) = example.to_yaml() {
                                app.pending_import
                                    .borrow_mut()
                                    .replace((example.name.to_string(), yaml));
                            }
                            ui.close();
                        }
                    }
                });
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

                ui.add_space(20.0);

                if let Some(buf) = &mut app.puzzle_name_edit {
                    let mut do_apply = false;
                    let mut cancel = false;
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(buf).desired_width(120.0));
                        if ui.button("OK").clicked() {
                            do_apply = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                    if do_apply {
                        let trimmed = buf.trim().to_string();
                        if !trimmed.is_empty() {
                            app.puzzle_name = Some(trimmed);
                            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                                document.set_title(&format!(
                                    "Puzzle Explorer - {}",
                                    app.puzzle_name.as_deref().unwrap_or("")
                                ));
                            }
                        }
                        app.puzzle_name_edit = None;
                    }
                    if cancel {
                        app.puzzle_name_edit = None;
                    }
                } else {
                    let blank = "".to_string();
                    let name = &app.puzzle_name.as_ref().unwrap_or(&blank);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(name.as_str()).color(egui::Color32::GRAY),
                        )
                        .truncate(),
                    );
                    if ui.add(Button::new("✏").small()).clicked() {
                        app.puzzle_name_edit = Some(name.to_string());
                    }
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        if name.is_empty() {
                            document.set_title("Puzzle Explorer");
                        } else {
                            document.set_title(&format!("Puzzle Explorer - {}", name));
                        }
                    }
                }
            });
    });
}
