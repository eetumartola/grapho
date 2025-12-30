use std::collections::{HashMap, HashSet};

use egui::{Color32, Pos2, Ui};
use egui_snarl::ui::{PinInfo, SnarlStyle, SnarlViewer};
use egui_snarl::{InPinId, OutPinId, Snarl};

use core::{
    default_params, node_definition, BuiltinNodeKind, Graph, NodeId, PinId, PinKind, PinType,
};
use tracing;

#[derive(Clone, Copy)]
struct SnarlNode {
    core_id: NodeId,
}

pub struct NodeGraphState {
    snarl: Snarl<SnarlNode>,
    core_to_snarl: HashMap<NodeId, egui_snarl::NodeId>,
    snarl_to_core: HashMap<egui_snarl::NodeId, NodeId>,
    next_pos: Pos2,
    needs_wire_sync: bool,
    selected_node: Option<NodeId>,
}

impl Default for NodeGraphState {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            core_to_snarl: HashMap::new(),
            snarl_to_core: HashMap::new(),
            next_pos: Pos2::new(0.0, 0.0),
            needs_wire_sync: true,
            selected_node: None,
        }
    }
}

impl NodeGraphState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn show(&mut self, ui: &mut Ui, graph: &mut Graph, eval_dirty: &mut bool) {
        self.ensure_nodes(graph);
        if self.needs_wire_sync {
            self.sync_wires(graph);
            self.needs_wire_sync = false;
        }

        let mut viewer = NodeGraphViewer {
            graph,
            core_to_snarl: &mut self.core_to_snarl,
            snarl_to_core: &mut self.snarl_to_core,
            next_pos: &mut self.next_pos,
            selected_node: &mut self.selected_node,
            changed: false,
        };
        let style = SnarlStyle::default();
        self.snarl.show(&mut viewer, &style, "node_graph", ui);

