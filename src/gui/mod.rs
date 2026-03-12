use crate::PuzzleApp;

pub mod axis_definitions;
pub mod controls;
pub mod gap_console;
pub mod measure_axis_angle;
pub mod menu;
pub mod orbit_analysis;
pub mod puzzle_params;

// --- Constants ---
pub const AXIS_DEFINITIONS_POS: [f32; 2] = [20.0, 40.0];
pub const PUZZLE_PARAMS_POS: [f32; 2] = [20.0, 225.0];
pub const ORBIT_ANALYSIS_POS: [f32; 2] = [20.0, 450.0];
pub const GAP_CONSOLE_POS: [f32; 2] = [460.0, 40.0];
pub const CONTROLS_POS: [f32; 2] = [460.0, 90.0];

pub const AXIS_DEFINITIONS_WIDTH: f32 = 400.0;
pub const PUZZLE_PARAMS_WIDTH: f32 = 400.0;
pub const ORBIT_ANALYSIS_WIDTH: f32 = 400.0;
pub const GAP_CONSOLE_WIDTH: f32 = 500.0;

pub const MAX_PUZZLE_AXES: usize = 26;
pub const MIN_N: u32 = 2;
pub const MAX_N: u32 = 8;
pub const MIN_COLAT: f32 = 10.0;
pub const MAX_COLAT: f32 = 170.0;
pub const COLAT_STEP: f64 = 0.1;
pub const COLAT_DECIMALS: usize = 1;
pub const COLAT_SPEED: f64 = 0.05;
pub const EULER_SPEED: f64 = 0.001;
pub const EULER_DECIMALS: usize = 4;
pub const AXIS_ANGLE_SPEED: f64 = 0.01;
pub const AXIS_ANGLE_DECIMALS: usize = 4;

// --- Entry Point ---

pub fn build_windows(app: &mut PuzzleApp, ctx: &egui::Context) {
    menu::build_menu_bar(app, ctx);
    controls::build_controls_window(app, ctx);
    measure_axis_angle::build_measure_axis_angle_window(app, ctx);
    gap_console::build_gap_console_window(app, ctx);

    orbit_analysis::build_orbit_analysis_window(app, ctx);
    axis_definitions::build_axis_definitions_window(app, ctx);
    puzzle_params::build_puzzle_params_window(app, ctx);
}

// --- Custom UI Elements ---

fn toggle_ui(ui: &mut egui::Ui, on: &mut bool, mut color: Option<egui::Color32>) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if !*on {
        color = None;
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.1);
        let visuals = ui.style().interact_selectable(&response, *on);
        let color = color.unwrap_or(visuals.bg_fill);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            color,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, color, visuals.fg_stroke);
    }

    response
}

/// Custom toggle element
pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on, None)
}

/// Custom toggle element with customizable foreground color
pub fn toggle_with_color(on: &mut bool, color: egui::Color32) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on, Some(color))
}

/// Axis reference combo box - returns true if the value changed
pub fn axis_combo_box(
    ui: &mut egui::Ui,
    id_salt: &str,
    selected: &mut String,
    available: &[String],
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(if selected.is_empty() {
            "(none)"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for name in available {
                if ui.selectable_value(selected, name.clone(), name).changed() {
                    changed = true;
                }
            }
        });
    changed
}
