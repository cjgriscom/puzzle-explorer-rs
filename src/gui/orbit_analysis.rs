use crate::app::PuzzleApp;
use crate::gap::GapManager;
use crate::gui::toggle;
use crate::gui::{ORBIT_ANALYSIS_POS, ORBIT_ANALYSIS_WIDTH};

pub fn build_orbit_analysis_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    let buttons_enabled = app.anim.is_none();

    // Orbit Analysis Window
    egui::Window::new("Orbit Analysis")
        .default_pos(ORBIT_ANALYSIS_POS)
        .default_width(ORBIT_ANALYSIS_WIDTH)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(toggle(&mut app.orbit_state.annotate_pieces))
                    .changed()
                {
                    app.set_face_group_visible(app.orbit_state.annotate_pieces);
                    
                }
                ui.label("Annotate pieces");
            });

            ui.horizontal(|ui| {
                if ui
                    .add(toggle(&mut app.orbit_state.number_pieces))
                    .changed()
                    && let Some(three) = &app.three
                    && let Some(orbit) = &app.orbit_result
                {
                    three.update_face_dots(orbit, app.orbit_state.number_pieces);
                }
                ui.label("Number pieces");
            });

            ui.horizontal(|ui| {
                if ui
                    .add(toggle(&mut app.orbit_state.auto_update_groups))
                    .changed()
                    && app.orbit_state.auto_update_groups
                    && app.orbit_result.is_some()
                {
                    app.orbit_state.groups_stale = true;
                    app.orbit_dreadnaut.clear();
                    app.spawn_orbit_worker();
                }
                ui.label("Compute groups");
            });

            ui.horizontal(|ui| {
                if ui
                    .add(toggle(&mut app.orbit_state.auto_update_orbits))
                    .changed()
                    && app.orbit_state.auto_update_orbits
                    && app.orbit_state.orbits_stale
                {
                    app.spawn_orbit_worker();
                }
                ui.label("Automatically update orbits");
            });

            ui.separator();

            let mut filter_changed = false;
            ui.horizontal(|ui| {
                if ui
                    .add(toggle(&mut app.orbit_state.fudged_mode))
                    .changed()
                {
                    filter_changed = true;
                }
                ui.label("Fudged Mode (experimental)");
            });

            if app.orbit_state.fudged_mode {
                ui.horizontal(|ui| {
                    ui.label("Min Piece Angle");
                    let changed = ui
                        .add(
                            egui::DragValue::new(&mut app.orbit_state.min_piece_angle_deg)
                                .range(0.1..=10.0)
                                .speed(0.02)
                                .suffix(" deg"),
                        )
                        .changed();
                    if changed {
                        filter_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Min Piece Perimeter");
                    let changed = ui
                        .add(
                            egui::DragValue::new(&mut app.orbit_state.min_piece_perimeter)
                                .range(0.0..=10.0)
                                .speed(0.001),
                        )
                        .changed();
                    if changed {
                        filter_changed = true;
                    }
                });
            }

            if filter_changed {
                app.orbit_state.orbits_stale = true;
                app.orbit_state.groups_stale = true;
                app.orbit_dreadnaut.clear();
                app.orbit_result = None;
                if let Some(three) = &app.three {
                    three.clear_face_dots();
                }
                if app.orbit_state.auto_update_orbits {
                    app.spawn_orbit_worker();
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        buttons_enabled
                            && (!app.orbit_state.auto_update_orbits
                                || app.orbit_state.orbits_stale),
                        egui::Button::new("Recompute Orbits"),
                    )
                    .clicked()
                {
                    app.spawn_orbit_worker();
                }
            });

            let err_msg = app.compute_output.borrow().clone();
            if err_msg.starts_with("Error:") && app.orbit_result.is_none() {
                ui.separator();
                ui.label(egui::RichText::new(&err_msg).color(egui::Color32::RED));
            }

            // Show orbit tree
            if let Some(orbit) = &app.orbit_result {
                ui.separator();
                egui::ScrollArea::vertical().vscroll(true).show(ui, |ui| {
                    let msg = app.compute_output.borrow().clone();
                    if msg.starts_with("Error:") {
                        ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                    } else {
                        ui.label(format!("Pieces: {}", orbit.face_count));
                    }
                    ui.label(format!("Total Orbits: {}", orbit.orbit_count));

                    let mut orbits_with_members: Vec<(usize, usize, Vec<usize>)> = (0..orbit
                        .orbit_count)
                        .map(|oi| {
                            (
                                oi,
                                0, // placeholder
                                orbit
                                    .face_orbit_indices
                                    .iter()
                                    .enumerate()
                                    .filter(|&(_, &foi)| match foi {
                                        Some(i) => i == oi,
                                        None => false,
                                    })
                                    .map(|(i, _)| i + 1)
                                    .collect::<Vec<usize>>(),
                            )
                        })
                        .filter(|(_, _, members)| members.len() > 1)
                        .collect();

                    // Give them an original color index based on the unsorted layout (skipping singletons)
                    (0..orbits_with_members.len()).for_each(|i| {
                        orbits_with_members[i].1 = i;
                    });

                    orbits_with_members
                        .sort_by_key(|(_, _, members)| -(members.len() as isize));
                    
                    for (oi, color_idx, members) in orbits_with_members {
                        let c = crate::color::ORBIT_COLORS
                            [color_idx % crate::color::ORBIT_COLORS.len()];
                        let rgb = c.1;
                        let color_name = c.0;

                        let header_text =
                            format!("     {}: {} pieces", color_name, members.len());

                        // Draw circle in header
                        let collapsing_resp = egui::CollapsingHeader::new(header_text)
                            .id_salt(format!("orbit_header_{}", oi))
                            .default_open(true)
                            .show(ui, |ui| {
                                // Show generator if number_pieces
                                if app.orbit_state.number_pieces
                                    && let Some(orbit) = &app.orbit_result
                                {
                                    ui.label(format!(
                                        "Generator: {}",
                                        match GapManager::reconstruct_generator_numbering_from_members(&orbit.generators[oi], &members) {
                                            Ok(renumbered) => GapManager::format_group_generator(true, &renumbered),
                                            Err(e) => e,
                                        }
                                    ));
                                }

                                if let Some(hash) = app.orbit_dreadnaut.get(&oi) {
                                    ui.label(format!("Canonical Label: {}", hash));
                                    match app.gap_cache.get(hash) {
                                        Some(None) => {
                                            ui.label("Structure: Computing...");
                                            ui.label("Permutations: Computing...");
                                        }
                                        Some(Some(cached)) => {
                                            ui.label(format!("Structure: {}", cached.structure));
                                            ui.label(format!("Permutations: {}", cached.size));
                                        }
                                        None => {
                                            ui.label("Structure: (not computed)");
                                            ui.label("Permutations: (not computed)");
                                        }
                                    }
                                } else {
                                    ui.label("Canonical Label: Computing...");
                                    ui.label("Structure: Computing...");
                                    ui.label("Permutations: Computing...");
                                }
                            });

                        // Draw circle on the header rect
                        let circle_center = collapsing_resp.header_response.rect.left_center()
                            + egui::vec2(24.0, 0.0);
                        ui.painter().circle_filled(
                            circle_center,
                            5.0,
                            egui::Color32::from_rgb(
                                (rgb[0] * 255.0) as u8,
                                (rgb[1] * 255.0) as u8,
                                (rgb[2] * 255.0) as u8,
                            ),
                        );
                    }
                });
            }
        });
}
