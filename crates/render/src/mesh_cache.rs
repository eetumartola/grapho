use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt as _;

pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    hash: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuMeshCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub uploads: u64,
    pub mesh_count: u32,
}

pub struct GpuMeshCache {
    meshes: HashMap<u64, GpuMesh>,
    hits: AtomicU64,
    misses: AtomicU64,
    uploads: AtomicU64,
}

impl GpuMeshCache {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            uploads: AtomicU64::new(0),
        }
    }

    pub fn get(&self, mesh_id: u64) -> Option<&GpuMesh> {
        let mesh = self.meshes.get(&mesh_id);
        if mesh.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        mesh
    }

    pub fn upload_or_update(
        &mut self,
        device: &wgpu::Device,
        mesh_id: u64,
        vertices: &[u8],
        indices: &[u32],
    ) -> &GpuMesh {
        let hash = hash_mesh(vertices, indices);
        let needs_upload = self
            .meshes
            .get(&mesh_id)
            .map(|mesh| mesh.hash != hash || mesh.index_count != indices.len() as u32)
            .unwrap_or(true);

        if needs_upload {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grapho_mesh_vertices"),
                contents: vertices,
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grapho_mesh_indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            let mesh = GpuMesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                hash,
            };
            self.meshes.insert(mesh_id, mesh);
            self.uploads.fetch_add(1, Ordering::Relaxed);
        }

        self.meshes.get(&mesh_id).expect("mesh cache insert")
    }

    pub fn stats_snapshot(&self) -> GpuMeshCacheStats {
        GpuMeshCacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            uploads: self.uploads.load(Ordering::Relaxed),
            mesh_count: self.meshes.len() as u32,
        }
    }
}

fn hash_mesh(vertices: &[u8], indices: &[u32]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vertices.hash(&mut hasher);
    indices.hash(&mut hasher);
    hasher.finish()
}
