use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::graph::{Graph, GraphError, NodeId, NodeParams};

#[derive(Debug, Clone, Copy, Default)]
pub struct EvalCacheStats {
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Default)]
pub struct EvalState {
    nodes: BTreeMap<NodeId, NodeEvalState>,
    pub stats: EvalCacheStats,
}

#[derive(Debug, Default)]
struct NodeEvalState {
    last_signature: u64,
    last_param_version: u64,
    last_upstream_signature: u64,
    initialized: bool,
    output_version: u64,
}

#[derive(Debug, Default)]
pub struct EvalReport {
    pub ordered: Vec<NodeId>,
    pub computed: Vec<NodeId>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub node_reports: BTreeMap<NodeId, EvalNodeReport>,
    pub dirty: Vec<DirtyNodeReport>,
    pub errors: Vec<EvalError>,
    pub output_valid: bool,
}

#[derive(Debug, Clone)]
pub struct EvalNodeReport {
    pub node: NodeId,
    pub duration_ms: f32,
    pub cache_hit: bool,
    pub output_version: u64,
    pub error: Option<EvalError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyReason {
    NewNode,
    ParamChanged,
    UpstreamChanged,
    ParamAndUpstreamChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyNodeReport {
    pub node: NodeId,
    pub reason: DirtyReason,
}

#[derive(Debug, Clone)]
pub enum EvalError {
    Node { node: NodeId, message: String },
    Upstream { node: NodeId, upstream: Vec<NodeId> },
}

impl EvalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_output_version(&self, node_id: NodeId) -> Option<u64> {
        self.nodes.get(&node_id).map(|state| state.output_version)
    }

    fn node_state_mut(&mut self, node_id: NodeId) -> &mut NodeEvalState {
        self.nodes.entry(node_id).or_default()
    }
}

pub fn evaluate_from(
    graph: &Graph,
    output: NodeId,
    state: &mut EvalState,
) -> Result<EvalReport, GraphError> {
    evaluate_from_with(graph, output, state, |_node_id, _params| Ok(()))
}

pub fn evaluate_from_with<F>(
    graph: &Graph,
    output: NodeId,
    state: &mut EvalState,
    mut compute: F,
) -> Result<EvalReport, GraphError>
where
    F: FnMut(NodeId, &NodeParams) -> Result<(), String>,
{
    let ordered = graph.topo_sort_from(output)?;
    let mut report = EvalReport {
        ordered,
        output_valid: true,
        ..Default::default()
    };
    let mut failed_nodes = Vec::<NodeId>::new();

    for node_id in &report.ordered {
        let node = graph
            .node(*node_id)
            .ok_or(GraphError::MissingNode(*node_id))?;
        let mut upstream = graph.upstream_nodes(*node_id);
        upstream.sort();

        let mut upstream_versions = Vec::with_capacity(upstream.len());
        for upstream_id in &upstream {
            let upstream_state = state.node_state_mut(*upstream_id);
            upstream_versions.push((*upstream_id, upstream_state.output_version));
        }

        let upstream_signature = hash_upstream(&upstream_versions);
        let signature = hash_signature(node.param_version, &upstream_versions);
        let (last_signature, output_version) = {
            let node_state = state.node_state_mut(*node_id);
            (node_state.last_signature, node_state.output_version)
        };
        let mut node_report = EvalNodeReport {
            node: *node_id,
            duration_ms: 0.0,
            cache_hit: false,
            output_version,
            error: None,
        };

        let upstream_failed: Vec<NodeId> = upstream
            .iter()
            .copied()
            .filter(|id| failed_nodes.contains(id))
            .collect();
        if !upstream_failed.is_empty() {
            let error = EvalError::Upstream {
                node: *node_id,
                upstream: upstream_failed,
            };
            node_report.error = Some(error.clone());
            report.errors.push(error);
            report.output_valid = false;
            failed_nodes.push(*node_id);
            report.node_reports.insert(*node_id, node_report);
            continue;
        }

        let dirty_reason = {
            let node_state = state.node_state_mut(*node_id);
            if !node_state.initialized {
                Some(DirtyReason::NewNode)
            } else if last_signature == signature {
                None
            } else {
                let param_changed = node.param_version != node_state.last_param_version;
                let upstream_changed = upstream_signature != node_state.last_upstream_signature;
                match (param_changed, upstream_changed) {
                    (true, true) => Some(DirtyReason::ParamAndUpstreamChanged),
                    (true, false) => Some(DirtyReason::ParamChanged),
                    (false, true) => Some(DirtyReason::UpstreamChanged),
                    (false, false) => None,
                }
            }
        };
        if let Some(reason) = dirty_reason {
            report.dirty.push(DirtyNodeReport {
                node: *node_id,
                reason,
            });
        }

        if last_signature == signature {
            report.cache_hits += 1;
            state.stats.hits += 1;
            node_report.cache_hit = true;
            node_report.output_version = output_version;
            let node_state = state.node_state_mut(*node_id);
            node_state.last_param_version = node.param_version;
            node_state.last_upstream_signature = upstream_signature;
            node_state.initialized = true;
            report.node_reports.insert(*node_id, node_report);
            continue;
        }

        let start = Instant::now();
        let compute_result = compute(*node_id, &node.params);
        node_report.duration_ms = start.elapsed().as_secs_f32() * 1000.0;

        match compute_result {
            Ok(()) => {
                let node_state = state.node_state_mut(*node_id);
                node_state.last_signature = signature;
                node_state.last_param_version = node.param_version;
                node_state.last_upstream_signature = upstream_signature;
                node_state.initialized = true;
                node_state.output_version = node_state.output_version.wrapping_add(1);
                node_report.output_version = node_state.output_version;
                report.cache_misses += 1;
                state.stats.misses += 1;
                report.computed.push(*node_id);
            }
            Err(message) => {
                let error = EvalError::Node {
                    node: *node_id,
                    message,
                };
                node_report.error = Some(error.clone());
                report.errors.push(error);
                report.output_valid = false;
                failed_nodes.push(*node_id);
            }
        }

        report.node_reports.insert(*node_id, node_report);
    }

    Ok(report)
}

fn hash_signature(param_version: u64, upstream_versions: &[(NodeId, u64)]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    param_version.hash(&mut hasher);
    upstream_versions.hash(&mut hasher);
    hasher.finish()
}

fn hash_upstream(upstream_versions: &[(NodeId, u64)]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    upstream_versions.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeDefinition, ParamValue, PinDefinition, PinType};

