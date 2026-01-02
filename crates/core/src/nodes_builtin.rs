use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue, PinDefinition, PinType};
use crate::mesh::{make_box, make_grid, make_uv_sphere, Mesh};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinNodeKind {
    Box,
    Grid,
    Sphere,
    File,
    Transform,
    CopyTransform,
    Merge,
    CopyToPoints,
    Scatter,
    Normal,
    Color,
    Noise,
    AttributeMath,
    ObjOutput,
    Output,
}

impl BuiltinNodeKind {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinNodeKind::Box => "Box",
            BuiltinNodeKind::Grid => "Grid",
            BuiltinNodeKind::Sphere => "Sphere",
            BuiltinNodeKind::File => "File",
            BuiltinNodeKind::Transform => "Transform",
            BuiltinNodeKind::CopyTransform => "Copy/Transform",
            BuiltinNodeKind::Merge => "Merge",
            BuiltinNodeKind::CopyToPoints => "Copy to Points",
            BuiltinNodeKind::Scatter => "Scatter",
            BuiltinNodeKind::Normal => "Normal",
            BuiltinNodeKind::Color => "Color",
            BuiltinNodeKind::Noise => "Noise/Mountain",
            BuiltinNodeKind::AttributeMath => "Attribute Math",
            BuiltinNodeKind::ObjOutput => "OBJ Output",
            BuiltinNodeKind::Output => "Output",
        }
    }
}

pub fn builtin_kind_from_name(name: &str) -> Option<BuiltinNodeKind> {
    match name {
        "Box" => Some(BuiltinNodeKind::Box),
        "Grid" => Some(BuiltinNodeKind::Grid),
        "Sphere" => Some(BuiltinNodeKind::Sphere),
        "File" => Some(BuiltinNodeKind::File),
        "Transform" => Some(BuiltinNodeKind::Transform),
        "Copy/Transform" => Some(BuiltinNodeKind::CopyTransform),
        "Merge" => Some(BuiltinNodeKind::Merge),
        "Copy to Points" => Some(BuiltinNodeKind::CopyToPoints),
        "Scatter" => Some(BuiltinNodeKind::Scatter),
        "Normal" => Some(BuiltinNodeKind::Normal),
        "Color" => Some(BuiltinNodeKind::Color),
        "Noise/Mountain" => Some(BuiltinNodeKind::Noise),
        "Attribute Math" => Some(BuiltinNodeKind::AttributeMath),
        "OBJ Output" => Some(BuiltinNodeKind::ObjOutput),
        "Output" => Some(BuiltinNodeKind::Output),
        _ => None,
    }
}

