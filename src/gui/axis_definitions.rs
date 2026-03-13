use egui::Button;
use puzzle_explorer_math::geometry::derive_axis_angle;

use crate::app::PuzzleApp;
use crate::color::hex_to_color32;
use crate::gui::{
    AXIS_ANGLE_DECIMALS, AXIS_ANGLE_SPEED, AXIS_DEFINITIONS_POS, AXIS_DEFINITIONS_WIDTH,
    EULER_DECIMALS, EULER_SPEED, axis_combo_box,
};
use crate::types::{AxisDefinition, DerivedAxis, PuzzleParams};

impl DerivedAxis {
    pub const VARIANT_LABELS: &[&str] = &[
        "Vector",
        "Euler",
        "Copy",
        "Cross Product",
        "Average",
        "Cosine Rule",
        "Circular Pattern",
    ];
}

pub fn build_axis_definitions_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    let mut changed = false;

    // Handle delete confirmation dialog
    if let Some(name) = app.axis_defs.pending_delete.clone() {
        let dependents = app.axis_defs.find_dependents(&name);
        if dependents.is_empty() {
            app.axis_defs.delete(&name);
            app.axis_defs.pending_delete = None;
            changed = true;
        } else {
            let mut open = true;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    let others = dependents.len().saturating_sub(1);
                    let msg = if others > 0 {
                        format!(
                            "This definition is referenced by {} and {} other(s). Continue deleting?",
                            dependents[0], others
                        )
                    } else {
                        format!(
                            "This definition is referenced by {}. Continue deleting?",
                            dependents[0]
                        )
                    };
                    ui.label(&msg);
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            app.axis_defs.delete(&name);
                            app.axis_defs.pending_delete = None;
                            changed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            app.axis_defs.pending_delete = None;
                        }
                    });
                });
            if !open {
                app.axis_defs.pending_delete = None;
            }
        }
    }

    egui::Window::new("Axis Definitions")
        .default_pos(AXIS_DEFINITIONS_POS)
        .default_width(AXIS_DEFINITIONS_WIDTH)
        .show(ctx, |ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("Hide All").clicked() && !app.axis_defs.visible_axes.is_empty() {
                    app.axis_defs.visible_axes.clear();
                    changed = true;
                }

                if ui.button("Show All").clicked() {
                    for name in app.axis_defs.definitions_keys() {
                        if app.axis_defs.visible_axes.insert(name.clone()) {
                            changed = true;
                        }
                    }
                    for name in app.axis_defs.get_builtin_axis_names() {
                        if app.axis_defs.visible_axes.insert(name) {
                            changed = true;
                        }
                    }
                }

                use crate::color::{BUILTIN_X_COLOR, BUILTIN_Y_COLOR, BUILTIN_Z_COLOR};
                let x_color = hex_to_color32(BUILTIN_X_COLOR);
                let y_color = hex_to_color32(BUILTIN_Y_COLOR);
                let z_color = hex_to_color32(BUILTIN_Z_COLOR);
                let xyz_colors = [x_color, y_color, z_color];
                let names = ["X", "Y", "Z"];
                for (name, color) in names.iter().zip(xyz_colors.iter()) {
                    let included = app.axis_defs.visible_axes.contains(*name);
                    if ui
                        .add(
                            Button::new(egui::RichText::new(*name).color(*color))
                                .selected(included),
                        )
                        .clicked()
                    {
                        if included {
                            app.axis_defs.visible_axes.remove(*name);
                        } else {
                            app.axis_defs.visible_axes.insert(name.to_string());
                        }
                        changed = true;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ New Axis").clicked() {
                        let name = app.axis_defs.generate_name();
                        app.axis_defs.definitions.push(AxisDefinition {
                            name: name.clone(),
                            axis: DerivedAxis::default_for_variant(0),
                        });
                        app.axis_defs.visible_axes.insert(name);
                        changed = true;
                    }
                });
            });

            ui.separator();

            // Ordered keys for iteration
            let keys: Vec<String> = app.axis_defs.definitions_keys();
            let num_defs = keys.len();
            let available = app.axis_defs.available_axis_names();
            let mut swap_up: Option<usize> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, name) in keys.iter().enumerate() {
                    if i != 0 {
                        ui.separator();
                    }

                    let result = app.axis_defs.resolved.get(name);
                    let is_err = matches!(result, Some(Err(_)));
                    let err_text = match result {
                        Some(Err(e)) => e.clone(),
                        _ => String::new(),
                    };

                    // Build header with colored name
                    let header_color = if is_err {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(100, 255, 100)
                    };

                    let header_text = egui::RichText::new(name).color(header_color);

                    // Use a horizontal layout to put buttons on the right
                    let id = ui.make_persistent_id(format!("axis_def_{}", name));
                    let mut state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            id,
                            true,
                        );

                    // Get variant index before mutable borrow for the combo box
                    let current_variant_idx = app
                        .axis_defs
                        .get_definition(name)
                        .map(|a| a.variant_index())
                        .unwrap_or(0);

                    let _header_resp = ui.horizontal(|ui| {
                        state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);

                        let label_resp = ui.label(header_text);
                        if is_err {
                            label_resp.on_hover_text(&err_text);
                        }

                        // Push buttons to the right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(i + 1 < num_defs, Button::new("▼")).clicked() {
                                swap_up = Some(i + 1);
                            }
                            if ui.add_enabled(i > 0, Button::new("▲")).clicked() {
                                swap_up = Some(i);
                            }

                            if ui.add(Button::new("🗑")).clicked() {
                                app.axis_defs.pending_delete = Some(name.clone());
                            }

                            // Rename button
                            if ui.add(Button::new("✏")).clicked() {
                                app.axis_defs.rename_state = Some((name.clone(), name.clone()));
                            }

                            // Visible toggle
                            let is_visible = app.axis_defs.visible_axes.contains(name);
                            if ui
                                .add(Button::new("👁").selected(is_visible).frame(true))
                                .clicked()
                            {
                                if !is_visible {
                                    app.axis_defs.visible_axes.insert(name.clone());
                                } else {
                                    app.axis_defs.visible_axes.remove(name);
                                }
                                changed = true;
                            }

                            // Type dropdown
                            let mut new_variant_idx = current_variant_idx;
                            egui::ComboBox::from_id_salt(format!("type_{}", name))
                                .selected_text(DerivedAxis::VARIANT_LABELS[current_variant_idx])
                                .show_ui(ui, |ui| {
                                    for (i, label) in DerivedAxis::VARIANT_LABELS.iter().enumerate()
                                    {
                                        if ui
                                            .selectable_label(i == current_variant_idx, *label)
                                            .clicked()
                                            && i != current_variant_idx
                                        {
                                            new_variant_idx = i;
                                        }
                                    }
                                });
                            if new_variant_idx != current_variant_idx
                                && let Some(axis) = app.axis_defs.get_definition_mut(name)
                            {
                                *axis = DerivedAxis::default_for_variant(new_variant_idx);
                                changed = true;
                            }
                        });
                    });

                    // Rename inline editor
                    if let Some((ref rename_target, _)) = app.axis_defs.rename_state
                        && rename_target == name
                    {
                        let mut do_rename = false;
                        let mut cancel = false;
                        ui.horizontal(|ui| {
                            if let Some((_, ref mut buf)) = app.axis_defs.rename_state {
                                ui.text_edit_singleline(buf);
                                if ui.button("OK").clicked() {
                                    do_rename = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    cancel = true;
                                }
                            }
                        });
                        if do_rename
                            && let Some((old, new_desired)) = app.axis_defs.rename_state.take()
                        {
                            let new_trimmed = new_desired.trim().to_string();
                            if !new_trimmed.is_empty() && new_trimmed != old {
                                let new_name = app.axis_defs.make_unique_name(&new_trimmed);
                                app.axis_defs.do_rename(&old, &new_name);
                                // Bubble rename into puzzle params
                                let suffixes = app
                                    .axis_defs
                                    .get_definition(&new_name)
                                    .map(|a| a.output_suffixes())
                                    .unwrap_or_default();
                                rename_axis_in_puzzle_params(
                                    &mut app.params,
                                    &old,
                                    &new_name,
                                    &suffixes,
                                );
                                changed = true;
                            }
                        }
                        if cancel {
                            app.axis_defs.rename_state = None;
                        }
                    }

                    // Collapsing body
                    state.show_body_unindented(ui, |ui| {
                        let axis = match app.axis_defs.get_definition_mut(name) {
                            Some(a) => a,
                            None => return,
                        };

                        // Per-variant controls
                        match axis {
                            DerivedAxis::Vector { x, y, z } => {
                                ui.horizontal(|ui| {
                                    ui.label("x:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(x)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(y)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(z)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Euler { pitch, yaw } => {
                                ui.horizontal(|ui| {
                                    ui.label("Pitch:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(pitch)
                                                .range(0.0..=180.0)
                                                .speed(AXIS_ANGLE_SPEED)
                                                .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("Yaw:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(yaw)
                                                .range(-180.0..=180.0)
                                                .speed(AXIS_ANGLE_SPEED)
                                                .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Copy { axis, invert } => {
                                ui.horizontal(|ui| {
                                    ui.label("Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("copy_axis_{}", name),
                                        axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    if ui.checkbox(invert, "Invert").changed() {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::CrossProduct { a0, a1 } => {
                                ui.horizontal(|ui| {
                                    ui.label("A:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cross_a0_{}", name),
                                        a0,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("B:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cross_a1_{}", name),
                                        a1,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Average { axes } => {
                                let mut to_remove = None;
                                for (i, a) in axes.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        if axis_combo_box(
                                            ui,
                                            &format!("avg_{}_{}", name, i),
                                            a,
                                            &available,
                                        ) {
                                            changed = true;
                                        }
                                        if ui.small_button("🗑").clicked() {
                                            to_remove = Some(i);
                                        }
                                    });
                                }
                                if let Some(idx) = to_remove {
                                    axes.remove(idx);
                                    changed = true;
                                }
                                if ui.small_button("+ Add Axis").clicked() {
                                    axes.push("X".to_string());
                                    changed = true;
                                }
                            }
                            DerivedAxis::CosineRule {
                                p,
                                q,
                                n_a,
                                n_b,
                                a_axis,
                                perpendicular_axis,
                                manual_axis_angle_deg: manual_axis_angle,
                            } => {
                                // Manual axis angle toggle
                                ui.horizontal(|ui| {
                                    ui.label("Axis Angle:");
                                    if let Some(manual_axis_angle_deg) = manual_axis_angle {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add(
                                                    egui::DragValue::new(manual_axis_angle_deg)
                                                        .range(0.0..=180.0)
                                                        .speed(AXIS_ANGLE_SPEED)
                                                        .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                        .suffix("°"),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                    } else {
                                        // Show derived angle
                                        if let Some(ang) = derive_axis_angle(*n_a, *n_b, *p, *q) {
                                            ui.label(format!("{:.4}°", ang.to_degrees()));
                                        }
                                    }
                                    ui.separator();
                                    let mut is_manual_axis_angle = manual_axis_angle.is_some();
                                    if ui
                                        .add(crate::gui::toggle(&mut is_manual_axis_angle))
                                        .changed()
                                    {
                                        // When toggling on, populate from current p/q
                                        if is_manual_axis_angle {
                                            let ang = derive_axis_angle(*n_a, *n_b, *p, *q)
                                                .unwrap_or(0.0);
                                            *manual_axis_angle = Some(ang.to_degrees());
                                        } else {
                                            *manual_axis_angle = None;
                                        }
                                        changed = true;
                                    }
                                    ui.label("Manual Override");
                                });

                                ui.horizontal(|ui| {
                                    ui.label("nA:");
                                    if ui
                                        .add(egui::DragValue::new(n_a).range(2..=8).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("nB:");
                                    if ui
                                        .add(egui::DragValue::new(n_b).range(2..=8).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if manual_axis_angle.is_none() {
                                        ui.label("p:");
                                        if ui
                                            .add(egui::DragValue::new(p).range(1..=20).speed(0.02))
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        ui.label("q:");
                                        if ui
                                            .add(egui::DragValue::new(q).range(2..=30).speed(0.02))
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("A Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cos_a_{}", name),
                                        a_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("Perp Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cos_perp_{}", name),
                                        perpendicular_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::CircularPattern {
                                pattern_axis,
                                target_axis,
                                n,
                                angle_range_deg,
                                invert_range: invert,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.label("Pattern Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("circ_pat_{}", name),
                                        pattern_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("Target Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("circ_tgt_{}", name),
                                        target_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("n:");
                                    if ui
                                        .add(egui::DragValue::new(n).range(2..=100).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("Range:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(angle_range_deg)
                                                .range(0.0..=360.0)
                                                .speed(1.0)
                                                .fixed_decimals(1)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if ui.checkbox(invert, "Reverse Direction").changed() {
                                        changed = true;
                                    }
                                });
                            }
                        }
                    });
                }
            });

            if let Some(idx) = swap_up {
                app.axis_defs.definitions.swap(idx - 1, idx);
                changed = true;
            }
        });

    if changed {
        app.axis_defs.resolve_all();
        app.sync_n_match();
        app.spawn_geometry_worker();
        if let Some(three) = &app.three {
            let axes = app.build_axes();
            let def_vecs = app.axis_defs.get_visible_vectors();
            let builtin_axes = app.axis_defs.get_visible_builtin_axes();
            three.update_axis_indicators(&axes, app.params.show_axes, &def_vecs, &builtin_axes);
        }
    }
}

/// Rename an axis (and sub-indexed variants) in puzzle params
fn rename_axis_in_puzzle_params(
    params: &mut PuzzleParams,
    old_name: &str,
    new_name: &str,
    suffixes: &[String],
) {
    for entry in &mut params.axes {
        if entry.axis_name == old_name {
            entry.axis_name = new_name.to_string();
        }
        for suffix in suffixes {
            let old_sub = format!("{}_{}", old_name, suffix);
            if entry.axis_name == old_sub {
                entry.axis_name = format!("{}_{}", new_name, suffix);
            }
        }
    }
}
