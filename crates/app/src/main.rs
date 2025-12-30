use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use core::Project;
use eframe::egui;
use render::ViewportRenderer;
use rfd::FileDialog;
use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

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
}

impl GraphoApp {
    fn new(console: ConsoleBuffer, log_level_state: Arc<AtomicU8>) -> Self {
        Self {
            project: Project::default(),
            project_path: None,
            console,
            log_level: LevelFilter::INFO,
            log_level_state,
            viewport_renderer: None,
        }
    }

    fn new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
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

#[derive(Debug, Deserialize)]
struct HeadlessPlan {
    #[serde(default)]
    nodes: Vec<PlanNode>,
    #[serde(default)]
    links: Vec<PlanLink>,
    #[serde(default)]
    output_node: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlanNode {
    name: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    inputs: Vec<PlanPin>,
    #[serde(default)]
    outputs: Vec<PlanPin>,
}

#[derive(Debug, Deserialize)]
struct PlanPin {
    name: String,
    pin_type: core::PinType,
}

#[derive(Debug, Deserialize)]
struct PlanLink {
    from: PlanEndpoint,
    to: PlanEndpoint,
}

#[derive(Debug, Deserialize)]
struct PlanEndpoint {
    node: String,
    pin: String,
}

struct HeadlessArgs {
    plan_path: Option<PathBuf>,
    save_path: Option<PathBuf>,
    print: bool,
}

fn maybe_run_headless(args: &[String]) -> Result<bool, String> {
    if !args.iter().any(|arg| arg == "--headless") {
        return Ok(false);
    }

    let parsed = parse_headless_args(args)?;
    let plan = if let Some(path) = parsed.plan_path {
        load_headless_plan(&path)?
    } else {
        default_headless_plan()
    };

    let project = build_project_from_plan(&plan)?;

    if let Some(path) = parsed.save_path {
        save_project_json(&project, &path)?;
        tracing::info!("headless: saved project to {:?}", path);
    }

    if parsed.print {
        let json = serde_json::to_string_pretty(&project).map_err(|err| err.to_string())?;
        println!("{json}");
    }

    if let Some(output) = plan.output_node {
        validate_topo_sort(&project, &output)?;
    }

    tracing::info!("headless: completed");
    Ok(true)
}

fn parse_headless_args(args: &[String]) -> Result<HeadlessArgs, String> {
    let mut plan_path = None;
    let mut save_path = None;
    let mut print = false;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--headless" => {}
            "--plan" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--plan requires a path".to_string())?;
                plan_path = Some(PathBuf::from(value));
            }
            "--save" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--save requires a path".to_string())?;
                save_path = Some(PathBuf::from(value));
            }
            "--print" => {
                print = true;
            }
            "--help" => {
                print_headless_help();
                process::exit(0);
            }
            _ => {}
        }
    }

    Ok(HeadlessArgs {
        plan_path,
        save_path,
        print,
    })
}

fn print_headless_help() {
    println!("Headless mode options:\n  --headless\n  --plan <path>\n  --save <path>\n  --print");
}

fn load_headless_plan(path: &Path) -> Result<HeadlessPlan, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    serde_json::from_slice(&data).map_err(|err| err.to_string())
}

fn default_headless_plan() -> HeadlessPlan {
    HeadlessPlan {
        nodes: vec![
            PlanNode {
                name: "Box".to_string(),
                category: "Source".to_string(),
                inputs: Vec::new(),
                outputs: vec![PlanPin {
                    name: "mesh".to_string(),
                    pin_type: core::PinType::Mesh,
                }],
            },
            PlanNode {
                name: "Output".to_string(),
                category: "Output".to_string(),
                inputs: vec![PlanPin {
                    name: "in".to_string(),
                    pin_type: core::PinType::Mesh,
                }],
                outputs: Vec::new(),
            },
        ],
        links: vec![PlanLink {
            from: PlanEndpoint {
                node: "Box".to_string(),
                pin: "mesh".to_string(),
            },
            to: PlanEndpoint {
                node: "Output".to_string(),
                pin: "in".to_string(),
            },
        }],
        output_node: Some("Output".to_string()),
    }
}

fn build_project_from_plan(plan: &HeadlessPlan) -> Result<Project, String> {
    let mut project = Project::default();
    let mut name_to_id = std::collections::HashMap::new();

    for node in &plan.nodes {
        let node_id = project.graph.add_node(core::NodeDefinition {
            name: node.name.clone(),
            category: node.category.clone(),
            inputs: node
                .inputs
                .iter()
                .map(|pin| core::PinDefinition {
                    name: pin.name.clone(),
                    pin_type: pin.pin_type,
                })
                .collect(),
            outputs: node
                .outputs
                .iter()
                .map(|pin| core::PinDefinition {
                    name: pin.name.clone(),
                    pin_type: pin.pin_type,
                })
                .collect(),
        });
        name_to_id.insert(node.name.clone(), node_id);
    }

    for link in &plan.links {
        let from_node = name_to_id
            .get(&link.from.node)
            .ok_or_else(|| format!("unknown node {}", link.from.node))?;
        let to_node = name_to_id
            .get(&link.to.node)
            .ok_or_else(|| format!("unknown node {}", link.to.node))?;

        let from_pin = find_pin_id(
            &project.graph,
            *from_node,
            &link.from.pin,
            core::PinKind::Output,
        )
        .ok_or_else(|| format!("unknown output pin {}", link.from.pin))?;
        let to_pin = find_pin_id(&project.graph, *to_node, &link.to.pin, core::PinKind::Input)
            .ok_or_else(|| format!("unknown input pin {}", link.to.pin))?;

        project
            .graph
            .add_link(from_pin, to_pin)
            .map_err(|err| format!("link error: {:?}", err))?;
    }

    Ok(project)
}

fn find_pin_id(
    graph: &core::Graph,
    node_id: core::NodeId,
    pin_name: &str,
    kind: core::PinKind,
) -> Option<core::PinId> {
    let node = graph.node(node_id)?;
    let pins = match kind {
        core::PinKind::Input => &node.inputs,
        core::PinKind::Output => &node.outputs,
    };

    pins.iter().copied().find(|pin_id| {
        graph
            .pin(*pin_id)
            .map(|pin| pin.name == pin_name)
            .unwrap_or(false)
    })
}

fn save_project_json(project: &Project, path: &Path) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(project).map_err(|err| err.to_string())?;
    std::fs::write(path, data).map_err(|err| err.to_string())
}

fn validate_topo_sort(project: &Project, output_node_name: &str) -> Result<(), String> {
    let node_id = project
        .graph
        .nodes()
        .find(|node| node.name == output_node_name)
        .map(|node| node.id)
        .ok_or_else(|| format!("output node {} not found", output_node_name))?;

    let order = project
        .graph
        .topo_sort_from(node_id)
        .map_err(|err| format!("topo sort failed: {:?}", err))?;
    tracing::info!("headless: topo order {:?}", order);
    Ok(())
}

fn default_category() -> String {
    "Default".to_string()
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
                if let Some(renderer) = &self.viewport_renderer {
                    let callback = renderer.paint_callback(rect);
                    ui.painter().add(egui::Shape::Callback(callback));
                } else {
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 24, 28));
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
    match maybe_run_headless(&args) {
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

impl GraphoApp {
    fn sync_wgpu_renderer(&mut self, frame: &eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };

        if self.viewport_renderer.is_none() {
            self.viewport_renderer = Some(ViewportRenderer::new(render_state.target_format));
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
            camera.yaw -= delta.x * orbit_speed;
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
}
