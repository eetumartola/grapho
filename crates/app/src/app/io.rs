use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use grapho_core::Project;

use super::GraphoApp;

const DEFAULT_GRAPH_PATH: &str = "graphs/default.json";

impl GraphoApp {
    pub(super) fn new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
        self.node_graph.reset();
        self.undo_stack.clear();
        self.pending_undo = None;
        self.eval_dirty = true;
        self.pending_scene = None;
        tracing::info!("new project created");
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_project_to(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(&self.project).map_err(io::Error::other)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub(super) fn save_project_to(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "File save is not available in web builds",
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn load_project_from(&mut self, path: &Path) -> io::Result<()> {
        let data = std::fs::read(path)?;
        let project: Project = serde_json::from_slice(&data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        self.project = project;
        self.project_path = Some(path.to_path_buf());
        self.node_graph.reset();
        self.undo_stack.clear();
        self.pending_undo = None;
        self.eval_dirty = true;
        self.pending_scene = None;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub(super) fn load_project_from(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "File load is not available in web builds",
        ))
    }

    pub(crate) fn try_load_default_graph(&mut self) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn open_project_dialog(&mut self) {
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
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_project_dialog(&mut self) {
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
    }
}
