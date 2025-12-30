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
        let upstream = graph.upstream_nodes(node_id);
        let mut inputs = Vec::with_capacity(upstream.len());
        for upstream_id in upstream {
            let mesh = outputs
                .get(&upstream_id)
                .ok_or_else(|| format!("missing upstream output {:?}", upstream_id))?;
            inputs.push(mesh.clone());
        }
        let mesh = compute_mesh_node(kind, params, &inputs)?;
        outputs.insert(node_id, mesh);
        Ok(())
    })?;

    let output_mesh = outputs.get(&output).cloned();
    Ok(MeshEvalResult {
        report,
        output: output_mesh,
    })
}
