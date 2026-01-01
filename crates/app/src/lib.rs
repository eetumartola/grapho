#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports, non_snake_case))]

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod node_graph;

#[cfg(target_arch = "wasm32")]
use app::GraphoApp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    let _ = console_error_panic_hook::set_once();
    let runner = eframe::WebRunner::new();
    let web_options = eframe::WebOptions::default();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?;
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into()
        .map_err(|_| JsValue::from_str("canvas is not HtmlCanvasElement"))?;
    runner
        .start(canvas, web_options, Box::new(|_cc| {
            let (console, log_level_state) = app::setup_tracing();
            let mut app = GraphoApp::new(console, log_level_state);
            app.try_load_default_graph();
            Ok(Box::new(app))
        }))
        .await
}
