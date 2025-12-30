use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core::{evaluate_mesh_graph, MeshEvalState, Project, SceneSnapshot, ShadingMode};
use eframe::egui;
use render::{
    CameraState, RenderMesh, RenderScene, ViewportDebug, ViewportRenderer, ViewportShadingMode,
};
use rfd::FileDialog;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::node_graph;

const MAX_LOG_LINES: usize = 500;
const DEFAULT_GRAPH_PATH: &str = "graphs/default.json";

#[derive(Clone)]
pub(crate) struct ConsoleBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
}

impl ConsoleBuffer {
    fn new() -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn push_line(&self, line: String) {
        let mut lines = self.lines.lock().expect("console buffer lock");
        lines.push_back(line);
        while lines.len() > MAX_LOG_LINES {
            lines.pop_front();
        }
    }

    fn snapshot(&self) -> Vec<String> {
        let lines = self.lines.lock().expect("console buffer lock");
        lines.iter().cloned().collect()
    }
}

struct ConsoleMakeWriter {
    buffer: ConsoleBuffer,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ConsoleMakeWriter {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ConsoleWriter {
            buffer: self.buffer.clone(),
        }
    }
}

struct ConsoleWriter {
    buffer: ConsoleBuffer,
}

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        for line in text.lines() {
            self.buffer.push_line(line.to_string());
        }

        let _ = io::stdout().write_all(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = io::stdout().flush();
        Ok(())
    }
}

