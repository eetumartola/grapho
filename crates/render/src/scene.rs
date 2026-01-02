#[derive(Debug, Clone)]
pub struct RenderMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub corner_colors: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub mesh: RenderMesh,
    pub base_color: [f32; 3],
    pub template_mesh: Option<RenderMesh>,
}
