pub mod axis_definitions;
pub mod controls;
pub mod gap_console;
pub mod orbit_analysis;
pub mod puzzle_params;

// --- App State ---

#[derive(Clone, Debug, PartialEq)]
pub struct ExtraAxisParams {
    pub pitch_deg: f64,
    pub yaw_deg: f64,
    pub colat: f32,
    pub n: u32,
}

impl Default for ExtraAxisParams {
    fn default() -> Self {
        Self {
            pitch_deg: 90.0,
            yaw_deg: 0.0,
            colat: 109.5,
            n: 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PuzzleParams {
    pub n_a: u32,
    pub n_b: u32,
    pub p: u32,
    pub q: u32,
    pub manual_axis_angle: bool,
    pub manual_axis_angle_deg: f64,
    pub manual_max_iterations: u32,
    pub colat_a: f32,
    pub colat_b: f32,
    pub lock_cuts: bool,
    pub show_axes: bool,
    // Additional axes (experimental)
    pub num_extra_axes: u32,
    pub extra_axes: Vec<ExtraAxisParams>,
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

impl Default for PuzzleParams {
    fn default() -> Self {
        Self {
            n_a: 3,
            n_b: 3,
            p: 1,
            q: 5,
            manual_axis_angle: false,
            manual_axis_angle_deg: 119.87,
            manual_max_iterations: 15,
            colat_a: 109.5,
            colat_b: 109.5,
            lock_cuts: true,
            show_axes: false,
            num_extra_axes: 0,
            extra_axes: Vec::new(),
        }
    }
}

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.1);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}