pub(crate) struct GraphoApp {
    project: Project,
    project_path: Option<PathBuf>,
    console: ConsoleBuffer,
    log_level: LevelFilter,
    log_level_state: Arc<AtomicU8>,
    viewport_renderer: Option<ViewportRenderer>,
    pending_scene: Option<RenderScene>,
    eval_state: MeshEvalState,
    last_eval_report: Option<core::EvalReport>,
    last_eval_ms: Option<f32>,
    eval_dirty: bool,
    last_param_change: Option<Instant>,
    node_graph: node_graph::NodeGraphState,
    last_output_state: OutputState,
    last_node_graph_rect: Option<egui::Rect>,
    last_selected_node: Option<core::NodeId>,
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
        }
    }

    fn new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
        self.node_graph.reset();
        self.eval_dirty = true;
        self.pending_scene = None;
        tracing::info!("new project created");
    }

    fn save_project_to(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(&self.project).map_err(io::Error::other)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    fn load_project_from(&mut self, path: &Path) -> io::Result<()> {
        let data = std::fs::read(path)?;
        let project: Project = serde_json::from_slice(&data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        self.project = project;
        self.project_path = Some(path.to_path_buf());
        self.node_graph.reset();
        self.eval_dirty = true;
        self.pending_scene = None;
        Ok(())
    }

    fn set_log_level(&mut self, new_level: LevelFilter) {
        if new_level == self.log_level {
            return;
        }

        self.log_level_state
            .store(level_filter_to_u8(new_level), Ordering::Relaxed);
        self.log_level = new_level;
    }

    pub(crate) fn try_load_default_graph(&mut self) {
        let path = Path::new(DEFAULT_GRAPH_PATH);
        if !path.exists() {
            return;
        }

        match self.load_project_from(path) {
            Ok(()) => {
                tracing::info!("default graph loaded");
            }
            Err(err) => {
                tracing::error!("failed to load default graph: {}", err);
            }
        }
    }
}

impl eframe::App for GraphoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.sync_wgpu_renderer(frame);
        let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
        if tab_pressed {
            let hover_pos = ctx.input(|i| i.pointer.hover_pos());
            if let (Some(rect), Some(pos)) = (self.last_node_graph_rect, hover_pos) {
                if rect.contains(pos) && !ctx.wants_keyboard_input() {
                    ctx.input_mut(|i| {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                    });
                    self.node_graph.open_add_menu(pos);
                }
            }
        }
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close();
                    }

                    if ui.button("Open...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Grapho Project", &["json"])
                            .pick_file()
                        {
                            match self.load_project_from(&path) {
                                Ok(()) => {
                                    self.project_path = Some(path);
                                    tracing::info!("project loaded");
                                }
                                Err(err) => {
                                    tracing::error!("failed to load project: {}", err);
                                }
                            }
                        }
                        ui.close();
                    }

                    if ui.button("Save").clicked() {
                        if let Some(path) = self.project_path.clone() {
                            if let Err(err) = self.save_project_to(&path) {
                                tracing::error!("failed to save project: {}", err);
                            } else {
                                tracing::info!("project saved");
                            }
                        } else {
                            tracing::warn!("no project path set; use Save As");
                        }
                        ui.close();
                    }

                    if ui.button("Save As...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Grapho Project", &["json"])
                            .set_file_name("project.json")
                            .save_file()
                        {
                            match self.save_project_to(&path) {
                                Ok(()) => {
                                    self.project_path = Some(path);
                                    tracing::info!("project saved");
                                }
                                Err(err) => {
                                    tracing::error!("failed to save project: {}", err);
                                }
                            }
                        }
                        ui.close();
                    }
                });

                ui.separator();
                ui.label("grapho");
                ui.separator();
                ui.checkbox(
                    &mut self.project.settings.panels.show_inspector,
                    "Parameters",
                );
                ui.checkbox(&mut self.project.settings.panels.show_debug, "Debug");
                ui.checkbox(&mut self.project.settings.panels.show_console, "Console");
            });
        });

        if self.project.settings.panels.show_debug || self.project.settings.panels.show_console {
            egui::SidePanel::right("side_panels")
                .resizable(true)
                .default_width(280.0)
                .show(ctx, |ui| {
                    if self.project.settings.panels.show_debug {
                        egui::CollapsingHeader::new("Debug")
                            .default_open(true)
                            .show(ui, |ui| {
                                let ratio_range = 0.2..=0.8;
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.project.settings.viewport_split,
                                        ratio_range,
                                    )
                                    .text("Viewport split")
                                    .custom_formatter(|value, _| format!("{:.0}%", value * 100.0)),
                                );

                                ui.separator();
                                ui.label("Viewport overlays");
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.show_grid,
                                    "Grid",
                                );
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.show_axes,
                                    "Axes",
                                );
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.show_normals,
                                    "Normals",
                                );
                                if self.project.settings.render_debug.show_normals {
                                    ui.horizontal(|ui| {
                                        ui.label("Normal length");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self
                                                    .project
                                                    .settings
                                                    .render_debug
                                                    .normal_length,
                                            )
                                            .speed(0.02)
                                            .range(0.01..=10.0),
                                        );
                                    });
                                }
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.show_bounds,
                                    "Bounds",
                                );
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.show_stats,
                                    "Stats overlay",
                                );

                                ui.separator();
                                ui.label("Shading");
                                let shading = &mut self.project.settings.render_debug.shading_mode;
                                egui::ComboBox::from_label("Mode")
                                    .selected_text(match shading {
                                        ShadingMode::Lit => "Lit",
                                        ShadingMode::Normals => "Normals",
                                        ShadingMode::Depth => "Depth",
                                    })
                                    .show_ui(ui, |ui| {
                                        for (mode, label) in [
                                            (ShadingMode::Lit, "Lit"),
                                            (ShadingMode::Normals, "Normals"),
                                            (ShadingMode::Depth, "Depth"),
                                        ] {
                                            if ui
                                                .selectable_label(*shading == mode, label)
                                                .clicked()
                                            {
                                                *shading = mode;
                                            }
                                        }
                                    });

                                if *shading == ShadingMode::Depth {
                                    ui.horizontal(|ui| {
                                        ui.label("Near");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.project.settings.render_debug.depth_near,
                                            )
                                            .speed(0.1)
                                            .range(0.01..=1000.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Far");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.project.settings.render_debug.depth_far,
                                            )
                                            .speed(0.1)
                                            .range(0.01..=5000.0),
                                        );
                                    });
                                    let near = self.project.settings.render_debug.depth_near;
                                    let far = self.project.settings.render_debug.depth_far;
                                    if far <= near + 0.01 {
                                        self.project.settings.render_debug.depth_far = near + 0.01;
                                    }
                                }

                                ui.separator();
                                ui.label("Evaluation");
                                if ui.button("Create demo graph").clicked() {
                                    self.node_graph.add_demo_graph(&mut self.project.graph);
                                    self.mark_eval_dirty();
                                }
                                if ui.button("Recompute now").clicked() {
                                    self.eval_dirty = false;
                                    self.last_param_change = None;
                                    self.evaluate_graph();
                                }

                                if let Some(report) = &self.last_eval_report {
                                    let computed = report.computed.len();
                                    ui.label(format!(
                                        "Computed: {}  Cache hits: {}  Misses: {}",
                                        computed, report.cache_hits, report.cache_misses
                                    ));
                                    if let Some(ms) = self.last_eval_ms {
                                        ui.label(format!("Last eval: {:.2} ms", ms));
                                    }
                                    if !report.output_valid {
                                        ui.colored_label(egui::Color32::RED, "Output invalid");
                                    }
                                    let mut nodes: Vec<_> = report.node_reports.values().collect();
                                    nodes.sort_by(|a, b| {
                                        b.duration_ms
                                            .partial_cmp(&a.duration_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    for entry in nodes.into_iter().take(5) {
                                        ui.label(format!(
                                            "{:?}: {:.2} ms{}",
                                            entry.node,
                                            entry.duration_ms,
                                            if entry.cache_hit { " (cache)" } else { "" }
                                        ));
                                    }
                                }

                                egui::ComboBox::from_label("Log level")
                                    .selected_text(format!("{:?}", self.log_level))
                                    .show_ui(ui, |ui| {
                                        for level in [
                                            LevelFilter::ERROR,
                                            LevelFilter::WARN,
                                            LevelFilter::INFO,
                                            LevelFilter::DEBUG,
                                            LevelFilter::TRACE,
                                        ] {
                                            if ui
                                                .selectable_label(
                                                    self.log_level == level,
                                                    format!("{:?}", level),
                                                )
                                                .clicked()
                                            {
                                                self.set_log_level(level);
                                            }
                                        }
                                    });
                            });
                    }

                    if self.project.settings.panels.show_console {
                        egui::CollapsingHeader::new("Console")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for line in self.console.snapshot() {
                                            ui.label(line);
                                        }
                                    });
                            });
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let split_ratio = self.project.settings.viewport_split.clamp(0.2, 0.8);
            let split_x = rect.min.x + rect.width() * split_ratio;
            let left_rect = egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.max.y));
            let right_rect = egui::Rect::from_min_max(egui::pos2(split_x, rect.min.y), rect.max);

            ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                let available = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available, egui::Sense::click_and_drag());
                self.handle_viewport_input(&response);
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 28, 28));
                if let Some(renderer) = &self.viewport_renderer {
                    let camera = CameraState {
                        target: self.project.settings.camera.target,
                        distance: self.project.settings.camera.distance,
                        yaw: self.project.settings.camera.yaw,
                        pitch: self.project.settings.camera.pitch,
                    };
                    let shading_mode = match self.project.settings.render_debug.shading_mode {
                        ShadingMode::Lit => ViewportShadingMode::Lit,
                        ShadingMode::Normals => ViewportShadingMode::Normals,
                        ShadingMode::Depth => ViewportShadingMode::Depth,
                    };
                    let debug = ViewportDebug {
                        show_grid: self.project.settings.render_debug.show_grid,
                        show_axes: self.project.settings.render_debug.show_axes,
                        show_normals: self.project.settings.render_debug.show_normals,
                        show_bounds: self.project.settings.render_debug.show_bounds,
                        normal_length: self.project.settings.render_debug.normal_length,
                        shading_mode,
                        depth_near: self.project.settings.render_debug.depth_near,
                        depth_far: self.project.settings.render_debug.depth_far,
                    };
                    let callback = renderer.paint_callback(rect, camera, debug);
                    ui.painter().add(egui::Shape::Callback(callback));

                    if self.project.settings.render_debug.show_stats {
                        let stats = renderer.stats_snapshot();
                        let text = format!(
                            "FPS: {:.1}\nFrame: {:.2} ms\nVerts: {}\nTris: {}\nMeshes: {}\nCache: {} hits / {} misses / {} uploads",
                            stats.fps,
                            stats.frame_time_ms,
                            stats.vertex_count,
                            stats.triangle_count,
                            stats.mesh_count,
                            stats.cache_hits,
                            stats.cache_misses,
                            stats.cache_uploads
                        );
                        let font_id = egui::FontId::monospace(12.0);
                        let galley = ui.fonts_mut(|f| {
                            f.layout_no_wrap(text.clone(), font_id.clone(), egui::Color32::WHITE)
                        });
                        let padding = egui::vec2(6.0, 4.0);
                        let bg_rect = egui::Rect::from_min_size(
                            rect.min + egui::vec2(8.0, 8.0),
                            galley.size() + padding * 2.0,
                        );
                        let painter = ui.painter();
                        painter.rect_filled(bg_rect, 4.0, egui::Color32::from_black_alpha(160));
                        painter.galley(
                            bg_rect.min + padding,
                            galley,
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "WGPU not ready",
                        egui::FontId::proportional(14.0),
                        egui::Color32::GRAY,
                    );
                }
            });

            let mut params_height = 0.0;
            let separator_height = 1.0;
            let min_params = 140.0;
            if self.project.settings.panels.show_inspector {
                let selected = self.node_graph.selected_node_id();
                let rows = self.node_graph.inspector_row_count(&self.project.graph);
                let row_height = 36.0;
                let header_height = 46.0;
                let padding = 40.0;
                let desired_height = header_height + rows as f32 * row_height + padding;
                let max = right_rect.height() * 0.5;
                let target = desired_height.clamp(min_params, max.max(min_params));
                if selected != self.last_selected_node
                    || (self.project.settings.node_params_split * right_rect.height()) < target
                {
                    let clamped = desired_height.clamp(min_params, max.max(min_params));
                    self.project.settings.node_params_split =
                        (clamped / right_rect.height()).clamp(0.1, 0.5);
                    self.last_selected_node = selected;
                }
                params_height = (right_rect.height()
                    * self.project.settings.node_params_split.clamp(0.1, 0.5))
                .clamp(min_params, right_rect.height() * 0.5);
            }
            let params_ratio = if params_height > 0.0 { 1.0 } else { 0.0 };
            let params_rect = if params_ratio > 0.0 {
                egui::Rect::from_min_size(
                    right_rect.min,
                    egui::vec2(right_rect.width(), params_height),
                )
            } else {
                egui::Rect::from_min_size(right_rect.min, egui::vec2(right_rect.width(), 0.0))
            };
            let separator_rect = if params_ratio > 0.0 {
                egui::Rect::from_min_size(
                    egui::pos2(right_rect.min.x, params_rect.max.y),
                    egui::vec2(right_rect.width(), separator_height),
                )
            } else {
                egui::Rect::from_min_size(right_rect.min, egui::vec2(0.0, 0.0))
            };
            let graph_rect = if params_ratio > 0.0 {
                egui::Rect::from_min_max(
                    egui::pos2(right_rect.min.x, separator_rect.max.y),
                    right_rect.max,
                )
            } else {
                right_rect
            };

            if params_ratio > 0.0 {
                let sep_response = ui.interact(
                    separator_rect,
                    ui.make_persistent_id("node_split"),
                    egui::Sense::drag(),
                );
                if sep_response.dragged() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let local =
                            (pos.y - right_rect.min.y).clamp(min_params, right_rect.height() * 0.5);
                        self.project.settings.node_params_split =
                            (local / right_rect.height()).clamp(0.1, 0.5);
                    }
                }
                let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70));
                ui.painter().line_segment(
                    [
                        egui::pos2(separator_rect.left(), separator_rect.center().y),
                        egui::pos2(separator_rect.right(), separator_rect.center().y),
                    ],
                    stroke,
                );
            }

            if params_ratio > 0.0 {
                ui.scope_builder(egui::UiBuilder::new().max_rect(params_rect), |ui| {
                    let frame = egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(55, 55, 55))
                        .inner_margin(egui::Margin::symmetric(16, 12));
                    frame.show(ui, |ui| {
                        let style = ui.style_mut();
                        style.visuals = egui::Visuals::dark();
                        let text_color = egui::Color32::from_rgb(230, 230, 230);
                        style.visuals.override_text_color = Some(text_color);
                        style.visuals.widgets.inactive.fg_stroke.color = text_color;
                        style.visuals.widgets.hovered.fg_stroke.color = text_color;
                        style.visuals.widgets.active.fg_stroke.color = text_color;
                        style.visuals.widgets.inactive.bg_fill =
                            egui::Color32::from_rgb(60, 60, 60);
                        style.visuals.widgets.hovered.bg_fill =
                            egui::Color32::from_rgb(75, 75, 75);
                        style.visuals.widgets.active.bg_fill =
                            egui::Color32::from_rgb(90, 90, 90);
                        style.visuals.widgets.inactive.bg_stroke.color =
                            egui::Color32::from_rgb(85, 85, 85);
                        style.visuals.widgets.hovered.bg_stroke.color =
                            egui::Color32::from_rgb(105, 105, 105);
                        style.visuals.widgets.active.bg_stroke.color =
                            egui::Color32::from_rgb(125, 125, 125);
                        style.visuals.extreme_bg_color = egui::Color32::from_rgb(45, 45, 45);
                        style.visuals.faint_bg_color = egui::Color32::from_rgb(55, 55, 55);
                        style.text_styles.insert(
                            egui::TextStyle::Body,
                            egui::FontId::proportional(16.0),
                        );
                        style.text_styles.insert(
                            egui::TextStyle::Button,
                            egui::FontId::proportional(16.0),
                        );
                        style.text_styles.insert(
                            egui::TextStyle::Heading,
                            egui::FontId::proportional(18.0),
                        );
                        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
                        style.spacing.interact_size = egui::vec2(44.0, 26.0);

                        let max_height = ui.available_height();
                        egui::ScrollArea::vertical()
                            .max_height(max_height)
                            .show(ui, |ui| {
                                if self
                                    .node_graph
                                    .show_inspector(ui, &mut self.project.graph)
                                {
                                    self.mark_eval_dirty();
                                }
                            });
                    });
                });
            }

            ui.scope_builder(egui::UiBuilder::new().max_rect(graph_rect), |ui| {
                self.node_graph
                    .show(ui, &mut self.project.graph, &mut self.eval_dirty);
            });
            self.last_node_graph_rect = Some(right_rect);
        });

        self.evaluate_if_needed();
    }
}

