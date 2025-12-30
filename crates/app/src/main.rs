use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core::{
    default_params, evaluate_mesh_graph, node_definition, BuiltinNodeKind, MeshEvalState, NodeId,
    ParamValue, Project, SceneSnapshot, ShadingMode,
};
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

mod headless;

const MAX_LOG_LINES: usize = 500;

#[derive(Clone)]
struct ConsoleBuffer {
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

struct GraphoApp {
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
    demo_graph: Option<DemoGraphIds>,
}

impl GraphoApp {
    fn new(console: ConsoleBuffer, log_level_state: Arc<AtomicU8>) -> Self {
        let mut box_mesh = core::make_box([1.0, 1.0, 1.0]);
        box_mesh.compute_normals();
        let snapshot = SceneSnapshot::from_mesh(&box_mesh, [0.7, 0.72, 0.75]);
        Self {
            project: Project::default(),
            project_path: None,
            console,
            log_level: LevelFilter::INFO,
            log_level_state,
            viewport_renderer: None,
            pending_scene: Some(scene_to_render(&snapshot)),
            eval_state: MeshEvalState::new(),
            last_eval_report: None,
            last_eval_ms: None,
            eval_dirty: false,
            last_param_change: None,
            demo_graph: None,
        }
    }

    fn new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
        self.demo_graph = None;
        self.eval_dirty = true;
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
}

impl eframe::App for GraphoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.sync_wgpu_renderer(frame);
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close_menu();
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
                        ui.close_menu();
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
                        ui.close_menu();
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
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.label("grapho");
                ui.separator();
                ui.checkbox(
                    &mut self.project.settings.panels.show_inspector,
                    "Inspector",
                );
                ui.checkbox(&mut self.project.settings.panels.show_debug, "Debug");
                ui.checkbox(&mut self.project.settings.panels.show_console, "Console");
            });
        });

        if self.project.settings.panels.show_inspector
            || self.project.settings.panels.show_debug
            || self.project.settings.panels.show_console
        {
            egui::SidePanel::right("side_panels")
                .resizable(true)
                .default_width(280.0)
                .show(ctx, |ui| {
                    if self.project.settings.panels.show_inspector {
                        egui::CollapsingHeader::new("Inspector")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("No selection.");
                            });
                    }

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
                                            .clamp_range(0.01..=10.0),
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
                                            .clamp_range(0.01..=1000.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Far");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.project.settings.render_debug.depth_far,
                                            )
                                            .speed(0.1)
                                            .clamp_range(0.01..=5000.0),
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
                                if self.demo_graph.is_none() {
                                    if ui.button("Create demo graph").clicked() {
                                        self.create_demo_graph();
                                    }
                                }
                                if let Some(ids) = self.demo_graph {
                                    if let Some(node) = self.project.graph.node(ids.box_node) {
                                        ui.label("Box");
                                        let mut size =
                                            get_vec3_param(node, "size", [1.0, 1.0, 1.0]);
                                        let mut changed = false;
                                        ui.horizontal(|ui| {
                                            ui.label("Size");
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut size[0]).speed(0.1))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut size[1]).speed(0.1))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut size[2]).speed(0.1))
                                                .changed();
                                        });
                                        if changed {
                                            let _ = self.project.graph.set_param(
                                                ids.box_node,
                                                "size",
                                                ParamValue::Vec3(size),
                                            );
                                            self.mark_eval_dirty();
                                        }
                                    }

                                    if let Some(node) = self.project.graph.node(ids.transform_node)
                                    {
                                        ui.separator();
                                        ui.label("Transform");
                                        let mut translate =
                                            get_vec3_param(node, "translate", [0.0, 0.0, 0.0]);
                                        let mut rotate =
                                            get_vec3_param(node, "rotate_deg", [0.0, 0.0, 0.0]);
                                        let mut scale =
                                            get_vec3_param(node, "scale", [1.0, 1.0, 1.0]);
                                        let mut changed = false;
                                        ui.horizontal(|ui| {
                                            ui.label("Translate");
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut translate[0]))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut translate[1]))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut translate[2]))
                                                .changed();
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Rotate");
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut rotate[0]))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut rotate[1]))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut rotate[2]))
                                                .changed();
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Scale");
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut scale[0]).speed(0.1))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut scale[1]).speed(0.1))
                                                .changed();
                                            changed |= ui
                                                .add(egui::DragValue::new(&mut scale[2]).speed(0.1))
                                                .changed();
                                        });
                                        if changed {
                                            let _ = self.project.graph.set_param(
                                                ids.transform_node,
                                                "translate",
                                                ParamValue::Vec3(translate),
                                            );
                                            let _ = self.project.graph.set_param(
                                                ids.transform_node,
                                                "rotate_deg",
                                                ParamValue::Vec3(rotate),
                                            );
                                            let _ = self.project.graph.set_param(
                                                ids.transform_node,
                                                "scale",
                                                ParamValue::Vec3(scale),
                                            );
                                            self.mark_eval_dirty();
                                        }
                                    }

                                    if ui.button("Recompute now").clicked() {
                                        self.eval_dirty = false;
                                        self.last_param_change = None;
                                        self.evaluate_graph();
                                    }
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

            ui.allocate_ui_at_rect(left_rect, |ui| {
                ui.heading("Viewport");
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
                        let galley = ui.fonts(|f| {
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

            ui.allocate_ui_at_rect(right_rect, |ui| {
                ui.heading("Node Graph");
                ui.label("egui-snarl graph placeholder.");
            });
        });

        self.evaluate_if_needed();
    }
}

