use serde::{Deserialize, Serialize};

use crate::graph::Graph;

pub const PROJECT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub settings: ProjectSettings,
    pub graph: Graph,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: PROJECT_VERSION,
            settings: ProjectSettings::default(),
            graph: Graph::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub viewport_split: f32,
    pub viewport_sheet_split: f32,
    pub node_params_split: f32,
    pub panels: PanelSettings,
    pub camera: CameraSettings,
    pub render_debug: RenderDebugSettings,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            viewport_split: 0.5,
            viewport_sheet_split: 0.75,
            node_params_split: 0.25,
            panels: PanelSettings::default(),
            camera: CameraSettings::default(),
            render_debug: RenderDebugSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    pub show_inspector: bool,
    pub show_spreadsheet: bool,
    pub show_debug: bool,
    pub show_console: bool,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            show_inspector: true,
            show_spreadsheet: true,
            show_debug: false,
            show_console: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSettings {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadingMode {
    Lit,
    Normals,
    Depth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderDebugSettings {
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_normals: bool,
    pub show_bounds: bool,
    pub normal_length: f32,
    pub show_stats: bool,
    pub show_points: bool,
    pub point_size: f32,
    pub key_shadows: bool,
    pub shading_mode: ShadingMode,
    pub depth_near: f32,
    pub depth_far: f32,
}

impl Default for RenderDebugSettings {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_axes: true,
            show_normals: false,
            show_bounds: false,
            normal_length: 0.3,
            show_stats: true,
            show_points: false,
            point_size: 4.0,
            key_shadows: false,
            shading_mode: ShadingMode::Lit,
            depth_near: 0.5,
            depth_far: 20.0,
        }
    }
}
