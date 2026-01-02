use std::path::PathBuf;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use grapho_core::{MeshEvalState, Project};
use eframe::egui;
use render::{RenderScene, ViewportRenderer};
use tracing_subscriber::filter::LevelFilter;

use crate::node_graph;

mod eval;
mod io;
mod logging;
mod node_info;
mod ui;
mod viewport;

pub(crate) use logging::ConsoleBuffer;
pub(crate) use logging::setup_tracing;

use logging::level_filter_to_u8;
use node_info::NodeInfoPanel;

pub(crate) struct GraphoApp {
    project: Project,
    project_path: Option<PathBuf>,
    console: ConsoleBuffer,
    log_level: LevelFilter,
    log_level_state: Arc<AtomicU8>,
    viewport_renderer: Option<ViewportRenderer>,
    pending_scene: Option<RenderScene>,
    eval_state: MeshEvalState,
    last_eval_report: Option<grapho_core::EvalReport>,
    last_eval_ms: Option<f32>,
    eval_dirty: bool,
    last_param_change: Option<Instant>,
    node_graph: node_graph::NodeGraphState,
    last_output_state: OutputState,
    last_node_graph_rect: Option<egui::Rect>,
    last_selected_node: Option<grapho_core::NodeId>,
    info_panel: Option<NodeInfoPanel>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputState {
    Ok,
    Missing,
    Multiple,
}

impl GraphoApp {
    pub(crate) fn new(console: ConsoleBuffer, log_level_state: Arc<AtomicU8>) -> Self {
        Self {
            project: Project::default(),
            project_path: None,
            console,
            log_level: LevelFilter::INFO,
            log_level_state,
            viewport_renderer: None,
            pending_scene: None,
            eval_state: MeshEvalState::new(),
            last_eval_report: None,
            last_eval_ms: None,
            eval_dirty: false,
            last_param_change: None,
            node_graph: node_graph::NodeGraphState::default(),
            last_output_state: OutputState::Ok,
            last_node_graph_rect: None,
            last_selected_node: None,
            info_panel: None,
        }
    }

    fn set_log_level(&mut self, new_level: LevelFilter) {
        if new_level == self.log_level {
            return;
        }

        self.log_level_state
            .store(level_filter_to_u8(new_level), std::sync::atomic::Ordering::Relaxed);
        self.log_level = new_level;
    }
}
