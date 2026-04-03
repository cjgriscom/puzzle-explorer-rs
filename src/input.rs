use crate::app::ThreeState;

#[derive(Default)]
pub struct CameraInputState {
    pub is_rotating_drag: bool,
    pub is_panning_drag: bool,
    pub drag_started_outside_ui: bool,
    pub last_mouse_pos: [f32; 2],
}

pub fn handle_camera_input(
    ctx: &egui::Context,
    three: &mut Option<ThreeState>,
    state: &mut CameraInputState,
) {
    let pointer_over_ui = ctx.is_pointer_over_egui();
    let multi_touch = ctx.input(|i| i.multi_touch());
    let any_touches = ctx.input(|i| i.any_touches());
    let using_multi_touch = !pointer_over_ui && multi_touch.is_some();

    // Touch input handling
    if !pointer_over_ui && any_touches {
        let pos = ctx.input(|i| i.pointer.interact_pos());
        let touch_pressed_now = ctx.input(|i| i.pointer.any_pressed());

        if using_multi_touch {
            if let Some(touch) = multi_touch
                && let Some(three) = three
            {
                let viewport = ctx.input(|i| i.content_rect().size());
                if touch.translation_delta != egui::Vec2::ZERO {
                    let dx = touch.translation_delta.x as f64 * 0.25;
                    let dy = touch.translation_delta.y as f64 * 0.25;
                    three.pan_drag(dx, dy, [viewport.x, viewport.y]);
                }
                let zoom_delta = touch.zoom_delta as f64;
                if (zoom_delta - 1.0).abs() > 1e-4 {
                    three.zoom_by_scale(zoom_delta, 0.4);
                }
            }
            // Store baseline to avoid jump when returning to single-touch rotate
            if let Some(pos) = pos {
                state.last_mouse_pos = [pos.x, pos.y];
            }
            state.is_rotating_drag = false;
            state.is_panning_drag = false;
            state.drag_started_outside_ui = false;
            return;
        }

        // Single-touch rotation: store baseline when a touch is newly pressed
        if let Some(pos) = pos {
            if state.is_rotating_drag && !touch_pressed_now {
                let dx = (pos.x - state.last_mouse_pos[0]) as f64 * 0.005;
                let dy = (pos.y - state.last_mouse_pos[1]) as f64 * 0.005;
                if let Some(three) = three.as_ref() {
                    three.rotate_drag(dx, dy);
                }
            }
            state.last_mouse_pos = [pos.x, pos.y];
            state.is_rotating_drag = true;
        }
        state.is_panning_drag = false;
        state.drag_started_outside_ui = false;
    } else {
        let primary_down = ctx.input(|i| i.pointer.primary_down());
        let middle_down = ctx.input(|i| i.pointer.middle_down());
        let any_down = primary_down || middle_down;
        let drag_started_now = ctx.input(|i| i.pointer.any_pressed());
        if drag_started_now {
            state.drag_started_outside_ui = !pointer_over_ui;
        }
        if !any_down {
            state.drag_started_outside_ui = false;
        }

        if !using_multi_touch && state.drag_started_outside_ui {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if primary_down && !middle_down {
                    if state.is_rotating_drag {
                        let dx = (pos.x - state.last_mouse_pos[0]) as f64 * 0.005;
                        let dy = (pos.y - state.last_mouse_pos[1]) as f64 * 0.005;
                        if let Some(three) = three.as_ref() {
                            three.rotate_drag(dx, dy);
                        }
                    }
                    state.is_rotating_drag = true;
                    state.is_panning_drag = false;
                    state.last_mouse_pos = [pos.x, pos.y];
                } else if middle_down && !primary_down {
                    if state.is_panning_drag {
                        let dx = (pos.x - state.last_mouse_pos[0]) as f64 * 0.25;
                        let dy = (pos.y - state.last_mouse_pos[1]) as f64 * 0.25;
                        if let Some(three) = three.as_mut() {
                            let viewport = ctx.input(|i| i.content_rect().size());
                            three.pan_drag(dx, dy, [viewport.x, viewport.y]);
                        }
                    }
                    state.is_panning_drag = true;
                    state.is_rotating_drag = false;
                    state.last_mouse_pos = [pos.x, pos.y];
                } else {
                    state.is_rotating_drag = false;
                    state.is_panning_drag = false;
                }
            }
        } else {
            state.is_rotating_drag = false;
            state.is_panning_drag = false;
        }

        if !using_multi_touch && !pointer_over_ui && !any_touches {
            let (is_scrolling, scroll_y) =
                ctx.input(|i| (i.is_scrolling(), i.smooth_scroll_delta.y));
            if is_scrolling
                && scroll_y != 0.0
                && let Some(three) = three.as_mut()
            {
                three.zoom(scroll_y as f64);
            }
        }
    }
}
