//! Rust egui webapp with puzzle geometry analysis toolsuite

mod app;
mod color;
mod dreadnaut;
mod gap;
mod geometry;
mod gui;
mod puzzle;
mod three;
mod time;
mod worker;

pub use app::PuzzleApp;
pub use worker::worker_handle_msg;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlCanvasElement};

/// Entry point for the web application.
///
/// Initializes the Egui app and binds it to the specified canvas ID.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn run_app(
    egui_canvas_id: String,
    three_canvas_id: String,
    build_hash: String,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    let document = web_sys::window()
        .ok_or(JsValue::from_str("No window"))?
        .document()
        .ok_or(JsValue::from_str("No document"))?;

    let egui_canvas = document
        .get_element_by_id(&egui_canvas_id)
        .ok_or(JsValue::from_str("No canvas found"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

    let three_canvas = document
        .get_element_by_id(&three_canvas_id)
        .ok_or(JsValue::from_str("No three canvas found"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

    // Prevent drag thumbnail from appearing when dragging the mouse over the canvas
    for canvas in [&egui_canvas, &three_canvas] {
        let _ = canvas.set_attribute("draggable", "false");
        let on_drag_start = Closure::wrap(Box::new(|e: Event| {
            e.prevent_default();
        }) as Box<dyn FnMut(_)>);
        let _ = canvas
            .add_event_listener_with_callback("dragstart", on_drag_start.as_ref().unchecked_ref());
        on_drag_start.forget();
    }

    let web_options = eframe::WebOptions::default();

    eframe::WebRunner::new()
        .start(
            egui_canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(PuzzleApp::new(cc, three_canvas_id, build_hash)))),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to start eframe: {:?}", e)))?;

    Ok(())
}
