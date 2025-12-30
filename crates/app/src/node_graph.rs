use std::collections::{HashMap, HashSet};

use egui::{vec2, Color32, Frame, Pos2, Stroke, Ui};
use egui_snarl::ui::{BackgroundPattern, PinInfo, SnarlStyle, SnarlViewer};
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
    add_menu_open: bool,
    add_menu_screen_pos: Pos2,
    add_menu_graph_pos: Pos2,
    add_menu_filter: String,
    add_menu_focus: bool,
    graph_transform: GraphTransformState,
    error_nodes: HashSet<NodeId>,
    error_messages: HashMap<NodeId, String>,
}

#[derive(Clone, Copy)]
struct GraphTransformState {
    to_global: egui::emath::TSTransform,
    valid: bool,
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
            add_menu_open: false,
            add_menu_screen_pos: Pos2::new(0.0, 0.0),
            add_menu_graph_pos: Pos2::new(0.0, 0.0),
            add_menu_filter: String::new(),
            add_menu_focus: false,
            graph_transform: GraphTransformState {
                to_global: egui::emath::TSTransform::IDENTITY,
                valid: false,
            },
            error_nodes: HashSet::new(),
            error_messages: HashMap::new(),
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
            graph_transform: &mut self.graph_transform,
            error_nodes: &self.error_nodes,
            error_messages: &self.error_messages,
            changed: false,
        };
        let style = SnarlStyle {
            pin_size: Some(10.0),
            bg_frame: Some(Frame::NONE.fill(Color32::from_rgb(18, 18, 18))),
            bg_pattern: Some(BackgroundPattern::grid(vec2(64.0, 64.0), 0.0)),
            bg_pattern_stroke: Some(Stroke::new(1.0, Color32::from_rgb(26, 26, 26))),
            ..SnarlStyle::default()
        };
        self.snarl.show(&mut viewer, &style, "node_graph", ui);

        if viewer.changed {
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        if self.add_menu_open {
            self.show_add_menu(ui, graph);
        }
    }

    pub fn open_add_menu(&mut self, pos: Pos2) {
        self.add_menu_open = true;
        self.add_menu_screen_pos = pos;
        self.add_menu_filter.clear();
        self.add_menu_focus = true;
        if self.graph_transform.valid {
            self.add_menu_graph_pos = self.graph_transform.to_global.inverse() * pos;
        } else {
            self.add_menu_graph_pos = self.next_pos;
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

    pub fn set_error_state(
        &mut self,
        nodes: HashSet<NodeId>,
        messages: HashMap<NodeId, String>,
    ) {
        self.error_nodes = nodes;
        self.error_messages = messages;
    }

    fn show_add_menu(&mut self, ui: &mut Ui, graph: &mut Graph) {
        let mut close_menu = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut menu_rect = None;
        let activate_first = ui.input(|i| i.key_pressed(egui::Key::Enter));

        let response = egui::Window::new("add_node_menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, self.add_menu_screen_pos.to_vec2())
            .frame(Frame::popup(ui.style()))
            .show(ui.ctx(), |ui| {
                ui.label("Add node");
                ui.separator();
                let search_id = ui.make_persistent_id("add_node_search");
                let search = egui::TextEdit::singleline(&mut self.add_menu_filter)
                    .id(search_id)
                    .hint_text("Search...");
                let search_response = ui.add(search);
                if self.add_menu_focus {
                    ui.memory_mut(|mem| mem.request_focus(search_id));
                    self.add_menu_focus = false;
                }
                if search_response.has_focus() && activate_first {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                }

                let filter = self.add_menu_filter.to_lowercase();
                let mut last_category = None;
                let mut matched = false;
                let mut first_match: Option<BuiltinNodeKind> = None;
                for item in builtin_menu_items() {
                    if !filter.is_empty()
                        && !item.name.to_lowercase().contains(&filter)
                        && !item.category.to_lowercase().contains(&filter)
                    {
                        continue;
                    }
                    matched = true;
                    if first_match.is_none() {
                        first_match = Some(item.kind);
                    }
                    if last_category != Some(item.category) {
                        ui.label(item.category);
                        last_category = Some(item.category);
                    }
                    if ui.button(item.name).clicked() {
                        self.try_add_node(graph, item.kind, self.add_menu_graph_pos);
                        close_menu = true;
                    }
                }
                if !matched {
                    ui.label("No matches.");
                } else if activate_first {
                    if let Some(kind) = first_match {
                        self.try_add_node(graph, kind, self.add_menu_graph_pos);
                        close_menu = true;
                    }
                }
            });

        if let Some(inner) = response {
            menu_rect = Some(inner.response.rect);
        }

        if !close_menu {
            if let Some(rect) = menu_rect {
                if ui.input(|i| i.pointer.any_pressed()) {
                    let hover = ui.input(|i| i.pointer.hover_pos());
                    if hover.map_or(true, |pos| !rect.contains(pos)) {
                        close_menu = true;
                    }
                }
            }
        }

        if close_menu {
            self.add_menu_open = false;
        }
    }

    fn try_add_node(&mut self, graph: &mut Graph, kind: BuiltinNodeKind, pos: Pos2) {
        if kind == BuiltinNodeKind::Output
            && graph.nodes().any(|node| node.name == "Output")
        {
            tracing::warn!("Only one Output node is supported right now.");
            return;
        }

        let _ = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            kind,
            pos,
        );
        self.needs_wire_sync = true;
    }
}

struct NodeGraphViewer<'a> {
    graph: &'a mut Graph,
    core_to_snarl: &'a mut HashMap<NodeId, egui_snarl::NodeId>,
    snarl_to_core: &'a mut HashMap<egui_snarl::NodeId, NodeId>,
    next_pos: &'a mut Pos2,
    selected_node: &'a mut Option<NodeId>,
    graph_transform: &'a mut GraphTransformState,
    error_nodes: &'a HashSet<NodeId>,
    error_messages: &'a HashMap<NodeId, String>,
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
        if kind == BuiltinNodeKind::Output
            && self.graph.nodes().any(|node| node.name == "Output")
        {
            tracing::warn!("Only one Output node is supported right now.");
            return;
        }

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
        snarl: &mut Snarl<SnarlNode>,
    ) {
        let title = self.title(&snarl[node]);
        ui.label(title);
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
        snarl: &mut Snarl<SnarlNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
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
        snarl: &mut Snarl<SnarlNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
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

    fn show_graph_menu(&mut self, pos: Pos2, ui: &mut Ui, snarl: &mut Snarl<SnarlNode>) {
        ui.label("Add node");
        for item in builtin_menu_items() {
            if ui.button(item.name).clicked() {
                self.add_node(snarl, item.kind, pos);
                ui.close();
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
        snarl: &mut Snarl<SnarlNode>,
    ) {
        if ui.button("Delete node").clicked() {
            if let Some(core_id) = self.core_node_id(snarl, node) {
                self.graph.remove_node(core_id);
                self.core_to_snarl.remove(&core_id);
                self.snarl_to_core.remove(&node);
                let _ = snarl.remove_node(node);
                if self.selected_node.as_ref() == Some(&core_id) {
                    *self.selected_node = None;
                }
                self.changed = true;
            }
            ui.close();
        }
    }

    fn final_node_rect(
        &mut self,
        node: egui_snarl::NodeId,
        ui_rect: egui::Rect,
        ui: &mut Ui,
        snarl: &mut Snarl<SnarlNode>,
    ) {
        let Some(core_id) = self.core_node_id(snarl, node) else {
            return;
        };
        if self.selected_node.as_ref() == Some(&core_id) {
            let stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(235, 200, 60));
            ui.painter()
                .rect_stroke(ui_rect, 6.0, stroke, egui::StrokeKind::Inside);
        }

        if self.error_nodes.contains(&core_id) {
            let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(220, 60, 60));
            ui.painter()
                .rect_stroke(ui_rect, 6.0, stroke, egui::StrokeKind::Inside);

            let badge_center = egui::pos2(ui_rect.right() - 8.0, ui_rect.top() + 8.0);
            let badge_rect = egui::Rect::from_center_size(badge_center, egui::vec2(12.0, 12.0));
            ui.painter()
                .circle_filled(badge_center, 5.0, egui::Color32::from_rgb(220, 60, 60));
            ui.painter().text(
                badge_center,
                egui::Align2::CENTER_CENTER,
                "!",
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
            let badge_response = ui.interact(
                badge_rect,
                ui.make_persistent_id(("node-error", node)),
                egui::Sense::hover(),
            );
            if let Some(message) = self.error_messages.get(&core_id) {
                badge_response.on_hover_text(message);
            }
        }

        let response = ui.interact(
            ui_rect,
            ui.make_persistent_id(("node-select", node)),
            egui::Sense::click(),
        );
        if response.clicked_by(egui::PointerButton::Primary) {
            *self.selected_node = Some(core_id);
        }
    }

    fn current_transform(
        &mut self,
        to_global: &mut egui::emath::TSTransform,
        _snarl: &mut Snarl<SnarlNode>,
    ) {
        if to_global.is_valid() {
            self.graph_transform.to_global = *to_global;
            self.graph_transform.valid = true;
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
            Err(core::GraphError::InputAlreadyConnected { .. }) => {
                let _ = self.graph.remove_links_for_pin(to_pin);
                snarl.drop_inputs(to.id);
                if self.graph.add_link(from_pin, to_pin).is_ok() {
                    let _ = snarl.connect(from.id, to.id);
                    self.changed = true;
                }
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
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(label);
                if ui.add(egui::DragValue::new(&mut v).speed(0.1)).changed() {
                    changed = true;
                }
                let range = float_slider_range(label, v);
                if ui
                    .add(egui::Slider::new(&mut v, range).show_value(false))
                    .changed()
                {
                    changed = true;
                }
            });
            (core::ParamValue::Float(v), changed)
        }
        core::ParamValue::Int(mut v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label(label);
                if ui.add(egui::DragValue::new(&mut v).speed(1.0)).changed() {
                    changed = true;
                }
                let range = int_slider_range(label, v);
                if ui
                    .add(egui::Slider::new(&mut v, range).show_value(false))
                    .changed()
                {
                    changed = true;
                }
            });
            (core::ParamValue::Int(v), changed)
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

fn float_slider_range(label: &str, _value: f32) -> std::ops::RangeInclusive<f32> {
    match label {
        "threshold_deg" => 0.0..=180.0,
        _ => -1000.0..=1000.0,
    }
}

fn int_slider_range(label: &str, _value: i32) -> std::ops::RangeInclusive<i32> {
    match label {
        "rows" | "cols" => 2..=64,
        _ => -1000..=1000,
    }
}

struct MenuItem {
    kind: BuiltinNodeKind,
    name: &'static str,
    category: &'static str,
}

fn builtin_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem {
            kind: BuiltinNodeKind::Box,
            name: "Box",
            category: "Sources",
        },
        MenuItem {
            kind: BuiltinNodeKind::Grid,
            name: "Grid",
            category: "Sources",
        },
        MenuItem {
            kind: BuiltinNodeKind::Sphere,
            name: "Sphere",
            category: "Sources",
        },
        MenuItem {
            kind: BuiltinNodeKind::Scatter,
            name: "Scatter",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Transform,
            name: "Transform",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Merge,
            name: "Merge",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::CopyToPoints,
            name: "Copy to Points",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Normal,
            name: "Normal",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Output,
            name: "Output",
            category: "Outputs",
        },
    ]
}
