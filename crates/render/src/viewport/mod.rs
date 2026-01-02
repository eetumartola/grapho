use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use egui::epaint::{PaintCallback, Rect};
use egui_wgpu::Callback;

use crate::camera::CameraState;
use crate::scene::RenderScene;

mod callback;
mod mesh;
mod pipeline;

use callback::ViewportCallback;

pub struct ViewportRenderer {
    target_format: egui_wgpu::wgpu::TextureFormat,
    stats: Arc<Mutex<ViewportStatsState>>,
    scene: Arc<Mutex<ViewportSceneState>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ViewportShadingMode {
    Lit,
    Normals,
    Depth,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportDebug {
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_normals: bool,
    pub show_bounds: bool,
    pub normal_length: f32,
    pub shading_mode: ViewportShadingMode,
    pub depth_near: f32,
    pub depth_far: f32,
    pub show_points: bool,
    pub point_size: f32,
    pub key_shadows: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewportStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub mesh_count: u32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_uploads: u64,
}

impl Default for ViewportStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            vertex_count: 0,
            triangle_count: 0,
            mesh_count: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_uploads: 0,
        }
    }
}

pub(super) struct ViewportStatsState {
    last_frame: Option<Instant>,
    stats: ViewportStats,
}

pub(super) struct ViewportSceneState {
    version: u64,
    scene: Option<RenderScene>,
}

impl ViewportRenderer {
    pub fn new(target_format: egui_wgpu::wgpu::TextureFormat) -> Self {
        Self {
            target_format,
            stats: Arc::new(Mutex::new(ViewportStatsState {
                last_frame: None,
                stats: ViewportStats::default(),
            })),
            scene: Arc::new(Mutex::new(ViewportSceneState {
                version: 0,
                scene: None,
            })),
        }
    }

    pub fn paint_callback(
        &self,
        rect: Rect,
        camera: CameraState,
        debug: ViewportDebug,
    ) -> PaintCallback {
        Callback::new_paint_callback(
            rect,
            ViewportCallback {
                target_format: self.target_format,
                rect,
                camera,
                debug,
                stats: self.stats.clone(),
                scene: self.scene.clone(),
            },
        )
    }

    pub fn stats_snapshot(&self) -> ViewportStats {
        self.stats
            .lock()
            .map(|state| state.stats)
            .unwrap_or_default()
    }

    pub fn set_scene(&self, scene: RenderScene) {
        if let Ok(mut state) = self.scene.lock() {
            state.version = state.version.wrapping_add(1);
            state.scene = Some(scene);
        }
    }

    pub fn clear_scene(&self) {
        if let Ok(mut state) = self.scene.lock() {
            state.version = state.version.wrapping_add(1);
            state.scene = None;
        }
    }
}
