pub mod axis_definitions;
pub mod controls;
pub mod gap_console;
pub mod orbit_analysis;
pub mod puzzle_params;

// --- Constants ---
pub const AXIS_DEFINITIONS_POS: [f32; 2] = [20.0, 20.0];
pub const PUZZLE_PARAMS_POS: [f32; 2] = [20.0, 220.0];
pub const ORBIT_ANALYSIS_POS: [f32; 2] = [370.0, 20.0];
pub const CONTROLS_POS: [f32; 2] = [720.0, 70.0];
pub const GAP_CONSOLE_POS: [f32; 2] = [720.0, 20.0];

pub const AXIS_DEFINITIONS_WIDTH: f32 = 320.0;
pub const PUZZLE_PARAMS_WIDTH: f32 = 320.0;
pub const ORBIT_ANALYSIS_WIDTH: f32 = 320.0;
pub const GAP_CONSOLE_WIDTH: f32 = 500.0;

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

// --- App State ---

#[derive(Clone, Debug, PartialEq)]
pub struct AxisEntry {
    pub axis_name: String, // references an axis definition (or X/Y/Z)
    pub n: u32,
    pub colat: f32,
    pub n_match: bool, // when true, n syncs from WillsEquation definition
    pub enabled: bool, // when false, axis is skipped during geometry build
}

impl Default for AxisEntry {
    fn default() -> Self {
        Self {
            axis_name: String::new(),
            n: 3,
            colat: 109.5,
            n_match: false,
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PuzzleParams {
    pub max_iterations: u32,
    pub lock_cuts: bool,
    pub show_axes: bool,
    pub axes: Vec<AxisEntry>,
}

impl Default for PuzzleParams {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            lock_cuts: true,
            show_axes: true,
            axes: vec![
                AxisEntry {
                    axis_name: "Trapentrix_A".to_string(),
                    n: 3,
                    colat: 109.5,
                    n_match: true,
                    enabled: true,
                },
                AxisEntry {
                    axis_name: "Trapentrix_B".to_string(),
                    n: 3,
                    colat: 109.5,
                    n_match: true,
                    enabled: true,
                },
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrbitAnalysisState {
    pub annotate_pieces: bool,
    pub number_pieces: bool,
    pub fudged_mode: bool,
    pub min_piece_angle_deg: f32,
    pub min_piece_perimeter: f64,
    pub auto_update_orbits: bool,
    pub auto_update_groups: bool,
    pub orbits_stale: bool,
    pub groups_stale: bool,
}

impl Default for OrbitAnalysisState {
    fn default() -> Self {
        Self {
            annotate_pieces: true,
            number_pieces: false,
            fudged_mode: false,
            min_piece_angle_deg: 5.0,
            min_piece_perimeter: 0.02,
            auto_update_orbits: false,
            auto_update_groups: false,
            orbits_stale: false,
            groups_stale: false,
        }
    }
}

pub fn toggle_ui(
    ui: &mut egui::Ui,
    on: &mut bool,
    mut color: Option<egui::Color32>,
) -> egui::Response {
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

pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on, None)
}

pub fn toggle_with_color(on: &mut bool, color: egui::Color32) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on, Some(color))
}