pub fn builtin_definitions() -> Vec<NodeDefinition> {
    vec![
        node_definition(BuiltinNodeKind::Box),
        node_definition(BuiltinNodeKind::Grid),
        node_definition(BuiltinNodeKind::Sphere),
        node_definition(BuiltinNodeKind::File),
        node_definition(BuiltinNodeKind::Transform),
        node_definition(BuiltinNodeKind::CopyTransform),
        node_definition(BuiltinNodeKind::Merge),
        node_definition(BuiltinNodeKind::CopyToPoints),
        node_definition(BuiltinNodeKind::Scatter),
        node_definition(BuiltinNodeKind::Normal),
        node_definition(BuiltinNodeKind::Color),
        node_definition(BuiltinNodeKind::Noise),
        node_definition(BuiltinNodeKind::AttributeMath),
        node_definition(BuiltinNodeKind::ObjOutput),
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
        BuiltinNodeKind::File => NodeDefinition {
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
        BuiltinNodeKind::CopyTransform => NodeDefinition {
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
        BuiltinNodeKind::Scatter => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Normal => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Color => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::Noise => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::AttributeMath => NodeDefinition {
            name: kind.name().to_string(),
            category: "Operators".to_string(),
            inputs: vec![mesh_in()],
            outputs: vec![mesh_out()],
        },
        BuiltinNodeKind::ObjOutput => NodeDefinition {
            name: kind.name().to_string(),
            category: "Outputs".to_string(),
            inputs: vec![mesh_in()],
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
        BuiltinNodeKind::File => {
            values.insert(
                "path".to_string(),
                ParamValue::String(r"C:\code\grapho\geo\pig.obj".to_string()),
            );
        }
        BuiltinNodeKind::Transform => {
            values.insert("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
            values.insert("pivot".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::CopyTransform => {
            values.insert("count".to_string(), ParamValue::Int(5));
            values.insert(
                "translate_step".to_string(),
                ParamValue::Vec3([1.0, 0.0, 0.0]),
            );
            values.insert(
                "rotate_step_deg".to_string(),
                ParamValue::Vec3([0.0, 0.0, 0.0]),
            );
            values.insert(
                "scale_step".to_string(),
                ParamValue::Vec3([0.0, 0.0, 0.0]),
            );
        }
        BuiltinNodeKind::Merge => {}
        BuiltinNodeKind::CopyToPoints => {
            values.insert("align_to_normals".to_string(), ParamValue::Bool(true));
            values.insert("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
            values.insert("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
        }
        BuiltinNodeKind::Scatter => {
            values.insert("count".to_string(), ParamValue::Int(200));
            values.insert("seed".to_string(), ParamValue::Int(1));
        }
        BuiltinNodeKind::Normal => {
            values.insert("threshold_deg".to_string(), ParamValue::Float(60.0));
        }
        BuiltinNodeKind::Color => {
            values.insert("color".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
            values.insert("domain".to_string(), ParamValue::Int(0));
        }
        BuiltinNodeKind::Noise => {
            values.insert("amplitude".to_string(), ParamValue::Float(0.2));
            values.insert("frequency".to_string(), ParamValue::Float(1.0));
            values.insert("seed".to_string(), ParamValue::Int(1));
            values.insert("offset".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0]));
        }
        BuiltinNodeKind::AttributeMath => {
            values.insert("attr".to_string(), ParamValue::String("Cd".to_string()));
            values.insert("result".to_string(), ParamValue::String("Cd".to_string()));
            values.insert("domain".to_string(), ParamValue::Int(0));
            values.insert("op".to_string(), ParamValue::Int(0));
            values.insert("value_f".to_string(), ParamValue::Float(0.0));
            values.insert("value_v3".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0]));
        }
        BuiltinNodeKind::ObjOutput => {
            values.insert("path".to_string(), ParamValue::String("output.obj".to_string()));
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
        BuiltinNodeKind::File => {
            let path = param_string(params, "path", "");
            if path.trim().is_empty() {
                return Err("File node requires a path".to_string());
            }
            load_obj_mesh(path)
        }
        BuiltinNodeKind::Transform => {
            let input = inputs
                .first()
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
        BuiltinNodeKind::CopyTransform => {
            let input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Copy/Transform requires a mesh input".to_string())?;
            let count = param_int(params, "count", 1).max(0) as usize;
            if count == 0 {
                return Ok(Mesh::default());
            }
            let translate_step = param_vec3(params, "translate_step", [0.0, 0.0, 0.0]);
            let rotate_step = param_vec3(params, "rotate_step_deg", [0.0, 0.0, 0.0]);
            let scale_step = param_vec3(params, "scale_step", [0.0, 0.0, 0.0]);

            let mut copies = Vec::with_capacity(count);
            for i in 0..count {
                let factor = i as f32;
                let translate = Vec3::from(translate_step) * factor;
                let rot = Vec3::from(rotate_step) * factor * std::f32::consts::PI / 180.0;
                let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
                let scale = Vec3::new(1.0, 1.0, 1.0) + Vec3::from(scale_step) * factor;
                let matrix = Mat4::from_scale_rotation_translation(scale, quat, translate);
                let mut mesh = input.clone();
                mesh.transform(matrix);
                copies.push(mesh);
            }
            Ok(Mesh::merge(&copies))
        }
        BuiltinNodeKind::Merge => {
            if inputs.is_empty() {
                return Err("Merge requires at least one mesh input".to_string());
            }
            Ok(Mesh::merge(inputs))
        }
        BuiltinNodeKind::CopyToPoints => {
            let source = inputs
                .first()
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
                    let normal = normals.get(idx).copied().unwrap_or([0.0, 1.0, 0.0]);
                    let normal = Vec3::from(normal);
                    if normal.length_squared() > 0.0001 {
                        let align = Quat::from_rotation_arc(Vec3::Y, normal.normalize());
                        rotation = align * user_quat;
                    }
                }
                let matrix = Mat4::from_scale_rotation_translation(
                    scale,
                    rotation,
                    Vec3::from(*pos) + translate,
                );
                let mut mesh = source.clone();
                mesh.transform(matrix);
                copies.push(mesh);
            }
            Ok(Mesh::merge(&copies))
        }
        BuiltinNodeKind::Scatter => {
            let input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Scatter requires a mesh input".to_string())?;
            let count = param_int(params, "count", 200).max(0) as usize;
            let seed = param_int(params, "seed", 1) as u32;
            scatter_points(&input, count, seed)
        }
        BuiltinNodeKind::Normal => {
            let mut input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Normal requires a mesh input".to_string())?;
            let threshold = param_float(params, "threshold_deg", 60.0).clamp(0.0, 180.0);
            if !input.compute_normals_with_threshold(threshold) {
                return Err("Normal node requires triangle mesh input".to_string());
            }
            Ok(input)
        }
        BuiltinNodeKind::Color => {
            let mut input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Color requires a mesh input".to_string())?;
            let color = param_vec3(params, "color", [1.0, 1.0, 1.0]);
            let domain = match param_int(params, "domain", 0).clamp(0, 3) {
                0 => AttributeDomain::Point,
                1 => AttributeDomain::Vertex,
                2 => AttributeDomain::Primitive,
                _ => AttributeDomain::Detail,
            };
            let count = input.attribute_domain_len(domain);
            let values = vec![color; count];
            input
                .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
                .map_err(|err| format!("Color attribute error: {:?}", err))?;
            Ok(input)
        }
        BuiltinNodeKind::Noise => {
            let mut input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Noise/Mountain requires a mesh input".to_string())?;
            let amplitude = param_float(params, "amplitude", 0.2);
            let frequency = param_float(params, "frequency", 1.0).max(0.0);
            let seed = param_int(params, "seed", 1) as u32;
            let offset = Vec3::from(param_vec3(params, "offset", [0.0, 0.0, 0.0]));

            if input.normals.is_none() {
                let _ = input.compute_normals();
            }
            let normals = input
                .normals
                .clone()
                .ok_or_else(|| "Noise/Mountain requires point normals".to_string())?;

            for (pos, normal) in input.positions.iter_mut().zip(normals.iter()) {
                let p = Vec3::from(*pos) * frequency + offset;
                let n = fractal_noise(p, seed);
                let displacement = Vec3::from(*normal) * (n * amplitude);
                let next = Vec3::from(*pos) + displacement;
                *pos = next.to_array();
            }

            Ok(input)
        }
        BuiltinNodeKind::AttributeMath => {
            let mut input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "Attribute Math requires a mesh input".to_string())?;
            let attr = param_string(params, "attr", "Cd");
            let result = param_string(params, "result", attr);
            let domain = match param_int(params, "domain", 0).clamp(0, 3) {
                0 => AttributeDomain::Point,
                1 => AttributeDomain::Vertex,
                2 => AttributeDomain::Primitive,
                _ => AttributeDomain::Detail,
            };
            let op = param_int(params, "op", 0).clamp(0, 3);
            let value_f = param_float(params, "value_f", 0.0);
            let value_v3 = param_vec3(params, "value_v3", [0.0, 0.0, 0.0]);

            let attr_ref = input
                .attribute(domain, attr)
                .ok_or_else(|| format!("Attribute '{}' not found", attr))?;
            match attr_ref {
                crate::attributes::AttributeRef::Float(values) => {
                    let mut next = Vec::with_capacity(values.len());
                    for &v in values {
                        next.push(apply_op_f(v, value_f, op));
                    }
                    input
                        .set_attribute(domain, result, AttributeStorage::Float(next))
                        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
                }
                crate::attributes::AttributeRef::Int(values) => {
                    let mut next = Vec::with_capacity(values.len());
                    let value_i = value_f.round() as i32;
                    for &v in values {
                        next.push(apply_op_i(v, value_i, op));
                    }
                    input
                        .set_attribute(domain, result, AttributeStorage::Int(next))
                        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
                }
                crate::attributes::AttributeRef::Vec2(values) => {
                    let mut next = Vec::with_capacity(values.len());
                    for &v in values {
                        next.push([
                            apply_op_f(v[0], value_f, op),
                            apply_op_f(v[1], value_f, op),
                        ]);
                    }
                    input
                        .set_attribute(domain, result, AttributeStorage::Vec2(next))
                        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
                }
                crate::attributes::AttributeRef::Vec3(values) => {
                    let mut next = Vec::with_capacity(values.len());
                    for &v in values {
                        next.push([
                            apply_op_f(v[0], value_v3[0], op),
                            apply_op_f(v[1], value_v3[1], op),
                            apply_op_f(v[2], value_v3[2], op),
                        ]);
                    }
                    input
                        .set_attribute(domain, result, AttributeStorage::Vec3(next))
                        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
                }
                crate::attributes::AttributeRef::Vec4(values) => {
                    let mut next = Vec::with_capacity(values.len());
                    for &v in values {
                        next.push([
                            apply_op_f(v[0], value_f, op),
                            apply_op_f(v[1], value_f, op),
                            apply_op_f(v[2], value_f, op),
                            apply_op_f(v[3], value_f, op),
                        ]);
                    }
                    input
                        .set_attribute(domain, result, AttributeStorage::Vec4(next))
                        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
                }
            }
            Ok(input)
        }
        BuiltinNodeKind::ObjOutput => {
            let input = inputs
                .first()
                .cloned()
                .ok_or_else(|| "OBJ Output requires a mesh input".to_string())?;
            let path = param_string(params, "path", "output.obj");
            if path.trim().is_empty() {
                return Err("OBJ Output requires a path".to_string());
            }
            write_obj(path, &input)?;
            Ok(input)
        }
        BuiltinNodeKind::Output => {
            let input = inputs
                .first()
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

fn param_string<'a>(params: &'a NodeParams, key: &str, default: &'a str) -> &'a str {
    params
        .values
        .get(key)
        .and_then(|value| match value {
            ParamValue::String(v) => Some(v.as_str()),
            _ => None,
        })
        .unwrap_or(default)
}

fn apply_op_f(value: f32, rhs: f32, op: i32) -> f32 {
    match op {
        0 => value + rhs,
        1 => value - rhs,
        2 => value * rhs,
        3 => {
            if rhs.abs() < 1.0e-6 {
                value
            } else {
                value / rhs
            }
        }
        _ => value,
    }
}

fn apply_op_i(value: i32, rhs: i32, op: i32) -> i32 {
    match op {
        0 => value.saturating_add(rhs),
        1 => value.saturating_sub(rhs),
        2 => value.saturating_mul(rhs),
        3 => {
            if rhs == 0 {
                value
            } else {
                value / rhs
            }
        }
        _ => value,
    }
}

fn fractal_noise(p: Vec3, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    for _ in 0..3 {
        value += value_noise(p * freq, seed) * amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    value
}

fn value_noise(p: Vec3, seed: u32) -> f32 {
    let base = p.floor();
    let frac = p - base;
    let f = frac * frac * (Vec3::splat(3.0) - 2.0 * frac);

    let x0 = base.x as i32;
    let y0 = base.y as i32;
    let z0 = base.z as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;

    let c000 = hash3(x0, y0, z0, seed);
    let c100 = hash3(x1, y0, z0, seed);
    let c010 = hash3(x0, y1, z0, seed);
    let c110 = hash3(x1, y1, z0, seed);
    let c001 = hash3(x0, y0, z1, seed);
    let c101 = hash3(x1, y0, z1, seed);
    let c011 = hash3(x0, y1, z1, seed);
    let c111 = hash3(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, f.x);
    let x10 = lerp(c010, c110, f.x);
    let x01 = lerp(c001, c101, f.x);
    let x11 = lerp(c011, c111, f.x);
    let y0 = lerp(x00, x10, f.y);
    let y1 = lerp(x01, x11, f.y);
    lerp(y0, y1, f.z) * 2.0 - 1.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = x as u32;
    h ^= (y as u32).wrapping_mul(374761393);
    h = h.rotate_left(13);
    h ^= (z as u32).wrapping_mul(668265263);
    h = h.rotate_left(17);
    h ^= seed.wrapping_mul(2246822519);
    h = h.wrapping_mul(3266489917);
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    (h as f32) / (u32::MAX as f32)
}

#[cfg(target_arch = "wasm32")]
fn load_obj_mesh(_path: &str) -> Result<Mesh, String> {
    Err("File node is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn load_obj_mesh(path: &str) -> Result<Mesh, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let (models, _) = {
        let options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };
        tobj::load_obj(path, &options).map_err(|err| format!("OBJ load failed: {err}"))?
    };

    if models.is_empty() {
        return Err("OBJ has no geometry".to_string());
    }

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut include_normals = true;
    let mut include_uvs = true;
    let mut vertex_offset = 0u32;

    for model in models {
        let mesh = &model.mesh;
        if mesh.positions.len() % 3 != 0 {
            return Err("OBJ has malformed positions".to_string());
        }
        let vertex_count = mesh.positions.len() / 3;

        positions.extend(mesh.positions.chunks_exact(3).map(|v| [v[0], v[1], v[2]]));
        indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));
        vertex_offset += vertex_count as u32;

        if mesh.normals.len() == mesh.positions.len() {
            normals.extend(mesh.normals.chunks_exact(3).map(|n| [n[0], n[1], n[2]]));
        } else {
            include_normals = false;
        }

        if mesh.texcoords.len() / 2 == vertex_count {
            uvs.extend(mesh.texcoords.chunks_exact(2).map(|t| [t[0], t[1]]));
        } else {
            include_uvs = false;
        }
    }

    let mut mesh = Mesh::with_positions_indices(positions, indices);
    if include_normals && !normals.is_empty() {
        mesh.normals = Some(normals);
    }
    if include_uvs && !uvs.is_empty() {
        let corner_uvs: Vec<[f32; 2]> = mesh
            .indices
            .iter()
            .filter_map(|idx| uvs.get(*idx as usize).copied())
            .collect();
        if corner_uvs.len() == mesh.indices.len() {
            let _ = mesh.set_attribute(
                AttributeDomain::Vertex,
                "uv",
                AttributeStorage::Vec2(corner_uvs),
            );
        }
        mesh.uvs = Some(uvs);
    }

    if mesh.normals.is_none() && mesh.corner_normals.is_none() {
        mesh.compute_normals();
    }

    Ok(mesh)
}

#[cfg(target_arch = "wasm32")]
fn write_obj(_path: &str, _mesh: &Mesh) -> Result<(), String> {
    Err("OBJ Output is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_obj(path: &str, mesh: &Mesh) -> Result<(), String> {
    use std::io::Write;

    let mut file = std::fs::File::create(path).map_err(|err| err.to_string())?;
    for p in &mesh.positions {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2]).map_err(|err| err.to_string())?;
    }

    let has_uv = mesh
        .uvs
        .as_ref()
        .map_or(false, |uvs| uvs.len() == mesh.positions.len());
    if let Some(uvs) = &mesh.uvs {
        if has_uv {
            for uv in uvs {
                writeln!(file, "vt {} {}", uv[0], uv[1]).map_err(|err| err.to_string())?;
            }
        }
    }

    let has_normals = mesh
        .normals
        .as_ref()
        .map_or(false, |normals| normals.len() == mesh.positions.len());
    if let Some(normals) = &mesh.normals {
        if has_normals {
            for n in normals {
                writeln!(file, "vn {} {} {}", n[0], n[1], n[2]).map_err(|err| err.to_string())?;
            }
        }
    }

    if !mesh.indices.is_empty() {
        for tri in mesh.indices.chunks_exact(3) {
            let a = tri[0] + 1;
            let b = tri[1] + 1;
            let c = tri[2] + 1;
            if has_uv && has_normals {
                writeln!(file, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}")
                    .map_err(|err| err.to_string())?;
            } else if has_uv {
                writeln!(file, "f {a}/{a} {b}/{b} {c}/{c}")
                    .map_err(|err| err.to_string())?;
            } else if has_normals {
                writeln!(file, "f {a}//{a} {b}//{b} {c}//{c}")
                    .map_err(|err| err.to_string())?;
            } else {
                writeln!(file, "f {a} {b} {c}").map_err(|err| err.to_string())?;
            }
        }
    }
    Ok(())
}

fn scatter_points(input: &Mesh, count: usize, seed: u32) -> Result<Mesh, String> {
    if count == 0 {
        return Ok(Mesh::default());
    }
    if !input.indices.len().is_multiple_of(3) || input.positions.is_empty() {
        return Err("Scatter requires a triangle mesh input".to_string());
    }

    let mut areas = Vec::new();
    let mut total = 0.0f32;
    for tri in input.indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= input.positions.len() || i1 >= input.positions.len() || i2 >= input.positions.len()
        {
            areas.push(total);
            continue;
        }
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        total += area.max(0.0);
        areas.push(total);
    }

    if total <= 0.0 {
        return Err("Scatter requires non-degenerate triangles".to_string());
    }

    let mut rng = XorShift32::new(seed);
    let mut positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);

    for _ in 0..count {
        let sample = rng.next_f32() * total;
        let tri_index = find_area_index(&areas, sample);
        let tri = input
            .indices
            .get(tri_index * 3..tri_index * 3 + 3)
            .ok_or_else(|| "scatter triangle index out of range".to_string())?;
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);

        let r1 = rng.next_f32().clamp(0.0, 1.0);
        let r2 = rng.next_f32().clamp(0.0, 1.0);
        let sqrt_r1 = r1.sqrt();
        let u = 1.0 - sqrt_r1;
        let v = r2 * sqrt_r1;
        let w = 1.0 - u - v;
        let point = p0 * u + p1 * v + p2 * w;

        let normal = (p1 - p0).cross(p2 - p0);
        let normal = if normal.length_squared() > 0.0 {
            normal.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };

        positions.push(point.to_array());
        normals.push(normal);
    }

    Ok(Mesh {
        positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
    })
}

fn find_area_index(cumulative: &[f32], sample: f32) -> usize {
    let mut lo = 0usize;
    let mut hi = cumulative.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sample < cumulative[mid] {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo.min(cumulative.len().saturating_sub(1))
}

struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    fn new(seed: u32) -> Self {
        let seed = if seed == 0 { 0x12345678 } else { seed };
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        let value = self.next_u32();
        value as f32 / u32::MAX as f32
    }
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

    #[test]
    fn scatter_produces_points() {
        let params = NodeParams {
            values: BTreeMap::from([
                ("count".to_string(), ParamValue::Int(12)),
                ("seed".to_string(), ParamValue::Int(3)),
            ]),
        };
        let input = make_box([1.0, 1.0, 1.0]);
        let mesh = compute_mesh_node(BuiltinNodeKind::Scatter, &params, &[input]).unwrap();
        assert_eq!(mesh.positions.len(), 12);
        assert!(mesh.indices.is_empty());
        assert_eq!(mesh.normals.as_ref().map(|n| n.len()), Some(12));
    }

    #[test]
    fn normal_recomputes_normals() {
        let mut input = make_box([1.0, 1.0, 1.0]);
        input.normals = None;
        let mesh =
            compute_mesh_node(BuiltinNodeKind::Normal, &NodeParams::default(), &[input]).unwrap();
        assert!(mesh.normals.is_some());
    }
}
