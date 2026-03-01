// --- App State ---

#[derive(Clone, Debug, PartialEq)]
pub struct PuzzleParams {
    pub n_a: u32,
    pub n_b: u32,
    pub p: u32,
    pub q: u32,
    pub colat_a: f32,
    pub colat_b: f32,
    pub lock_cuts: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrbitAnalysisState {
    pub annotate_pieces: bool,
    pub auto_update_orbits: bool,
    pub auto_update_groups: bool,
    pub orbits_stale: bool,
    pub groups_stale: bool,
    pub requested_groups_update: bool, // Manually requested update using button
}

impl Default for OrbitAnalysisState {
    fn default() -> Self {
        Self {
            annotate_pieces: true,
            auto_update_orbits: false,
            auto_update_groups: false,
            orbits_stale: false,
            groups_stale: false,
            requested_groups_update: false,
        }
    }
}

impl Default for PuzzleParams {
    fn default() -> Self {
        Self {
            n_a: 3,
            n_b: 2,
            p: 1,
            q: 3,
            colat_a: 119.4,
            colat_b: 119.4,
            lock_cuts: true,
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
