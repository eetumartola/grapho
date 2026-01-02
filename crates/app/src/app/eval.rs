use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use grapho_core::{evaluate_mesh_graph, SceneSnapshot, ShadingMode};
use render::{RenderMesh, RenderScene, ViewportDebug, ViewportShadingMode};

use super::{GraphoApp, OutputState};

impl GraphoApp {
    pub(super) fn mark_eval_dirty(&mut self) {
        self.eval_dirty = true;
        self.last_param_change = Some(Instant::now());
    }

    pub(super) fn evaluate_if_needed(&mut self) {
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

    pub(super) fn evaluate_graph(&mut self) {
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

    pub(super) fn viewport_debug(&self) -> ViewportDebug {
        let shading_mode = match self.project.settings.render_debug.shading_mode {
            ShadingMode::Lit => ViewportShadingMode::Lit,
            ShadingMode::Normals => ViewportShadingMode::Normals,
            ShadingMode::Depth => ViewportShadingMode::Depth,
        };
        ViewportDebug {
            show_grid: self.project.settings.render_debug.show_grid,
            show_axes: self.project.settings.render_debug.show_axes,
            show_normals: self.project.settings.render_debug.show_normals,
            show_bounds: self.project.settings.render_debug.show_bounds,
            normal_length: self.project.settings.render_debug.normal_length,
            shading_mode,
            depth_near: self.project.settings.render_debug.depth_near,
            depth_far: self.project.settings.render_debug.depth_far,
            show_points: self.project.settings.render_debug.show_points,
            point_size: self.project.settings.render_debug.point_size,
            key_shadows: self.project.settings.render_debug.key_shadows,
        }
    }
}

pub(super) fn scene_to_render(scene: &SceneSnapshot) -> RenderScene {
    let has_colors = scene.mesh.colors.is_some() || scene.mesh.corner_colors.is_some();
    let base_color = if has_colors {
        [1.0, 1.0, 1.0]
    } else {
        scene.base_color
    };
    RenderScene {
        mesh: RenderMesh {
            positions: scene.mesh.positions.clone(),
            normals: scene.mesh.normals.clone(),
            indices: scene.mesh.indices.clone(),
            corner_normals: scene.mesh.corner_normals.clone(),
            colors: scene.mesh.colors.clone(),
            corner_colors: scene.mesh.corner_colors.clone(),
        },
        base_color,
    }
}

pub(super) fn collect_error_state(
    report: &grapho_core::EvalReport,
) -> (HashSet<grapho_core::NodeId>, HashMap<grapho_core::NodeId, String>) {
    let mut nodes = HashSet::new();
    let mut messages = HashMap::new();
    for err in &report.errors {
        match err {
            grapho_core::EvalError::Node { node, message } => {
                nodes.insert(*node);
                messages.entry(*node).or_insert_with(|| message.clone());
            }
            grapho_core::EvalError::Upstream { node, upstream } => {
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