fn main() -> eframe::Result<()> {
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
        Box::new(|_cc| Box::new(GraphoApp::new(console, log_level_state))),
    )
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
        let output_node = match self.demo_graph {
            Some(ids) => ids.output_node,
            None => return,
        };

        let start = Instant::now();
        match evaluate_mesh_graph(&self.project.graph, output_node, &mut self.eval_state) {
            Ok(result) => {
                self.last_eval_ms = Some(start.elapsed().as_secs_f32() * 1000.0);
                self.last_eval_report = Some(result.report);
                if let Some(mesh) = result.output {
                    let snapshot = SceneSnapshot::from_mesh(&mesh, [0.7, 0.72, 0.75]);
                    let scene = scene_to_render(&snapshot);
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.set_scene(scene);
                    } else {
                        self.pending_scene = Some(scene);
                    }
                }
            }
            Err(err) => {
                tracing::error!("eval failed: {:?}", err);
            }
        }
    }

    fn create_demo_graph(&mut self) {
        let graph = &mut self.project.graph;
        let box_id = graph.add_node(node_definition(BuiltinNodeKind::Box));
        let transform_id = graph.add_node(node_definition(BuiltinNodeKind::Transform));
        let output_id = graph.add_node(node_definition(BuiltinNodeKind::Output));

        let box_out = graph.node(box_id).unwrap().outputs[0];
        let transform_in = graph.node(transform_id).unwrap().inputs[0];
        let transform_out = graph.node(transform_id).unwrap().outputs[0];
        let output_in = graph.node(output_id).unwrap().inputs[0];
        let _ = graph.add_link(box_out, transform_in);
        let _ = graph.add_link(transform_out, output_in);

        apply_default_params(graph, box_id, BuiltinNodeKind::Box);
        apply_default_params(graph, transform_id, BuiltinNodeKind::Transform);

        self.demo_graph = Some(DemoGraphIds {
            box_node: box_id,
            transform_node: transform_id,
            output_node: output_id,
        });
        self.mark_eval_dirty();
    }
}

#[derive(Clone, Copy)]
struct DemoGraphIds {
    box_node: NodeId,
    transform_node: NodeId,
    output_node: NodeId,
}

fn apply_default_params(graph: &mut core::Graph, node_id: NodeId, kind: BuiltinNodeKind) {
    let params = default_params(kind);
    for (key, value) in params.values {
        let _ = graph.set_param(node_id, key, value);
    }
}

fn get_vec3_param(node: &core::Node, key: &str, default: [f32; 3]) -> [f32; 3] {
    node.params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Vec3(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(default)
}
