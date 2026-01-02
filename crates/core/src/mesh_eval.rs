use std::collections::BTreeMap;

use crate::eval::{evaluate_from_with, EvalReport, EvalState};
use crate::graph::{Graph, GraphError, NodeId};
use crate::mesh::Mesh;
use crate::nodes_builtin::{builtin_kind_from_name, compute_mesh_node};

#[derive(Debug, Default)]
pub struct MeshEvalState {
    pub eval: EvalState,
    outputs: BTreeMap<NodeId, Mesh>,
}

#[derive(Debug)]
pub struct MeshEvalResult {
    pub report: EvalReport,
    pub output: Option<Mesh>,
}

impl MeshEvalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mesh_for_node(&self, node_id: NodeId) -> Option<&Mesh> {
        self.outputs.get(&node_id)
    }
}

pub fn evaluate_mesh_graph(
    graph: &Graph,
    output: NodeId,
    state: &mut MeshEvalState,
) -> Result<MeshEvalResult, GraphError> {
    let outputs = &mut state.outputs;
    let report = evaluate_from_with(graph, output, &mut state.eval, |node_id, params| {
        let node = graph
            .node(node_id)
            .ok_or_else(|| "missing node".to_string())?;
        let kind = builtin_kind_from_name(&node.name)
            .ok_or_else(|| format!("unknown node type {}", node.name))?;

        let mut input_meshes = Vec::with_capacity(node.inputs.len());
        let mut input_names = Vec::with_capacity(node.inputs.len());
        for pin_id in &node.inputs {
            let pin = graph
                .pin(*pin_id)
                .ok_or_else(|| "missing input pin".to_string())?;
            input_names.push(pin.name.clone());
            let link = graph.links().find(|link| link.to == *pin_id);
            let mesh = if let Some(link) = link {
                let from_pin = graph
                    .pin(link.from)
                    .ok_or_else(|| "missing upstream pin".to_string())?;
                let upstream_id = from_pin.node;
                let mesh = outputs
                    .get(&upstream_id)
                    .ok_or_else(|| format!("missing upstream output {:?}", upstream_id))?;
                Some(mesh.clone())
            } else {
                None
            };
            input_meshes.push(mesh);
        }

        let inputs = match kind {
            crate::nodes_builtin::BuiltinNodeKind::Transform
            | crate::nodes_builtin::BuiltinNodeKind::CopyTransform
            | crate::nodes_builtin::BuiltinNodeKind::Normal
            | crate::nodes_builtin::BuiltinNodeKind::Scatter
            | crate::nodes_builtin::BuiltinNodeKind::Color
            | crate::nodes_builtin::BuiltinNodeKind::Noise
            | crate::nodes_builtin::BuiltinNodeKind::AttributeMath
            | crate::nodes_builtin::BuiltinNodeKind::Wrangle
            | crate::nodes_builtin::BuiltinNodeKind::ObjOutput
            | crate::nodes_builtin::BuiltinNodeKind::Output => {
                if let Some(mesh) = input_meshes.first().and_then(|mesh| mesh.clone()) {
                    vec![mesh]
                } else {
                    let name = input_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "in".to_string());
                    return Err(format!("missing input '{}'", name));
                }
            }
            crate::nodes_builtin::BuiltinNodeKind::CopyToPoints => {
                let source = input_meshes.first().and_then(|mesh| mesh.clone());
                let template = input_meshes.get(1).and_then(|mesh| mesh.clone());
                if source.is_none() {
                    let name = input_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "source".to_string());
                    return Err(format!("missing input '{}'", name));
                }
                if template.is_none() {
                    let name = input_names
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| "template".to_string());
                    return Err(format!("missing input '{}'", name));
                }
                vec![source.unwrap(), template.unwrap()]
            }
            crate::nodes_builtin::BuiltinNodeKind::Merge => {
                input_meshes.into_iter().flatten().collect()
            }
            _ => Vec::new(),
        };

        if matches!(kind, crate::nodes_builtin::BuiltinNodeKind::Merge) && inputs.is_empty() {
            return Err("Merge requires at least one mesh input".to_string());
        }

        let mesh = compute_mesh_node(kind, params, &inputs)?;
        outputs.insert(node_id, mesh);
        Ok(())
    })?;

    if !report.output_valid {
        for err in &report.errors {
            match err {
                crate::eval::EvalError::Node { node, .. } => {
                    outputs.remove(node);
                }
                crate::eval::EvalError::Upstream { node, upstream } => {
                    outputs.remove(node);
                    for upstream_node in upstream {
                        outputs.remove(upstream_node);
                    }
                }
            }
        }
        outputs.remove(&output);
        return Ok(MeshEvalResult {
            report,
            output: None,
        });
    }

    let output_mesh = outputs.get(&output).cloned();
    Ok(MeshEvalResult {
        report,
        output: output_mesh,
    })
}
