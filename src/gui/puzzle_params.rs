use crate::app::PuzzleApp;
use crate::gui::AxisEntry;
use crate::gui::{
    COLAT_DECIMALS, COLAT_SPEED, COLAT_STEP, MAX_COLAT, MAX_N, MIN_COLAT, MIN_N, PUZZLE_PARAMS_POS,
    PUZZLE_PARAMS_WIDTH,
};

pub fn build_puzzle_params_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    let buttons_enabled = app.anim.is_none();

    egui::Window::new("Puzzle Parameters")
        .default_pos(PUZZLE_PARAMS_POS)
        .default_width(PUZZLE_PARAMS_WIDTH)
        .show(ctx, |ui| {
            // Bigger slider than default
            ui.spacing_mut().slider_width = 250.0;

            let mut changed = false;

            // Show axes toggle
            ui.horizontal(|ui| {
                if ui
                    .add(crate::gui::toggle(&mut app.params.show_axes))
                    .changed()
                    && let Some(three) = &app.three
                {
                    let axes = app.build_axes();
                    let def_vecs = app.axis_defs.get_visible_vectors();
                    three.update_axis_indicators(&axes, app.params.show_axes, &def_vecs);
                }
                ui.label("Show axes");
            });

            // Max iterations
            ui.horizontal(|ui| {
                ui.label("Max Iterations:");
                if ui
                    .add(
                        egui::DragValue::new(&mut app.params.max_iterations)
                            .range(1..=150)
                            .speed(0.1),
                    )
                    .changed()
                {
                    changed = true;
                }
            });

            ui.separator();

            // Lock cuts toggle (slider style)
            ui.horizontal(|ui| {
                if ui
                    .add(crate::gui::toggle(&mut app.params.lock_cuts))
                    .changed()
                    && app.params.lock_cuts
                {
                    // Sync all colats to first axis
                    if let Some(first_colat) = app.params.axes.first().map(|a| a.colat) {
                        for entry in &mut app.params.axes {
                            entry.colat = first_colat;
                        }
                    }
                    changed = true;
                }
                ui.label("Lock cuts together");
            });

            ui.separator();

            // Get available axis names from axis definitions
            let available_names = app.axis_defs.available_axis_names();

            // Per-axis entries
            let axis_labels: Vec<char> = ('A'..='Z').collect();
            let num_axes = app.params.axes.len();
            let mut to_remove: Option<usize> = None;

            for idx in 0..num_axes {
                let label = axis_labels.get(idx).copied().unwrap_or('?');

                // Check if axis name is valid (exists and resolves OK)
                let axis_name = app.params.axes[idx].axis_name.clone();
                let name_ok = is_axis_ok(&axis_name, &app.axis_defs);

                // Check if this axis references a WillsEquation definition
                let wills_n = app.axis_defs.get_wills_n_for_axis(&axis_name);

                // If n_match is on, sync n from the definition
                if app.params.axes[idx].n_match {
                    if let Some(matched_n) = wills_n {
                        if app.params.axes[idx].n != matched_n {
                            app.params.axes[idx].n = matched_n;
                            changed = true;
                        }
                    } else {
                        // No longer a WillsEquation reference, turn off n_match
                        app.params.axes[idx].n_match = false;
                    }
                }

                ui.horizontal(|ui| {
                    // Enabled toggle
                    if ui
                        .add(crate::gui::toggle_with_color(
                            &mut app.params.axes[idx].enabled,
                            crate::color::color32(&crate::color::axis_color(idx)),
                        ))
                        .changed()
                    {
                        changed = true;
                    }

                    // Axis label
                    ui.label(format!("n{}:", label));

                    // n (fold symmetry) combo - with "Match" option for WillsEquation axes
                    let n_display = if app.params.axes[idx].n_match
                        && let Some(matched_n) = wills_n
                    {
                        format!("Match ({})", matched_n)
                    } else {
                        format!("{}", app.params.axes[idx].n)
                    };
                    egui::ComboBox::from_id_salt(format!("n_{}", idx))
                        .selected_text(&n_display)
                        .show_ui(ui, |ui| {
                            // "Match" option (only for WillsEquation axes)
                            if let Some(matched_n) = wills_n {
                                let mut match_val = app.params.axes[idx].n_match;
                                if ui
                                    .selectable_label(match_val, format!("Match ({})", matched_n))
                                    .clicked()
                                {
                                    match_val = !match_val;
                                    app.params.axes[idx].n_match = match_val;
                                    if match_val {
                                        app.params.axes[idx].n = matched_n;
                                    }
                                    changed = true;
                                }
                            }
                            for i in MIN_N..=MAX_N {
                                if ui
                                    .selectable_value(
                                        &mut app.params.axes[idx].n,
                                        i,
                                        format!("{}", i),
                                    )
                                    .changed()
                                {
                                    app.params.axes[idx].n_match = false;
                                    changed = true;
                                }
                            }
                        });

                    // Axis name dropdown (colored)
                    let name_color = if axis_name.is_empty() {
                        egui::Color32::GRAY
                    } else if name_ok {
                        ui.visuals().text_color()
                    } else {
                        egui::Color32::from_rgb(255, 80, 80)
                    };
                    let display_label = if axis_name.is_empty() {
                        "(none)".to_string()
                    } else {
                        axis_name.clone()
                    };
                    let display_text = egui::RichText::new(&display_label).color(name_color);

                    let prev_name = app.params.axes[idx].axis_name.clone();
                    egui::ComboBox::from_id_salt(format!("axis_name_{}", idx))
                        .selected_text(display_text)
                        .show_ui(ui, |ui| {
                            // Blank option
                            if ui
                                .selectable_value(
                                    &mut app.params.axes[idx].axis_name,
                                    String::new(),
                                    "(none)",
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            for name in &available_names {
                                if ui
                                    .selectable_value(
                                        &mut app.params.axes[idx].axis_name,
                                        name.clone(),
                                        name,
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                    // Auto-enable Match when selecting a WillsEquation axis
                    if app.params.axes[idx].axis_name != prev_name {
                        if let Some(matched_n) = app
                            .axis_defs
                            .get_wills_n_for_axis(&app.params.axes[idx].axis_name)
                        {
                            app.params.axes[idx].n_match = true;
                            app.params.axes[idx].n = matched_n;
                            changed = true;
                        } else {
                            app.params.axes[idx].n_match = false;
                        }
                    }

                    // Per-frame sync for n_match
                    if app.params.axes[idx].n_match
                        && let Some(matched_n) = app
                            .axis_defs
                            .get_wills_n_for_axis(&app.params.axes[idx].axis_name)
                        && app.params.axes[idx].n != matched_n
                    {
                        app.params.axes[idx].n = matched_n;
                        changed = true;
                    }

                    // Delete button (disabled if only 1 axis)
                    ui.add_enabled_ui(num_axes > 1, |ui| {
                        if ui.small_button("🗑").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                });

                // Colat slider
                ui.add_enabled_ui(!app.params.lock_cuts || idx == 0, |ui| {
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut app.params.axes[idx].colat,
                                MIN_COLAT..=MAX_COLAT,
                            )
                            .smallest_positive(COLAT_STEP)
                            .fixed_decimals(COLAT_DECIMALS)
                            .step_by(COLAT_STEP)
                            .drag_value_speed(COLAT_SPEED)
                            .show_value(true)
                            .trailing_fill(true),
                        )
                        .changed()
                    {
                        if app.params.lock_cuts {
                            // Sync all colats from this slider
                            let new_colat = app.params.axes[idx].colat;
                            for entry in &mut app.params.axes {
                                entry.colat = new_colat;
                            }
                        }
                        changed = true;
                    }
                });

                ui.separator();
            }

            // Handle removal
            if let Some(idx) = to_remove {
                app.params.axes.remove(idx);
                changed = true;
            }

            // Add axis button
            if ui.button("+ Add Axis").clicked() {
                let mut new_entry = AxisEntry::default();
                if app.params.lock_cuts
                    && let Some(first_colat) = app.params.axes.first().map(|a| a.colat)
                {
                    new_entry.colat = first_colat;
                }
                app.params.axes.push(new_entry);
                changed = true;
            }

            if changed {
                app.spawn_geometry_worker();
            }

            ui.separator();

            // Rotation buttons
            ui.add_enabled_ui(buttons_enabled, |ui| {
                let axis_count = app.params.axes.len();
                ui.horizontal(|ui| {
                    for idx in 0..axis_count.min(3) {
                        let label = axis_labels.get(idx).copied().unwrap_or('?');
                        if ui.button(format!("Rotate {}", label)).clicked() {
                            app.start_rotation(idx, true);
                        }
                        if ui.button(format!("{}'", label)).clicked() {
                            app.start_rotation(idx, false);
                        }
                    }
                });
            });
        });
}

/// Check if an axis name resolves to a valid (single) vector.
fn is_axis_ok(name: &str, axis_defs: &crate::gui::axis_definitions::AxisDefinitions) -> bool {
    match name {
        "" => false,
        "X" | "Y" | "Z" => true,
        _ => axis_defs.get_resolved_vector(name).is_some(),
    }
}
