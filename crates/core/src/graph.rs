use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PinId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkId(u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    nodes: BTreeMap<NodeId, Node>,
    pins: BTreeMap<PinId, Pin>,
    links: BTreeMap<LinkId, Link>,
    next_node_id: u64,
    next_pin_id: u64,
    next_link_id: u64,
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            pins: BTreeMap::new(),
            links: BTreeMap::new(),
            next_node_id: 1,
            next_pin_id: 1,
            next_link_id: 1,
        }
    }
}

impl Graph {
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn display_node(&self) -> Option<NodeId> {
        self.nodes
            .values()
            .find(|node| node.display)
            .map(|node| node.id)
    }

    pub fn template_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|node| node.template)
            .map(|node| node.id)
            .collect()
    }

    pub fn set_display_node(&mut self, node_id: Option<NodeId>) -> Result<(), GraphError> {
        if let Some(id) = node_id {
            if !self.nodes.contains_key(&id) {
                return Err(GraphError::MissingNode(id));
            }
        }
        for node in self.nodes.values_mut() {
            node.display = Some(node.id) == node_id;
        }
        Ok(())
    }

    pub fn toggle_display_node(&mut self, node_id: NodeId) -> Result<(), GraphError> {
        let display = self
            .nodes
            .get(&node_id)
            .ok_or(GraphError::MissingNode(node_id))?
            .display;
        if display {
            self.set_display_node(None)
        } else {
            self.set_display_node(Some(node_id))
        }
    }

    pub fn set_template_node(&mut self, node_id: NodeId, enabled: bool) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(GraphError::MissingNode(node_id))?;
        node.template = enabled;
        Ok(())
    }

    pub fn toggle_template_node(&mut self, node_id: NodeId) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(GraphError::MissingNode(node_id))?;
        node.template = !node.template;
        Ok(())
    }

    pub fn pin(&self, id: PinId) -> Option<&Pin> {
        self.pins.get(&id)
    }

    pub fn add_node(&mut self, def: NodeDefinition) -> NodeId {
        let node_id = self.alloc_node_id();
        let mut input_ids = Vec::new();
        let mut output_ids = Vec::new();

        for input in def.inputs {
            let pin_id = self.alloc_pin_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    node: node_id,
                    name: input.name,
                    kind: PinKind::Input,
                    pin_type: input.pin_type,
                },
            );
            input_ids.push(pin_id);
        }

        for output in def.outputs {
            let pin_id = self.alloc_pin_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    node: node_id,
                    name: output.name,
                    kind: PinKind::Output,
                    pin_type: output.pin_type,
                },
            );
            output_ids.push(pin_id);
        }

        self.nodes.insert(
            node_id,
            Node {
                id: node_id,
                name: def.name,
                inputs: input_ids,
                outputs: output_ids,
                params: NodeParams::default(),
                category: def.category,
                param_version: 0,
                display: false,
                template: false,
            },
        );

        node_id
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> bool {
        let Some(node) = self.nodes.remove(&node_id) else {
            return false;
        };

        let mut pins_to_remove: HashSet<PinId> = node.inputs.into_iter().collect();
        pins_to_remove.extend(node.outputs);

        self.links.retain(|_, link| {
            !pins_to_remove.contains(&link.from) && !pins_to_remove.contains(&link.to)
        });

        for pin_id in pins_to_remove {
            self.pins.remove(&pin_id);
        }

        true
    }

    pub fn add_link(&mut self, from: PinId, to: PinId) -> Result<LinkId, GraphError> {
        let from_pin = self.pins.get(&from).ok_or(GraphError::MissingPin(from))?;
        let to_pin = self.pins.get(&to).ok_or(GraphError::MissingPin(to))?;

        if from_pin.kind != PinKind::Output || to_pin.kind != PinKind::Input {
            return Err(GraphError::WrongPinDirection { from, to });
        }

        if self.links.values().any(|link| link.to == to) {
            return Err(GraphError::InputAlreadyConnected { to });
        }

        if from_pin.pin_type != to_pin.pin_type {
            return Err(GraphError::IncompatiblePinTypes {
                from: from_pin.pin_type,
                to: to_pin.pin_type,
            });
        }

        let link_id = self.alloc_link_id();
        self.links.insert(
            link_id,
            Link {
                id: link_id,
                from,
                to,
            },
        );
        Ok(link_id)
    }

    pub fn remove_link(&mut self, link_id: LinkId) -> bool {
        self.links.remove(&link_id).is_some()
    }

    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.links.values()
    }

    pub fn remove_link_between(&mut self, from: PinId, to: PinId) -> bool {
        let link_id = self.links.iter().find_map(|(id, link)| {
            if link.from == from && link.to == to {
                Some(*id)
            } else {
                None
            }
        });

        link_id.map(|id| self.links.remove(&id)).is_some()
    }

    pub fn remove_links_for_pin(&mut self, pin_id: PinId) -> usize {
        let before = self.links.len();
        self.links
            .retain(|_, link| link.from != pin_id && link.to != pin_id);
        before - self.links.len()
    }

    pub fn set_param(
        &mut self,
        node_id: NodeId,
        key: impl Into<String>,
        value: ParamValue,
    ) -> Result<(), GraphError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(GraphError::MissingNode(node_id))?;
        let key = key.into();
        let changed = node
            .params
            .values
            .get(&key)
            .map(|existing| existing != &value)
            .unwrap_or(true);

        if changed {
            node.params.values.insert(key, value);
            node.param_version = node.param_version.wrapping_add(1);
        }

        Ok(())
    }

    pub fn topo_sort_from(&self, output: NodeId) -> Result<Vec<NodeId>, GraphError> {
        if !self.nodes.contains_key(&output) {
            return Err(GraphError::MissingNode(output));
        }

        let mut ordered = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        self.visit_node(
            output,
            &mut visiting,
            &mut visited,
            &mut ordered,
            &mut stack,
        )?;

        Ok(ordered)
    }

    fn visit_node(
        &self,
        node_id: NodeId,
        visiting: &mut HashSet<NodeId>,
        visited: &mut HashSet<NodeId>,
        ordered: &mut Vec<NodeId>,
        stack: &mut Vec<NodeId>,
    ) -> Result<(), GraphError> {
        if visited.contains(&node_id) {
            return Ok(());
        }
        if visiting.contains(&node_id) {
            stack.push(node_id);
            return Err(GraphError::CycleDetected(stack.clone()));
        }

        visiting.insert(node_id);
        stack.push(node_id);

        for upstream in self.upstream_nodes(node_id) {
            self.visit_node(upstream, visiting, visited, ordered, stack)?;
        }

        visiting.remove(&node_id);
        visited.insert(node_id);
        ordered.push(node_id);
        stack.pop();
        Ok(())
    }

    pub fn upstream_nodes(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut upstream = Vec::new();

        for link in self.links.values() {
            let Some(to_node) = self.node_for_pin(link.to) else {
                continue;
            };
            if to_node != node_id {
                continue;
            }

            if let Some(from_node) = self.node_for_pin(link.from) {
                upstream.push(from_node);
            }
        }

        upstream
    }

    fn node_for_pin(&self, pin_id: PinId) -> Option<NodeId> {
        self.pins.get(&pin_id).map(|pin| pin.node)
    }

    fn alloc_node_id(&mut self) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        NodeId(id)
    }

    fn alloc_pin_id(&mut self) -> PinId {
        let id = self.next_pin_id;
        self.next_pin_id += 1;
        PinId(id)
    }

    fn alloc_link_id(&mut self) -> LinkId {
        let id = self.next_link_id;
        self.next_link_id += 1;
        LinkId(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PinId>,
    pub outputs: Vec<PinId>,
    pub params: NodeParams,
    #[serde(default)]
    pub param_version: u64,
    #[serde(default)]
    pub display: bool,
    #[serde(default)]
    pub template: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeParams {
    pub values: BTreeMap<String, ParamValue>,
}

impl NodeParams {
    pub fn get_vec2(&self, key: &str, default: [f32; 2]) -> [f32; 2] {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::Vec2(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(default)
    }

    pub fn get_vec3(&self, key: &str, default: [f32; 3]) -> [f32; 3] {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::Vec3(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(default)
    }

    pub fn get_float(&self, key: &str, default: f32) -> f32 {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::Float(v) => Some(*v),
                ParamValue::Int(v) => Some(*v as f32),
                _ => None,
            })
            .unwrap_or(default)
    }

    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::Int(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::Bool(v) => Some(*v),
                _ => None,
            })
            .unwrap_or(default)
    }

    pub fn get_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.values
            .get(key)
            .and_then(|value| match value {
                ParamValue::String(v) => Some(v.as_str()),
                _ => None,
            })
            .unwrap_or(default)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    String(String),
}