pub(crate) fn setup_tracing() -> (ConsoleBuffer, Arc<AtomicU8>) {
    let console = ConsoleBuffer::new();
    let log_level_state = Arc::new(AtomicU8::new(level_filter_to_u8(LevelFilter::INFO)));
    let filter_state = log_level_state.clone();
    let filter_layer = tracing_subscriber::filter::filter_fn(move |metadata| {
        let level = match filter_state.load(Ordering::Relaxed) {
            value if value == level_filter_to_u8(LevelFilter::ERROR) => Level::ERROR,
            value if value == level_filter_to_u8(LevelFilter::WARN) => Level::WARN,
            value if value == level_filter_to_u8(LevelFilter::INFO) => Level::INFO,
            value if value == level_filter_to_u8(LevelFilter::DEBUG) => Level::DEBUG,
            _ => Level::TRACE,
        };
        metadata.level() <= &level
    });
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(ConsoleMakeWriter {
            buffer: console.clone(),
        });

    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(filter_layer))
        .init();

    (console, log_level_state)
}

fn level_filter_to_u8(level: LevelFilter) -> u8 {
    match level {
        LevelFilter::OFF => 0,
        LevelFilter::ERROR => 1,
        LevelFilter::WARN => 2,
        LevelFilter::INFO => 3,
        LevelFilter::DEBUG => 4,
        LevelFilter::TRACE => 5,
    }
}