    fn node_def(name: &str, inputs: usize, outputs: usize) -> NodeDefinition {
        let make_pin = |label: &str| PinDefinition {
            name: label.to_string(),
            pin_type: PinType::Mesh,
        };

        NodeDefinition {
            name: name.to_string(),
            category: "Eval".to_string(),
            inputs: (0..inputs).map(|i| make_pin(&format!("in{}", i))).collect(),
            outputs: (0..outputs)
                .map(|i| make_pin(&format!("out{}", i)))
                .collect(),
        }
    }

    fn connect(graph: &mut Graph, from: NodeId, to: NodeId) {
        let from_pin = graph.node(from).unwrap().outputs[0];
        let to_pin = graph.node(to).unwrap().inputs[0];
        graph.add_link(from_pin, to_pin).unwrap();
    }

    #[test]
    fn cache_hits_when_unchanged() {
        let mut graph = Graph::default();
        let a = graph.add_node(node_def("A", 0, 1));
        let b = graph.add_node(node_def("B", 1, 1));
        let c = graph.add_node(node_def("C", 1, 0));
        connect(&mut graph, a, b);
        connect(&mut graph, b, c);

        let mut state = EvalState::new();
        let first = evaluate_from(&graph, c, &mut state).unwrap();
        assert_eq!(first.computed.len(), 3);

        let second = evaluate_from(&graph, c, &mut state).unwrap();
        assert_eq!(second.computed.len(), 0);
        assert_eq!(second.cache_hits, 3);
    }

    #[test]
    fn upstream_change_recomputes_downstream() {
        let mut graph = Graph::default();
        let a = graph.add_node(node_def("A", 0, 1));
        let b = graph.add_node(node_def("B", 1, 1));
        let c = graph.add_node(node_def("C", 1, 0));
        connect(&mut graph, a, b);
        connect(&mut graph, b, c);

        let mut state = EvalState::new();
        evaluate_from(&graph, c, &mut state).unwrap();

        graph.set_param(a, "size", ParamValue::Float(2.0)).unwrap();
        let report = evaluate_from(&graph, c, &mut state).unwrap();
        assert_eq!(report.computed.len(), 3);
    }

    #[test]
    fn mid_change_skips_upstream() {
        let mut graph = Graph::default();
        let a = graph.add_node(node_def("A", 0, 1));
        let b = graph.add_node(node_def("B", 1, 1));
        let c = graph.add_node(node_def("C", 1, 0));
        connect(&mut graph, a, b);
        connect(&mut graph, b, c);

        let mut state = EvalState::new();
        evaluate_from(&graph, c, &mut state).unwrap();

        graph.set_param(b, "twist", ParamValue::Float(1.0)).unwrap();
        let report = evaluate_from(&graph, c, &mut state).unwrap();
        assert_eq!(report.computed.len(), 2);
        assert_eq!(report.computed[0], b);
        assert_eq!(report.computed[1], c);
    }

    #[test]
    fn error_propagates_downstream() {
        let mut graph = Graph::default();
        let a = graph.add_node(node_def("A", 0, 1));
        let b = graph.add_node(node_def("B", 1, 1));
        let c = graph.add_node(node_def("C", 1, 0));
        connect(&mut graph, a, b);
        connect(&mut graph, b, c);

        let mut state = EvalState::new();
        let report = evaluate_from_with(&graph, c, &mut state, |node_id, _params| {
            if node_id == b {
                Err("boom".to_string())
            } else {
                Ok(())
            }
        })
        .unwrap();

        assert!(!report.output_valid);
        assert!(report
            .errors
            .iter()
            .any(|err| matches!(err, EvalError::Node { node, .. } if *node == b)));
        assert!(report
            .errors
            .iter()
            .any(|err| matches!(err, EvalError::Upstream { node, .. } if *node == c)));
    }
}
