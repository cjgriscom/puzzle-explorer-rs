//! Rust egui webapp with puzzle geometry analysis toolsuite

mod app;
mod color;
mod dreadnaut;
mod geometry;
mod gui;
mod puzzle;
mod three;
mod worker;

pub use app::PuzzleApp;
pub use worker::worker_handle_msg;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// Entry point for the web application.
///
/// Initializes the Egui app and binds it to the specified canvas ID.
#[wasm_bindgen]
pub async fn run_app(egui_canvas_id: String, three_canvas_id: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    let document = web_sys::window()
        .ok_or(JsValue::from_str("No window"))?
        .document()
        .ok_or(JsValue::from_str("No document"))?;
    let canvas = document
        .get_element_by_id(&egui_canvas_id)
        .ok_or(JsValue::from_str("No canvas found"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

    let web_options = eframe::WebOptions::default();

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(PuzzleApp::new(cc, three_canvas_id)))),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to start eframe: {:?}", e)))?;

    Ok(())
}