        if viewer.changed {
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }
    }

    pub fn add_demo_graph(&mut self, graph: &mut Graph) {
        let origin = self.next_pos;
        let box_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Box,
            origin,
        );
        let transform_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Transform,
            Pos2::new(origin.x + 240.0, origin.y),
        );
        let output_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Output,
            Pos2::new(origin.x + 480.0, origin.y),
        );

        let box_out = graph
            .node(box_id)
            .and_then(|node| node.outputs.get(0).copied());
        let transform_in = graph
            .node(transform_id)
            .and_then(|node| node.inputs.get(0).copied());
        let transform_out = graph
            .node(transform_id)
            .and_then(|node| node.outputs.get(0).copied());
        let output_in = graph
            .node(output_id)
            .and_then(|node| node.inputs.get(0).copied());

        if let (Some(box_out), Some(transform_in), Some(transform_out), Some(output_in)) =
            (box_out, transform_in, transform_out, output_in)
        {
            let _ = graph.add_link(box_out, transform_in);
            let _ = graph.add_link(transform_out, output_in);
        }

        self.needs_wire_sync = true;
    }

    fn ensure_nodes(&mut self, graph: &Graph) {
        for node in graph.nodes() {
            if self.core_to_snarl.contains_key(&node.id) {
                continue;
            }

            let pos = self.next_pos;
            let snarl_id = self.snarl.insert_node(pos, SnarlNode { core_id: node.id });
            self.core_to_snarl.insert(node.id, snarl_id);
            self.snarl_to_core.insert(snarl_id, node.id);
            self.advance_pos();
            self.needs_wire_sync = true;
        }

        let mut to_remove = Vec::new();
        for (snarl_id, core_id) in &self.snarl_to_core {
            if graph.node(*core_id).is_none() {
                to_remove.push(*snarl_id);
            }
        }

        for snarl_id in to_remove {
            if let Some(core_id) = self.snarl_to_core.remove(&snarl_id) {
                self.core_to_snarl.remove(&core_id);
            }
            let _ = self.snarl.remove_node(snarl_id);
            self.needs_wire_sync = true;
        }

        if let Some(selected) = self.selected_node {
            if graph.node(selected).is_none() {
                self.selected_node = None;
            }
        }
    }

    fn sync_wires(&mut self, graph: &Graph) {
        let mut desired = HashSet::new();
        for link in graph.links() {
            if let Some((out_pin, in_pin)) = self.snarl_link_for_core(graph, link.from, link.to) {
                desired.insert((out_pin, in_pin));
            }
        }

        let existing: Vec<_> = self.snarl.wires().collect();
        for (out_pin, in_pin) in existing {
            if !desired.contains(&(out_pin, in_pin)) {
                let _ = self.snarl.disconnect(out_pin, in_pin);
            }
        }

        for (out_pin, in_pin) in desired {
            let _ = self.snarl.connect(out_pin, in_pin);
        }
    }

    fn snarl_link_for_core(
        &self,
        graph: &Graph,
        from: PinId,
        to: PinId,
    ) -> Option<(OutPinId, InPinId)> {
        let from_pin = graph.pin(from)?;
        let to_pin = graph.pin(to)?;
        if from_pin.kind != PinKind::Output || to_pin.kind != PinKind::Input {
            return None;
        }

        let from_node = graph.node(from_pin.node)?;
        let to_node = graph.node(to_pin.node)?;
        let from_index = from_node.outputs.iter().position(|id| *id == from)?;
        let to_index = to_node.inputs.iter().position(|id| *id == to)?;

        let snarl_from = *self.core_to_snarl.get(&from_pin.node)?;
        let snarl_to = *self.core_to_snarl.get(&to_pin.node)?;

        Some((
            OutPinId {
                node: snarl_from,
                output: from_index,
            },
            InPinId {
                node: snarl_to,
                input: to_index,
            },
        ))
    }

    fn advance_pos(&mut self) {
        self.next_pos.x += 240.0;
        if self.next_pos.x > 1000.0 {
            self.next_pos.x = 0.0;
            self.next_pos.y += 200.0;
        }
    }

    pub fn show_inspector(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        let Some(node_id) = self.selected_node else {
            ui.label("No selection.");
            return false;
        };

        let Some(node) = graph.node(node_id) else {
            self.selected_node = None;
            ui.label("No selection.");
            return false;
        };

        ui.label(format!("{} ({})", node.name, node.category));
        ui.separator();

        let params: Vec<(String, core::ParamValue)> = node
            .params
            .values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        if params.is_empty() {
            ui.label("No parameters.");
            return false;
        }

        let mut changed = false;
        for (key, value) in params {
            let (next_value, did_change) = edit_param(ui, &key, value);
            if did_change {
                if graph.set_param(node_id, key, next_value).is_ok() {
                    changed = true;
                }
            }
        }

        changed
    }
}

struct NodeGraphViewer<'a> {
    graph: &'a mut Graph,
    core_to_snarl: &'a mut HashMap<NodeId, egui_snarl::NodeId>,
    snarl_to_core: &'a mut HashMap<egui_snarl::NodeId, NodeId>,
    next_pos: &'a mut Pos2,
    selected_node: &'a mut Option<NodeId>,
    changed: bool,
}

impl<'a> NodeGraphViewer<'a> {
    fn core_node_id(
        &self,
        snarl: &Snarl<SnarlNode>,
        node_id: egui_snarl::NodeId,
    ) -> Option<NodeId> {
        snarl.get_node(node_id).map(|node| node.core_id)
    }

    fn core_pin_for_input(&self, snarl: &Snarl<SnarlNode>, pin: InPinId) -> Option<PinId> {
        let core_node = self.core_node_id(snarl, pin.node)?;
        let node = self.graph.node(core_node)?;
        node.inputs.get(pin.input).copied()
    }

    fn core_pin_for_output(&self, snarl: &Snarl<SnarlNode>, pin: OutPinId) -> Option<PinId> {
        let core_node = self.core_node_id(snarl, pin.node)?;
        let node = self.graph.node(core_node)?;
        node.outputs.get(pin.output).copied()
    }

