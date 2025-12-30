use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue, PinDefinition, PinType};
use crate::mesh::{make_box, make_grid, Mesh};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinNodeKind {
    Box,
    Grid,
    Transform,
    Merge,
    Output,
}

impl BuiltinNodeKind {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinNodeKind::Box => "Box",
            BuiltinNodeKind::Grid => "Grid",
            BuiltinNodeKind::Transform => "Transform",
            BuiltinNodeKind::Merge => "Merge",
            BuiltinNodeKind::Output => "Output",
        }
    }
}

pub fn builtin_kind_from_name(name: &str) -> Option<BuiltinNodeKind> {
    match name {
        "Box" => Some(BuiltinNodeKind::Box),
        "Grid" => Some(BuiltinNodeKind::Grid),
        "Transform" => Some(BuiltinNodeKind::Transform),
        "Merge" => Some(BuiltinNodeKind::Merge),
        "Output" => Some(BuiltinNodeKind::Output),
        _ => None,
    }
}

pub fn builtin_definitions() -> Vec<NodeDefinition> {
    vec![
        node_definition(BuiltinNodeKind::Box),
        node_definition(BuiltinNodeKind::Grid),
        node_definition(BuiltinNodeKind::Transform),
        node_definition(BuiltinNodeKind::Merge),
        node_definition(BuiltinNodeKind::Output),
    ]
}

pub fn node_definition(kind: BuiltinNodeKind) -> NodeDefinition {
    let mesh_in = || PinDefinition {
        name: "in".to_string(),
        pin_type: PinType::Mesh,
    };
    let mesh_out = || PinDefinition {
        name: "out".to_string(),
        pin_type: PinType::Mesh,
    };

    match kind {
        BuiltinNodeKind::Box => NodeDefinition {
            name: kind.name().to_string(),
            category: "Sources".to_string(),
            inputs: Vec::new(),
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Grid => NodeDefinition {
            name: kind.name().to_string(),
            category: "Sources".to_string(),
            inputs: Vec::new(),
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Transform => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Merge => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![
                PinDefinition {
                    name: "a".to_string(),
                    pin_type: PinType::Mesh,
                },
                PinDefinition {
                    name: "b".to_string(),
                    pin_type: PinType::Mesh,
                },
            ],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Output => NodeDefinition {
            name: kind.name().to_string(),
            category: "Outputs".to_string(),
            inputs: vec![mesh_in()],
            outputs: Vec::new(),
        },
    }
}

pub fn default_params(kind: BuiltinNodeKind) -> NodeParams {
    let mut values = BTreeMap::new();
    match kind {
        BuiltinNodeKind::Box => {
            values.insert("size".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
        }
        BuiltinNodeKind::Grid => {
            values.insert("size".to_string(), ParamValue::Vec2([2.0, 2.0]));
            values.insert("divisions".to_string(), ParamValue::Vec2([10.0, 10.0]));
        }
        BuiltinNodeKind::Transform => {
            values.insert("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
        }
        BuiltinNodeKind::Merge => {}
        BuiltinNodeKind::Output => {}
    }

    NodeParams { values }
}

pub fn compute_mesh_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[Mesh],
) -> Result<Mesh, String> {
    match kind {
        BuiltinNodeKind::Box => {
            let size = param_vec3(params, "size", [1.0, 1.0, 1.0]);
            let mut mesh = make_box(size);
            if mesh.normals.is_none() {
                mesh.compute_normals();
            }
            Ok(mesh)
        }
        BuiltinNodeKind::Grid => {
            let size = param_vec2(params, "size", [2.0, 2.0]);
            let div = param_vec2(params, "divisions", [10.0, 10.0]);
            let divisions = [div[0].max(1.0) as u32, div[1].max(1.0) as u32];
            let mut mesh = make_grid(size, divisions);
            if mesh.normals.is_none() {
                mesh.compute_normals();
            }
            Ok(mesh)
        }
        BuiltinNodeKind::Transform => {
            let input = inputs
                .get(0)
                .cloned()
                .ok_or_else(|| "Transform requires a mesh input".to_string())?;
            let translate = param_vec3(params, "translate", [0.0, 0.0, 0.0]);
            let rotate_deg = param_vec3(params, "rotate_deg", [0.0, 0.0, 0.0]);
            let scale = param_vec3(params, "scale", [1.0, 1.0, 1.0]);

            let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
            let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
            let matrix = Mat4::from_scale_rotation_translation(
                Vec3::from(scale),
                quat,
                Vec3::from(translate),
            );
            let mut mesh = input;
            mesh.transform(matrix);
            Ok(mesh)
        }
        BuiltinNodeKind::Merge => {
            if inputs.is_empty() {
                return Err("Merge requires at least one mesh input".to_string());
            }
            Ok(Mesh::merge(inputs))
        }
        BuiltinNodeKind::Output => {
            let input = inputs
                .get(0)
                .cloned()
                .ok_or_else(|| "Output requires a mesh input".to_string())?;
            Ok(input)
        }
    }
}

fn param_vec2(params: &NodeParams, key: &str, default: [f32; 2]) -> [f32; 2] {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Vec2(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(default)
}

fn param_vec3(params: &NodeParams, key: &str, default: [f32; 3]) -> [f32; 3] {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Vec3(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_applies_scale() {
        let params = NodeParams {
            values: BTreeMap::from([("scale".to_string(), ParamValue::Vec3([2.0, 2.0, 2.0]))]),
        };
        let input = make_box([1.0, 1.0, 1.0]);
        let mesh = compute_mesh_node(BuiltinNodeKind::Transform, &params, &[input]).unwrap();
        let bounds = mesh.bounds().expect("bounds");
        assert!((bounds.max[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn merge_combines_meshes() {
        let a = make_box([1.0, 1.0, 1.0]);
        let b = make_box([2.0, 2.0, 2.0]);
        let mesh =
            compute_mesh_node(BuiltinNodeKind::Merge, &NodeParams::default(), &[a, b]).unwrap();
        assert!(mesh.positions.len() >= 16);
    }
}
