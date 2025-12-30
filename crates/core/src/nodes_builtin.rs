use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue, PinDefinition, PinType};
use crate::mesh::{make_box, make_grid, make_uv_sphere, Mesh};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinNodeKind {
    Box,
    Grid,
    Sphere,
    Transform,
    Merge,
    CopyToPoints,
    Output,
}

impl BuiltinNodeKind {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinNodeKind::Box => "Box",
            BuiltinNodeKind::Grid => "Grid",
            BuiltinNodeKind::Sphere => "Sphere",
            BuiltinNodeKind::Transform => "Transform",
            BuiltinNodeKind::Merge => "Merge",
            BuiltinNodeKind::CopyToPoints => "Copy to Points",
            BuiltinNodeKind::Output => "Output",
        }
    }
}

pub fn builtin_kind_from_name(name: &str) -> Option<BuiltinNodeKind> {
    match name {
        "Box" => Some(BuiltinNodeKind::Box),
        "Grid" => Some(BuiltinNodeKind::Grid),
        "Sphere" => Some(BuiltinNodeKind::Sphere),
        "Transform" => Some(BuiltinNodeKind::Transform),
        "Merge" => Some(BuiltinNodeKind::Merge),
        "Copy to Points" => Some(BuiltinNodeKind::CopyToPoints),
        "Output" => Some(BuiltinNodeKind::Output),
        _ => None,
    }
}

pub fn builtin_definitions() -> Vec<NodeDefinition> {
    vec![
        node_definition(BuiltinNodeKind::Box),
        node_definition(BuiltinNodeKind::Grid),
        node_definition(BuiltinNodeKind::Sphere),
        node_definition(BuiltinNodeKind::Transform),
        node_definition(BuiltinNodeKind::Merge),
        node_definition(BuiltinNodeKind::CopyToPoints),
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
        BuiltinNodeKind::Sphere => NodeDefinition {
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
        BuiltinNodeKind::CopyToPoints => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![
                PinDefinition {
                    name: "source".to_string(),
                    pin_type: PinType::Mesh,
                },
                PinDefinition {
                    name: "template".to_string(),
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
            values.insert("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::Grid => {
            values.insert("size".to_string(), ParamValue::Vec2([2.0, 2.0]));
            values.insert("rows".to_string(), ParamValue::Int(10));
            values.insert("cols".to_string(), ParamValue::Int(10));
            values.insert("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::Sphere => {
            values.insert("radius".to_string(), ParamValue::Float(1.0));
            values.insert("rows".to_string(), ParamValue::Int(16));
            values.insert("cols".to_string(), ParamValue::Int(32));
            values.insert("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::Transform => {
            values.insert("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
            values.insert("pivot".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::Merge => {}
        BuiltinNodeKind::CopyToPoints => {
            values.insert("align_to_normals".to_string(), ParamValue::Bool(true));
            values.insert("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
        }
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
            let center = param_vec3(params, "center", [0.0, 0.0, 0.0]);
            let mut mesh = make_box(size);
            if center != [0.0, 0.0, 0.0] {
                mesh.transform(Mat4::from_translation(Vec3::from(center)));
            }
            if mesh.normals.is_none() {
                mesh.compute_normals();
            }
            Ok(mesh)
        }
        BuiltinNodeKind::Grid => {
            let size = param_vec2(params, "size", [2.0, 2.0]);
            let rows = param_int(params, "rows", 10).max(1) as u32;
            let cols = param_int(params, "cols", 10).max(1) as u32;
            let center = param_vec3(params, "center", [0.0, 0.0, 0.0]);
            let divisions = [cols, rows];
            let mut mesh = make_grid(size, divisions);
            if center != [0.0, 0.0, 0.0] {
                mesh.transform(Mat4::from_translation(Vec3::from(center)));
            }
            if mesh.normals.is_none() {
                mesh.compute_normals();
            }
            Ok(mesh)
        }
        BuiltinNodeKind::Sphere => {
            let radius = param_float(params, "radius", 1.0).max(0.0);
            let rows = param_int(params, "rows", 16).max(3) as u32;
            let cols = param_int(params, "cols", 32).max(3) as u32;
            let center = param_vec3(params, "center", [0.0, 0.0, 0.0]);
            let mut mesh = make_uv_sphere(radius, rows, cols);
            if center != [0.0, 0.0, 0.0] {
                mesh.transform(Mat4::from_translation(Vec3::from(center)));
            }
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
            let pivot = param_vec3(params, "pivot", [0.0, 0.0, 0.0]);

            let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
            let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
            let matrix = Mat4::from_translation(Vec3::from(translate))
                * Mat4::from_translation(Vec3::from(pivot))
                * Mat4::from_quat(quat)
                * Mat4::from_scale(Vec3::from(scale))
                * Mat4::from_translation(-Vec3::from(pivot));
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
        BuiltinNodeKind::CopyToPoints => {
            let source = inputs
                .get(0)
                .cloned()
                .ok_or_else(|| "Copy to Points requires a source mesh".to_string())?;
            let template = inputs
                .get(1)
                .cloned()
                .ok_or_else(|| "Copy to Points requires a template mesh".to_string())?;

            if template.positions.is_empty() {
                return Err("Copy to Points requires template points".to_string());
            }

            let align_to_normals = param_bool(params, "align_to_normals", true);
            let translate = param_vec3(params, "translate", [0.0, 0.0, 0.0]);
            let rotate_deg = param_vec3(params, "rotate_deg", [0.0, 0.0, 0.0]);
            let scale = param_vec3(params, "scale", [1.0, 1.0, 1.0]);

            let mut normals = template.normals.clone().unwrap_or_default();
            if align_to_normals && normals.len() != template.positions.len() {
                let mut temp = template.clone();
                if temp.normals.is_none() {
                    temp.compute_normals();
                }
                normals = temp.normals.unwrap_or_default();
            }

            let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
            let user_quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
            let scale = Vec3::from(scale);
            let translate = Vec3::from(translate);

            let mut copies = Vec::with_capacity(template.positions.len());
            for (idx, pos) in template.positions.iter().enumerate() {
                let mut rotation = user_quat;
                if align_to_normals {
                    let normal = normals
                        .get(idx)
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]);
                    let normal = Vec3::from(normal);
                    if normal.length_squared() > 0.0001 {
                        let align = Quat::from_rotation_arc(Vec3::Y, normal.normalize());
                        rotation = align * user_quat;
                    }
                }
                let matrix =
                    Mat4::from_scale_rotation_translation(scale, rotation, Vec3::from(*pos) + translate);
                let mut mesh = source.clone();
                mesh.transform(matrix);
                copies.push(mesh);
            }
            Ok(Mesh::merge(&copies))
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

fn param_float(params: &NodeParams, key: &str, default: f32) -> f32 {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Float(v) => Some(*v),
            ParamValue::Int(v) => Some(*v as f32),
            _ => None,
        })
        .unwrap_or(default)
}

fn param_int(params: &NodeParams, key: &str, default: i32) -> i32 {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Int(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(default)
}

fn param_bool(params: &NodeParams, key: &str, default: bool) -> bool {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::Bool(v) => Some(*v),
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