    fn add_node(&mut self, snarl: &mut Snarl<SnarlNode>, kind: BuiltinNodeKind, pos: Pos2) {
        let core_id = self.graph.add_node(node_definition(kind));
        let params = default_params(kind);
        for (key, value) in params.values {
            let _ = self.graph.set_param(core_id, key, value);
        }

        let snarl_id = snarl.insert_node(pos, SnarlNode { core_id });
        self.core_to_snarl.insert(core_id, snarl_id);
        self.snarl_to_core.insert(snarl_id, core_id);
        *self.next_pos = Pos2::new(pos.x + 240.0, pos.y);
        self.changed = true;
    }
}

impl SnarlViewer<SnarlNode> for NodeGraphViewer<'_> {
    fn title(&mut self, node: &SnarlNode) -> String {
        self.graph
            .node(node.core_id)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| "Missing".to_string())
    }

    fn show_header(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        let title = self.title(&snarl[node]);
        let core_id = self.core_node_id(snarl, node);
        let selected = core_id.map_or(false, |id| {
            self.selected_node
                .as_ref()
                .map_or(false, |selected| *selected == id)
        });
        let response = ui.selectable_label(selected, title);
        if response.clicked() {
            *self.selected_node = core_id;
        }
    }

    fn inputs(&mut self, node: &SnarlNode) -> usize {
        self.graph
            .node(node.core_id)
            .map(|node| node.inputs.len())
            .unwrap_or(0)
    }

    fn outputs(&mut self, node: &SnarlNode) -> usize {
        self.graph
            .node(node.core_id)
            .map(|node| node.outputs.len())
            .unwrap_or(0)
    }

    fn show_input(
        &mut self,
        pin: &egui_snarl::InPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> PinInfo {
        if let Some(core_pin) = self.core_pin_for_input(snarl, pin.id) {
            if let Some(pin_data) = self.graph.pin(core_pin) {
                ui.label(&pin_data.name);
                return PinInfo::circle().with_fill(pin_color(pin_data.pin_type));
            }
        }
        ui.label("?");
        PinInfo::circle()
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> PinInfo {
        if let Some(core_pin) = self.core_pin_for_output(snarl, pin.id) {
            if let Some(pin_data) = self.graph.pin(core_pin) {
                ui.label(&pin_data.name);
                return PinInfo::circle().with_fill(pin_color(pin_data.pin_type));
            }
        }
        ui.label("?");
        PinInfo::circle()
    }

    fn has_graph_menu(&mut self, _pos: Pos2, _snarl: &mut Snarl<SnarlNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        ui.label("Add node");
        for kind in [
            BuiltinNodeKind::Box,
            BuiltinNodeKind::Grid,
            BuiltinNodeKind::Transform,
            BuiltinNodeKind::Merge,
            BuiltinNodeKind::Output,
        ] {
            if ui.button(kind.name()).clicked() {
                self.add_node(snarl, kind, pos);
                ui.close_menu();
            }
        }
    }

    fn has_node_menu(&mut self, _node: &SnarlNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        if ui.button("Delete node").clicked() {
            if let Some(core_id) = self.core_node_id(snarl, node) {
                self.graph.remove_node(core_id);
                self.core_to_snarl.remove(&core_id);
                self.snarl_to_core.remove(&node);
                let _ = snarl.remove_node(node);
                self.changed = true;
            }
            ui.close_menu();
        }
    }

    fn connect(
        &mut self,
        from: &egui_snarl::OutPin,
        to: &egui_snarl::InPin,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        let Some(from_pin) = self.core_pin_for_output(snarl, from.id) else {
            return;
        };
        let Some(to_pin) = self.core_pin_for_input(snarl, to.id) else {
            return;
        };

        match self.graph.add_link(from_pin, to_pin) {
            Ok(_) => {
                let _ = snarl.connect(from.id, to.id);
                self.changed = true;
            }
            Err(err) => {
                tracing::warn!("link rejected: {:?}", err);
            }
        }
    }

    fn disconnect(
        &mut self,
        from: &egui_snarl::OutPin,
        to: &egui_snarl::InPin,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        let Some(from_pin) = self.core_pin_for_output(snarl, from.id) else {
            return;
        };
        let Some(to_pin) = self.core_pin_for_input(snarl, to.id) else {
            return;
        };
        let _ = self.graph.remove_link_between(from_pin, to_pin);
        let _ = snarl.disconnect(from.id, to.id);
        self.changed = true;
    }

    fn drop_outputs(&mut self, pin: &egui_snarl::OutPin, snarl: &mut Snarl<SnarlNode>) {
        if let Some(core_pin) = self.core_pin_for_output(snarl, pin.id) {
            let _ = self.graph.remove_links_for_pin(core_pin);
        }
        snarl.drop_outputs(pin.id);
        self.changed = true;
    }

    fn drop_inputs(&mut self, pin: &egui_snarl::InPin, snarl: &mut Snarl<SnarlNode>) {
        if let Some(core_pin) = self.core_pin_for_input(snarl, pin.id) {
            let _ = self.graph.remove_links_for_pin(core_pin);
        }
        snarl.drop_inputs(pin.id);
        self.changed = true;
    }
}

fn pin_color(pin_type: PinType) -> Color32 {
    match pin_type {
        PinType::Mesh => Color32::from_rgb(80, 160, 255),
        PinType::Float => Color32::from_rgb(220, 180, 90),
        PinType::Int => Color32::from_rgb(200, 120, 220),
        PinType::Bool => Color32::from_rgb(140, 220, 140),
        PinType::Vec2 => Color32::from_rgb(255, 160, 90),
        PinType::Vec3 => Color32::from_rgb(90, 210, 210),
    }
}

fn add_builtin_node(
    graph: &mut Graph,
    snarl: &mut Snarl<SnarlNode>,
    core_to_snarl: &mut HashMap<NodeId, egui_snarl::NodeId>,
    snarl_to_core: &mut HashMap<egui_snarl::NodeId, NodeId>,
    kind: BuiltinNodeKind,
    pos: Pos2,
) -> NodeId {
    let core_id = graph.add_node(node_definition(kind));
    let params = default_params(kind);
    for (key, value) in params.values {
        let _ = graph.set_param(core_id, key, value);
    }
    let snarl_id = snarl.insert_node(pos, SnarlNode { core_id });
    core_to_snarl.insert(core_id, snarl_id);
    snarl_to_core.insert(snarl_id, core_id);
    core_id
}

fn edit_param(ui: &mut Ui, label: &str, value: core::ParamValue) -> (core::ParamValue, bool) {
    match value {
        core::ParamValue::Float(mut v) => {
            let response = ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::DragValue::new(&mut v).speed(0.1))
            });
            (core::ParamValue::Float(v), response.inner.changed())
        }
        core::ParamValue::Int(mut v) => {
            let response = ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::DragValue::new(&mut v).speed(1.0))
            });
            (core::ParamValue::Int(v), response.inner.changed())
        }
        core::ParamValue::Bool(mut v) => {
            let response = ui.checkbox(&mut v, label);
            (core::ParamValue::Bool(v), response.changed())
        }
        core::ParamValue::Vec2(mut v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(label);
                for idx in 0..2 {
                    if ui.add(egui::DragValue::new(&mut v[idx]).speed(0.1)).changed() {
                        changed = true;
                    }
                }
            });
            (core::ParamValue::Vec2(v), changed)
        }
        core::ParamValue::Vec3(mut v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(label);
                for idx in 0..3 {
                    if ui.add(egui::DragValue::new(&mut v[idx]).speed(0.1)).changed() {
                        changed = true;
                    }
                }
            });
            (core::ParamValue::Vec3(v), changed)
        }
    }
}