#[derive(Debug, Clone)]
pub struct NodeDefinition {
    pub name: String,
    pub category: String,
    pub inputs: Vec<PinDefinition>,
    pub outputs: Vec<PinDefinition>,
}

#[derive(Debug, Clone)]
pub struct PinDefinition {
    pub name: String,
    pub pin_type: PinType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinType {
    Mesh,
    Float,
    Int,
    Bool,
    Vec2,
    Vec3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinKind {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: PinId,
    pub node: NodeId,
    pub name: String,
    pub kind: PinKind,
    pub pin_type: PinType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: LinkId,
    pub from: PinId,
    pub to: PinId,
}

#[derive(Debug, Clone)]
pub enum GraphError {
    MissingNode(NodeId),
    MissingPin(PinId),
    WrongPinDirection { from: PinId, to: PinId },
    InputAlreadyConnected { to: PinId },
    IncompatiblePinTypes { from: PinType, to: PinType },
    CycleDetected(Vec<NodeId>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_node(name: &str) -> NodeDefinition {
        NodeDefinition {
            name: name.to_string(),
            category: "Demo".to_string(),
            inputs: vec![PinDefinition {
                name: "in".to_string(),
                pin_type: PinType::Mesh,
            }],
            outputs: vec![PinDefinition {
                name: "out".to_string(),
                pin_type: PinType::Mesh,
            }],
        }
    }

    #[test]
    fn add_and_remove_node() {
        let mut graph = Graph::default();
        let node_id = graph.add_node(demo_node("NodeA"));
        assert!(graph.node(node_id).is_some());
        assert!(graph.remove_node(node_id));
        assert!(graph.node(node_id).is_none());
    }

    #[test]
    fn rejects_incompatible_links() {
        let mut graph = Graph::default();
        let a = graph.add_node(NodeDefinition {
            name: "A".to_string(),
            category: "Demo".to_string(),
            inputs: vec![],
            outputs: vec![PinDefinition {
                name: "out".to_string(),
                pin_type: PinType::Float,
            }],
        });
        let b = graph.add_node(NodeDefinition {
            name: "B".to_string(),
            category: "Demo".to_string(),
            inputs: vec![PinDefinition {
                name: "in".to_string(),
                pin_type: PinType::Mesh,
            }],
            outputs: vec![],
        });

        let from = graph.nodes.get(&a).unwrap().outputs[0];
        let to = graph.nodes.get(&b).unwrap().inputs[0];

        let result = graph.add_link(from, to);
        assert!(matches!(
            result,
            Err(GraphError::IncompatiblePinTypes { .. })
        ));
    }

    #[test]
    fn accepts_valid_links() {
        let mut graph = Graph::default();
        let a = graph.add_node(demo_node("A"));
        let b = graph.add_node(demo_node("B"));

        let from = graph.nodes.get(&a).unwrap().outputs[0];
        let to = graph.nodes.get(&b).unwrap().inputs[0];

        let result = graph.add_link(from, to);
        assert!(result.is_ok());
    }

    fn node_def(name: &str, inputs: usize, outputs: usize) -> NodeDefinition {
        let make_pin = |label: &str| PinDefinition {
            name: label.to_string(),
            pin_type: PinType::Mesh,
        };

        NodeDefinition {
            name: name.to_string(),
            category: "Demo".to_string(),
            inputs: (0..inputs).map(|i| make_pin(&format!("in{}", i))).collect(),
            outputs: (0..outputs)
                .map(|i| make_pin(&format!("out{}", i)))
                .collect(),
        }
    }

    #[test]
    fn topo_sort_orders_upstream_first() {
        let mut graph = Graph::default();
        let source = graph.add_node(node_def("Source", 0, 1));
        let mid = graph.add_node(node_def("Mid", 1, 1));
        let output = graph.add_node(node_def("Output", 1, 0));

        let source_out = graph.nodes.get(&source).unwrap().outputs[0];
        let mid_in = graph.nodes.get(&mid).unwrap().inputs[0];
        let mid_out = graph.nodes.get(&mid).unwrap().outputs[0];
        let output_in = graph.nodes.get(&output).unwrap().inputs[0];

        graph.add_link(source_out, mid_in).unwrap();
        graph.add_link(mid_out, output_in).unwrap();

        let order = graph.topo_sort_from(output).unwrap();
        let pos_source = order.iter().position(|id| *id == source).unwrap();
        let pos_mid = order.iter().position(|id| *id == mid).unwrap();
        let pos_output = order.iter().position(|id| *id == output).unwrap();

        assert!(pos_source < pos_mid);
        assert!(pos_mid < pos_output);
    }

    #[test]
    fn topo_sort_detects_cycles() {
        let mut graph = Graph::default();
        let a = graph.add_node(node_def("A", 1, 1));
        let b = graph.add_node(node_def("B", 1, 1));

        let a_out = graph.nodes.get(&a).unwrap().outputs[0];
        let a_in = graph.nodes.get(&a).unwrap().inputs[0];
        let b_out = graph.nodes.get(&b).unwrap().outputs[0];
        let b_in = graph.nodes.get(&b).unwrap().inputs[0];

        graph.add_link(a_out, b_in).unwrap();
        graph.add_link(b_out, a_in).unwrap();

        let result = graph.topo_sort_from(a);
        assert!(matches!(result, Err(GraphError::CycleDetected(_))));
    }
}
