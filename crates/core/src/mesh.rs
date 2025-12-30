use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_positions_indices(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
        Self {
            positions,
            indices,
            normals: None,
            uvs: None,
        }
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let mut iter = self.positions.iter();
        let first = iter.next()?;
        let mut min = *first;
        let mut max = *first;

        for p in iter {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }

        Some(Aabb { min, max })
    }

    pub fn compute_normals(&mut self) -> bool {
        if !self.indices.len().is_multiple_of(3) || self.positions.is_empty() {
            return false;
        }

        let mut accum = vec![Vec3::ZERO; self.positions.len()];

        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            if i0 >= self.positions.len()
                || i1 >= self.positions.len()
                || i2 >= self.positions.len()
            {
                continue;
            }

            let p0 = Vec3::from(self.positions[i0]);
            let p1 = Vec3::from(self.positions[i1]);
            let p2 = Vec3::from(self.positions[i2]);
            let normal = (p1 - p0).cross(p2 - p0);
            accum[i0] += normal;
            accum[i1] += normal;
            accum[i2] += normal;
        }

        let normals = accum
            .into_iter()
            .map(|n| {
                let len = n.length();
                if len > 0.0 {
                    (n / len).to_array()
                } else {
                    [0.0, 1.0, 0.0]
                }
            })
            .collect();

        self.normals = Some(normals);
        true
    }

    pub fn transform(&mut self, matrix: Mat4) {
        for p in &mut self.positions {
            let v = matrix.transform_point3(Vec3::from(*p));
            *p = v.to_array();
        }

        if let Some(normals) = &mut self.normals {
            let normal_matrix = matrix.inverse().transpose();
            for n in normals {
                let v = normal_matrix.transform_vector3(Vec3::from(*n));
                let len = v.length();
                *n = if len > 0.0 {
                    (v / len).to_array()
                } else {
                    [0.0, 1.0, 0.0]
                };
            }
        }
    }

    pub fn merge(meshes: &[Mesh]) -> Mesh {
        let mut merged = Mesh::default();
        let mut vertex_offset = 0u32;
        let mut include_normals = true;
        let mut include_uvs = true;

        for mesh in meshes {
            include_normals &= mesh.normals.is_some();
            include_uvs &= mesh.uvs.is_some();
        }

        for mesh in meshes {
            merged.positions.extend_from_slice(&mesh.positions);
            merged
                .indices
                .extend(mesh.indices.iter().map(|i| i + vertex_offset));
            vertex_offset += mesh.positions.len() as u32;
        }

        if include_normals {
            let mut normals = Vec::new();
            for mesh in meshes {
                normals.extend_from_slice(mesh.normals.as_ref().unwrap());
            }
            merged.normals = Some(normals);
        }

        if include_uvs {
            let mut uvs = Vec::new();
            for mesh in meshes {
                uvs.extend_from_slice(mesh.uvs.as_ref().unwrap());
            }
            merged.uvs = Some(uvs);
        }

        merged
    }
}

pub fn make_box(size: [f32; 3]) -> Mesh {
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    let hz = size[2] * 0.5;

    let positions = vec![
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];

    let indices = vec![
        0, 2, 1, 0, 3, 2, // -Z
        4, 5, 6, 4, 6, 7, // +Z
        0, 1, 5, 0, 5, 4, // -Y
        2, 3, 7, 2, 7, 6, // +Y
        1, 2, 6, 1, 6, 5, // +X
        3, 0, 4, 3, 4, 7, // -X
    ];

    Mesh::with_positions_indices(positions, indices)
}

pub fn make_grid(size: [f32; 2], divisions: [u32; 2]) -> Mesh {
    let width = size[0].max(0.0);
    let depth = size[1].max(0.0);
    let div_x = divisions[0].max(1);
    let div_z = divisions[1].max(1);

    let step_x = width / div_x as f32;
    let step_z = depth / div_z as f32;
    let origin_x = -width * 0.5;
    let origin_z = -depth * 0.5;

    let mut positions = Vec::new();
    for z in 0..=div_z {
        for x in 0..=div_x {
            positions.push([
                origin_x + x as f32 * step_x,
                0.0,
                origin_z + z as f32 * step_z,
            ]);
        }
    }

    let mut indices = Vec::new();
    let stride = div_x + 1;
    for z in 0..div_z {
        for x in 0..div_x {
            let i0 = z * stride + x;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;

            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    Mesh::with_positions_indices(positions, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_for_simple_points() {
        let mesh =
            Mesh::with_positions_indices(vec![[1.0, -2.0, 0.5], [-3.0, 4.0, 2.0]], vec![0, 1, 0]);
        let bounds = mesh.bounds().expect("bounds");
        assert_eq!(bounds.min, [-3.0, -2.0, 0.5]);
        assert_eq!(bounds.max, [1.0, 4.0, 2.0]);
    }

    #[test]
    fn normals_for_triangle() {
        let mut mesh = Mesh::with_positions_indices(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![0, 1, 2],
        );
        assert!(mesh.compute_normals());
        let normals = mesh.normals.expect("normals");
        for n in normals {
            assert!((n[2] - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn merge_offsets_indices() {
        let mesh_a = Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0]], vec![0]);
        let mesh_b = Mesh::with_positions_indices(vec![[1.0, 0.0, 0.0]], vec![0]);
        let merged = Mesh::merge(&[mesh_a, mesh_b]);
        assert_eq!(merged.indices, vec![0, 1]);
    }

    #[test]
    fn box_has_expected_counts() {
        let mesh = make_box([2.0, 2.0, 2.0]);
        assert_eq!(mesh.positions.len(), 8);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn grid_has_expected_counts() {
        let mesh = make_grid([2.0, 2.0], [2, 3]);
        assert_eq!(mesh.positions.len(), (2 + 1) * (3 + 1));
        assert_eq!(mesh.indices.len(), 2 * 3 * 6);
    }
}
