use eframe::egui;
use grapho_core::ShadingMode;

use super::node_info::NodeInfoPanel;
use super::spreadsheet::show_spreadsheet;
use super::GraphoApp;

impl eframe::App for GraphoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.sync_wgpu_renderer(frame);
        let pointer_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if ctx.input(|i| i.pointer.any_released()) {
            self.flush_pending_undo();
        }
        if !ctx.wants_keyboard_input() {
            let undo_pressed = ctx.input(|i| {
                i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
            });
            let redo_pressed = ctx.input(|i| {
                (i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift)
                    || (i.key_pressed(egui::Key::Y) && i.modifiers.command)
            });
            if undo_pressed {
                self.try_undo();
            } else if redo_pressed {
                self.try_redo();
            }
        }
        let mut undo_pushed = false;
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

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui.button("Open...").clicked() {
                            self.open_project_dialog();
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
                            self.save_project_dialog();
                            ui.close();
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        ui.add_enabled(false, egui::Button::new("Open..."));
                        ui.add_enabled(false, egui::Button::new("Save"));
                        ui.add_enabled(false, egui::Button::new("Save As..."));
                        ui.label("File I/O is not available in web builds.");
                    }
                });

                ui.separator();
                ui.label("grapho");
                ui.separator();
                ui.checkbox(
                    &mut self.project.settings.panels.show_inspector,
                    "Parameters",
                );
                ui.checkbox(
                    &mut self.project.settings.panels.show_spreadsheet,
                    "Spreadsheet",
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
                                    &mut self.project.settings.render_debug.show_points,
                                    "Points",
                                );
                                if self.project.settings.render_debug.show_points {
                                    ui.horizontal(|ui| {
                                        ui.label("Point size");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.project.settings.render_debug.point_size,
                                            )
                                            .speed(0.5)
                                            .range(1.0..=24.0),
                                        );
                                    });
                                }
                                ui.checkbox(
                                    &mut self.project.settings.render_debug.key_shadows,
                                    "Key shadows",
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
                                        self.project.settings.render_debug.depth_far =
                                            near + 0.01;
                                    }
                                }

                                ui.separator();
                                ui.label("Evaluation");
                                if ui.button("Create demo graph").clicked() {
                                    let snapshot = self.snapshot_undo();
                                    self.node_graph.add_demo_graph(&mut self.project.graph);
                                    self.mark_eval_dirty();
                                    if !undo_pushed {
                                        self.queue_undo_snapshot(snapshot, pointer_down);
                                        undo_pushed = true;
                                    }
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
                                    let mut nodes: Vec<_> =
                                        report.node_reports.values().collect();
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
                                    if !report.dirty.is_empty() {
                                        ui.separator();
                                        ui.label(format!(
                                            "Dirty nodes: {}",
                                            report.dirty.len()
                                        ));
                                        for entry in &report.dirty {
                                            let reason = match entry.reason {
                                                grapho_core::DirtyReason::NewNode => "new",
                                                grapho_core::DirtyReason::ParamChanged => "param",
                                                grapho_core::DirtyReason::UpstreamChanged => {
                                                    "upstream"
                                                }
                                                grapho_core::DirtyReason::ParamAndUpstreamChanged => {
                                                    "param+upstream"
                                                }
                                            };
                                            ui.label(format!("{:?}: {}", entry.node, reason));
                                        }
                                    }
                                }

                                egui::ComboBox::from_label("Log level")
                                    .selected_text(format!("{:?}", self.log_level))
                                    .show_ui(ui, |ui| {
                                        for level in [
                                            tracing_subscriber::filter::LevelFilter::ERROR,
                                            tracing_subscriber::filter::LevelFilter::WARN,
                                            tracing_subscriber::filter::LevelFilter::INFO,
                                            tracing_subscriber::filter::LevelFilter::DEBUG,
                                            tracing_subscriber::filter::LevelFilter::TRACE,
                                        ] {
                                            if ui
                                                .selectable_label(self.log_level == level, format!("{:?}", level))
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
                                let console_lines = self.console.snapshot();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for line in console_lines {
                                        ui.label(line);
                                    }
                                });
                            });
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let (left_rect, right_rect) = {
                let full = ui.available_rect_before_wrap();
                let ratio = self.project.settings.viewport_split.clamp(0.2, 0.8);
                let left_width = full.width() * ratio;
                let left = egui::Rect::from_min_size(
                    full.min,
                    egui::vec2(left_width, full.height()),
                );
                let right = egui::Rect::from_min_max(
                    egui::pos2(left.max.x, full.min.y),
                    full.max,
                );
                (left, right)
            };

            ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                let full = ui.available_rect_before_wrap();
                let show_sheet = self.project.settings.panels.show_spreadsheet;
                let separator_height = if show_sheet { 1.0 } else { 0.0 };
                let min_sheet = 140.0;
                let min_viewport = 220.0;
                let total_height = full.height();
                let (viewport_height, sheet_height) = if show_sheet {
                    let mut viewport_height = (total_height
                        * self.project.settings.viewport_sheet_split.clamp(0.3, 0.9))
                    .clamp(min_viewport, (total_height - min_sheet).max(min_viewport));
                    if total_height <= min_viewport + min_sheet + separator_height {
                        viewport_height = total_height.max(min_viewport);
                    }
                    let sheet_height =
                        (total_height - viewport_height - separator_height).max(0.0);
                    (viewport_height, sheet_height)
                } else {
                    (total_height.max(min_viewport), 0.0)
                };
                let viewport_rect = egui::Rect::from_min_size(
                    full.min,
                    egui::vec2(full.width(), viewport_height),
                );
                let separator_rect = egui::Rect::from_min_size(
                    egui::pos2(full.min.x, viewport_rect.max.y),
                    egui::vec2(full.width(), separator_height),
                );
                let sheet_rect = egui::Rect::from_min_size(
                    egui::pos2(full.min.x, separator_rect.max.y),
                    egui::vec2(full.width(), sheet_height),
                );

                if sheet_height > 0.0 {
                    let sep_response = ui.interact(
                        separator_rect,
                        ui.make_persistent_id("viewport_sheet_split"),
                        egui::Sense::drag(),
                    );
                    if sep_response.dragged() {
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            let local = (pos.y - full.min.y)
                                .clamp(min_viewport, (total_height - min_sheet).max(min_viewport));
                            self.project.settings.viewport_sheet_split =
                                (local / total_height).clamp(0.3, 0.9);
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

                ui.scope_builder(egui::UiBuilder::new().max_rect(viewport_rect), |ui| {
                    let available = ui.available_size();
                    let (rect, response) =
                        ui.allocate_exact_size(available, egui::Sense::click_and_drag());
                    self.handle_viewport_input(&response);
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 28, 28));
                    if let Some(renderer) = &self.viewport_renderer {
                        let camera = self.camera_state();
                        let debug = self.viewport_debug();
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
                            painter.rect_filled(
                                bg_rect,
                                4.0,
                                egui::Color32::from_black_alpha(160),
                            );
                            painter.galley(bg_rect.min + padding, galley, egui::Color32::WHITE);
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

                    let toolbar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.max.x - 36.0, rect.min.y + 8.0),
                        egui::vec2(28.0, 160.0),
                    );
                    ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
                        ui.set_min_width(toolbar_rect.width());
                        ui.visuals_mut().widgets.inactive.bg_fill =
                            egui::Color32::from_rgb(45, 45, 45);
                        ui.visuals_mut().widgets.hovered.bg_fill =
                            egui::Color32::from_rgb(70, 70, 70);
                        ui.visuals_mut().widgets.active.bg_fill =
                            egui::Color32::from_rgb(90, 90, 90);
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                        let debug = &mut self.project.settings.render_debug;
                        if ui
                            .add(egui::Button::new("G").selected(debug.show_grid))
                            .on_hover_text("Grid")
                            .clicked()
                        {
                            debug.show_grid = !debug.show_grid;
                        }
                        if ui
                            .add(egui::Button::new("A").selected(debug.show_axes))
                            .on_hover_text("Axes")
                            .clicked()
                        {
                            debug.show_axes = !debug.show_axes;
                        }
                        if ui
                            .add(egui::Button::new("P").selected(debug.show_points))
                            .on_hover_text("Points")
                            .clicked()
                        {
                            debug.show_points = !debug.show_points;
                        }
                        if ui
                            .add(egui::Button::new("S").selected(debug.key_shadows))
                            .on_hover_text("Key shadows")
                            .clicked()
                        {
                            debug.key_shadows = !debug.key_shadows;
                        }
                    });
                });

                if sheet_height > 0.0 {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(sheet_rect), |ui| {
                        ui.painter().rect_filled(
                            sheet_rect,
                            0.0,
                            egui::Color32::from_rgb(38, 38, 38),
                        );
                        let frame = egui::Frame::NONE
                            .fill(egui::Color32::from_rgb(38, 38, 38))
                            .inner_margin(egui::Margin::symmetric(12, 10));
                        frame.show(ui, |ui| {
                            let style = ui.style_mut();
                            style.visuals = egui::Visuals::dark();
                            style.visuals.override_text_color =
                                Some(egui::Color32::from_rgb(220, 220, 220));
                            style.spacing.item_spacing = egui::vec2(10.0, 6.0);
                            let selected = self.node_graph.selected_node_id();
                            let mesh = selected.and_then(|id| self.eval_state.mesh_for_node(id));
                            show_spreadsheet(ui, mesh, &mut self.spreadsheet_domain);
                        });
                    });
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
                ui.painter().rect_filled(
                    params_rect,
                    0.0,
                    egui::Color32::from_rgb(55, 55, 55),
                );
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
                                let snapshot = self.snapshot_undo();
                                if self
                                    .node_graph
                                    .show_inspector(ui, &mut self.project.graph)
                                {
                                    self.mark_eval_dirty();
                                    if !undo_pushed {
                                        self.queue_undo_snapshot(snapshot, pointer_down);
                                        undo_pushed = true;
                                    }
                                }
                            });
                    });
                });
            }

            ui.scope_builder(egui::UiBuilder::new().max_rect(graph_rect), |ui| {
                let snapshot = self.snapshot_undo();
                self.node_graph
                    .show(ui, &mut self.project.graph, &mut self.eval_dirty);
                let layout_moved = self.node_graph.take_layout_changed();
                if (self.node_graph.take_changed() || layout_moved) && !undo_pushed {
                    self.queue_undo_snapshot(snapshot, pointer_down);
                    undo_pushed = true;
                }
            });
            self.last_node_graph_rect = Some(right_rect);
        });

        let middle_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
        if middle_down {
            let hover_pos = ctx.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.latest_pos()));
            if let Some(pos) = hover_pos {
                if let Some(node_id) = self.node_graph.node_at_screen_pos(pos) {
                    self.held_info_panel = Some(NodeInfoPanel {
                        node_id,
                        screen_pos: pos,
                        open: true,
                    });
                } else {
                    self.held_info_panel = None;
                }
            }
        } else {
            self.held_info_panel = None;
        }

        if let Some(request) = self.node_graph.take_info_request() {
            self.info_panel = Some(NodeInfoPanel {
                node_id: request.node_id,
                screen_pos: request.screen_pos,
                open: true,
            });
        }

        let mut info_panel = self.info_panel.take();
        self.show_node_info_panel(ctx, &mut info_panel);
        self.info_panel = info_panel;

        let mut held_info_panel = self.held_info_panel.take();
        self.show_node_info_panel(ctx, &mut held_info_panel);
        self.held_info_panel = held_info_panel;

        self.evaluate_if_needed();
    }
}
