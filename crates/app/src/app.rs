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
mod spreadsheet;
mod ui;
mod undo;
mod viewport;

pub(crate) use logging::ConsoleBuffer;
pub(crate) use logging::setup_tracing;

use logging::level_filter_to_u8;
use node_info::NodeInfoPanel;
use undo::{UndoSnapshot, UndoStack};

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
    last_display_state: DisplayState,
    last_node_graph_rect: Option<egui::Rect>,
    last_selected_node: Option<grapho_core::NodeId>,
    info_panel: Option<NodeInfoPanel>,
    undo_stack: UndoStack,
    pending_undo: Option<UndoSnapshot>,
    spreadsheet_domain: grapho_core::AttributeDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayState {
    Ok,
    Missing,
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
            last_display_state: DisplayState::Ok,
            last_node_graph_rect: None,
            last_selected_node: None,
            info_panel: None,
            undo_stack: UndoStack::new(),
            pending_undo: None,
            spreadsheet_domain: grapho_core::AttributeDomain::Point,
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

    fn snapshot_undo(&self) -> UndoSnapshot {
        self.undo_stack
            .snapshot(&self.project.graph, &self.node_graph)
    }

    fn queue_undo_snapshot(&mut self, snapshot: UndoSnapshot, pointer_down: bool) {
        if pointer_down {
            if self.pending_undo.is_none() {
                self.pending_undo = Some(snapshot);
            }
        } else {
            self.undo_stack.push(snapshot);
        }
    }

    fn flush_pending_undo(&mut self) {
        if let Some(snapshot) = self.pending_undo.take() {
            self.undo_stack.push(snapshot);
        }
    }

    fn restore_snapshot(&mut self, snapshot: UndoSnapshot) {
        self.project.graph = snapshot.graph;
        self.node_graph
            .restore_layout(&self.project.graph, &snapshot.layout);
        self.last_selected_node = snapshot.layout.selected;
        self.pending_scene = None;
        self.last_eval_report = None;
        self.eval_dirty = true;
        self.last_param_change = None;
        self.info_panel = None;
    }

    fn try_undo(&mut self) {
        self.pending_undo = None;
        let current = self.snapshot_undo();
        if let Some(snapshot) = self.undo_stack.undo(current) {
            self.restore_snapshot(snapshot);
        }
    }

    fn try_redo(&mut self) {
        self.pending_undo = None;
        let current = self.snapshot_undo();
        if let Some(snapshot) = self.undo_stack.redo(current) {
            self.restore_snapshot(snapshot);
        }
    }
}
