use crate::scene::RenderMesh;
use egui_wgpu::wgpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
}

pub(crate) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LineVertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
}

pub(crate) const LINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

pub(crate) struct CubeMesh {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
    pub(crate) bounds_min: [f32; 3],
    pub(crate) bounds_max: [f32; 3],
}

pub(crate) fn cube_mesh() -> CubeMesh {
    let positions = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];

    let faces = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
        ([2, 3, 7, 6], [0.0, 1.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
        ([3, 0, 4, 7], [-1.0, 0.0, 0.0]),
    ];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (face, normal) in faces {
        let base_index = vertices.len() as u32;
        for &idx in &face {
            vertices.push(Vertex {
                position: positions[idx],
                normal,
            });
        }
        indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    let (bounds_min, bounds_max) = mesh_bounds(&vertices);

    CubeMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}

fn mesh_bounds(vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in vertices {
        for i in 0..3 {
            min[i] = min[i].min(vertex.position[i]);
            max[i] = max[i].max(vertex.position[i]);
        }
    }
    (min, max)
}

pub(crate) fn bounds_from_positions(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    if positions.is_empty() {
        return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in positions {
        for i in 0..3 {
            min[i] = min[i].min(position[i]);
            max[i] = max[i].max(position[i]);
        }
    }
    (min, max)
}

pub(crate) fn build_vertices(mesh: &RenderMesh) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(mesh.positions.len());
    let fallback = [0.0, 1.0, 0.0];
    for (index, position) in mesh.positions.iter().enumerate() {
        let normal = mesh.normals.get(index).copied().unwrap_or(fallback);
        vertices.push(Vertex {
            position: *position,
            normal,
        });
    }
    vertices
}

pub(crate) fn normals_vertices(vertices: &[Vertex], length: f32) -> Vec<LineVertex> {
    let mut lines = Vec::with_capacity(vertices.len() * 2);
    let color = [1.0, 0.85, 0.3];
    for vertex in vertices {
        let start = vertex.position;
        let end = [
            vertex.position[0] + vertex.normal[0] * length,
            vertex.position[1] + vertex.normal[1] * length,
            vertex.position[2] + vertex.normal[2] * length,
        ];
        lines.push(LineVertex {
            position: start,
            color,
        });
        lines.push(LineVertex {
            position: end,
            color,
        });
    }
    lines
}

pub(crate) fn bounds_vertices(min: [f32; 3], max: [f32; 3]) -> Vec<LineVertex> {
    let color = [0.85, 0.85, 0.9];
    let [min_x, min_y, min_z] = min;
    let [max_x, max_y, max_z] = max;

    let corners = [
        [min_x, min_y, min_z],
        [max_x, min_y, min_z],
        [max_x, max_y, min_z],
        [min_x, max_y, min_z],
        [min_x, min_y, max_z],
        [max_x, min_y, max_z],
        [max_x, max_y, max_z],
        [min_x, max_y, max_z],
    ];

    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    let mut lines = Vec::with_capacity(edges.len() * 2);
    for (a, b) in edges {
        lines.push(LineVertex {
            position: corners[a],
            color,
        });
        lines.push(LineVertex {
            position: corners[b],
            color,
        });
    }
    lines
}

pub(crate) fn grid_and_axes() -> (Vec<LineVertex>, Vec<LineVertex>) {
    let grid_size = 10.0;
    let divisions = 20;
    let step = grid_size / divisions as f32;
    let half = grid_size * 0.5;

    let grid_color = [0.25, 0.25, 0.25];
    let mut grid = Vec::new();

    for i in 0..=divisions {
        let offset = -half + i as f32 * step;
        grid.push(LineVertex {
            position: [offset, 0.0, -half],
            color: grid_color,
        });
        grid.push(LineVertex {
            position: [offset, 0.0, half],
            color: grid_color,
        });

        grid.push(LineVertex {
            position: [-half, 0.0, offset],
            color: grid_color,
        });
        grid.push(LineVertex {
            position: [half, 0.0, offset],
            color: grid_color,
        });
    }

    let axis_len = 2.5;
    let axes = vec![
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        LineVertex {
            position: [axis_len, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        LineVertex {
            position: [0.0, axis_len, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.1, 0.3, 1.0],
        },
        LineVertex {
            position: [0.0, 0.0, axis_len],
            color: [0.1, 0.3, 1.0],
        },
    ];

    (grid, axes)
}
