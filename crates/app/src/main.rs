use std::process;

use eframe::egui;

mod app;
mod headless;
mod node_graph;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let (console, log_level_state) = app::setup_tracing();

    tracing::info!("grapho starting");

    let args: Vec<String> = std::env::args().collect();
    match headless::maybe_run_headless(&args) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(err) => {
            eprintln!("headless error: {err}");
            process::exit(1);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "grapho",
        native_options,
        Box::new(|_cc| {
            let mut app = app::GraphoApp::new(console, log_level_state);
            app.try_load_default_graph();
            Ok(Box::new(app))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