fn scene_to_render(scene: &SceneSnapshot) -> RenderScene {
    RenderScene {
        mesh: RenderMesh {
            positions: scene.mesh.positions.clone(),
            normals: scene.mesh.normals.clone(),
            indices: scene.mesh.indices.clone(),
            corner_normals: scene.mesh.corner_normals.clone(),
        },
        base_color: scene.base_color,
    }
}

impl GraphoApp {
    fn sync_wgpu_renderer(&mut self, frame: &eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };

        if self.viewport_renderer.is_none() {
            self.viewport_renderer = Some(ViewportRenderer::new(render_state.target_format));
        }

        if let (Some(renderer), Some(scene)) = (&self.viewport_renderer, self.pending_scene.take())
        {
            renderer.set_scene(scene);
        }
    }

    fn handle_viewport_input(&mut self, response: &egui::Response) {
        if !response.hovered() {
            return;
        }

        let camera = &mut self.project.settings.camera;
        let orbit_speed = 0.01;
        let pan_speed = 0.0025 * camera.distance.max(0.1);
        let zoom_speed = 0.1;

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_motion();
            camera.yaw += delta.x * orbit_speed;
            camera.pitch = (camera.pitch + delta.y * orbit_speed).clamp(-1.54, 1.54);
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_motion();
            camera.target[0] -= delta.x * pan_speed;
            camera.target[1] += delta.y * pan_speed;
        }

        let scroll_delta = response.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.0 {
            let zoom = 1.0 - (scroll_delta * zoom_speed / 100.0);
            camera.distance = (camera.distance * zoom).clamp(0.1, 1000.0);
        }
    }

    fn mark_eval_dirty(&mut self) {
        self.eval_dirty = true;
        self.last_param_change = Some(Instant::now());
    }

    fn evaluate_if_needed(&mut self) {
        if !self.eval_dirty {
            return;
        }

        if let Some(last_change) = self.last_param_change {
            let debounce = Duration::from_millis(150);
            if last_change.elapsed() < debounce {
                return;
            }
        }

        self.eval_dirty = false;
        self.last_param_change = None;
        self.evaluate_graph();
    }

    fn evaluate_graph(&mut self) {
        let outputs: Vec<_> = self
            .project
            .graph
            .nodes()
            .filter(|node| node.name == "Output")
            .map(|node| node.id)
            .collect();

        let output_node = match outputs.as_slice() {
            [] => {
                if self.last_output_state != OutputState::Missing {
                    tracing::warn!("no Output node found; nothing to evaluate");
                    self.last_output_state = OutputState::Missing;
                }
                if let Some(renderer) = &self.viewport_renderer {
                    renderer.clear_scene();
                }
                self.pending_scene = None;
                return;
            }
            [node] => *node,
            many => {
                if self.last_output_state != OutputState::Multiple {
                    tracing::error!(
                        "multiple Output nodes found ({}); only one is supported",
                        many.len()
                    );
                    self.last_output_state = OutputState::Multiple;
                }
                if let Some(renderer) = &self.viewport_renderer {
                    renderer.clear_scene();
                }
                self.pending_scene = None;
                return;
            }
        };
        self.last_output_state = OutputState::Ok;

        let start = Instant::now();
        match evaluate_mesh_graph(&self.project.graph, output_node, &mut self.eval_state) {
            Ok(result) => {
                self.last_eval_ms = Some(start.elapsed().as_secs_f32() * 1000.0);
                let output_valid = result.report.output_valid;
                let (error_nodes, error_messages) = collect_error_state(&result.report);
                self.node_graph.set_error_state(error_nodes, error_messages);
                self.last_eval_report = Some(result.report);
                if let Some(mesh) = result.output {
                    let snapshot = SceneSnapshot::from_mesh(&mesh, [0.7, 0.72, 0.75]);
                    let scene = scene_to_render(&snapshot);
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.set_scene(scene);
                    } else {
                        self.pending_scene = Some(scene);
                    }
                } else {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                }
                if !output_valid {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                }
            }
            Err(err) => {
                tracing::error!("eval failed: {:?}", err);
                self.node_graph
                    .set_error_state(HashSet::new(), HashMap::new());
            }
        }
    }

}

fn collect_error_state(
    report: &core::EvalReport,
) -> (HashSet<core::NodeId>, HashMap<core::NodeId, String>) {
    let mut nodes = HashSet::new();
    let mut messages = HashMap::new();
    for err in &report.errors {
        match err {
            core::EvalError::Node { node, message } => {
                nodes.insert(*node);
                messages.entry(*node).or_insert_with(|| message.clone());
            }
            core::EvalError::Upstream { node, upstream } => {
                nodes.insert(*node);
                messages
                    .entry(*node)
                    .or_insert_with(|| format!("Upstream error in nodes: {:?}", upstream));
                for upstream_node in upstream {
                    nodes.insert(*upstream_node);
                }
            }
        }
    }
    (nodes, messages)
}
