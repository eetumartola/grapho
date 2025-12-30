use crate::mesh::Mesh;

#[derive(Debug, Clone)]
pub struct SceneMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub mesh: SceneMesh,
    pub base_color: [f32; 3],
}

impl SceneMesh {
    pub fn from_mesh(mesh: &Mesh) -> Self {
        let normals = match &mesh.normals {
            Some(normals) => normals.clone(),
            None => {
                let mut temp = mesh.clone();
                temp.compute_normals();
                temp.normals
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; mesh.positions.len()])
            }
        };

        Self {
            positions: mesh.positions.clone(),
            normals,
            indices: mesh.indices.clone(),
        }
    }
}

impl SceneSnapshot {
    pub fn from_mesh(mesh: &Mesh, base_color: [f32; 3]) -> Self {
        Self {
            mesh: SceneMesh::from_mesh(mesh),
            base_color,
        }
    }
}
