use std::collections::{HashMap, HashSet};

use egui::{Pos2, Rect, Ui};
use egui_snarl::ui::{AnyPins, PinInfo, SnarlViewer};
use egui_snarl::{InPinId, OutPinId, Snarl};

use core::{default_params, node_definition, BuiltinNodeKind, Graph, NodeId, PinId};
use tracing;

use super::menu::builtin_menu_items;
use super::state::{GraphTransformState, PendingWire, SnarlNode};
use super::utils::pin_color;

pub(super) struct NodeGraphViewer<'a> {
    pub(super) graph: &'a mut Graph,
    pub(super) core_to_snarl: &'a mut HashMap<NodeId, egui_snarl::NodeId>,
    pub(super) snarl_to_core: &'a mut HashMap<egui_snarl::NodeId, NodeId>,
    pub(super) next_pos: &'a mut Pos2,
    pub(super) selected_node: &'a mut Option<NodeId>,
    pub(super) node_rects: &'a mut HashMap<egui_snarl::NodeId, Rect>,
    pub(super) graph_transform: &'a mut GraphTransformState,
    pub(super) add_menu_open: &'a mut bool,
    pub(super) add_menu_screen_pos: &'a mut Pos2,
    pub(super) add_menu_graph_pos: &'a mut Pos2,
    pub(super) add_menu_filter: &'a mut String,
    pub(super) add_menu_focus: &'a mut bool,
    pub(super) pending_wire: &'a mut Option<PendingWire>,
    pub(super) error_nodes: &'a HashSet<NodeId>,
    pub(super) error_messages: &'a HashMap<NodeId, String>,
    pub(super) changed: bool,
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
        if kind == BuiltinNodeKind::Output && self.graph.nodes().any(|node| node.name == "Output") {
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

    fn has_dropped_wire_menu(&mut self, _src_pins: AnyPins, _snarl: &mut Snarl<SnarlNode>) -> bool {
        true
    }

    fn show_dropped_wire_menu(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        src_pins: AnyPins,
        _snarl: &mut Snarl<SnarlNode>,
    ) {
        let pending = match src_pins {
            AnyPins::Out(pins) => PendingWire::FromOutputs(pins.to_vec()),
            AnyPins::In(pins) => PendingWire::FromInputs(pins.to_vec()),
        };
        *self.pending_wire = Some(pending);
        *self.add_menu_open = true;
        *self.add_menu_screen_pos = ui
            .ctx()
            .input(|i| i.pointer.hover_pos())
            .unwrap_or(ui.cursor().min);
        *self.add_menu_graph_pos = pos;
        self.add_menu_filter.clear();
        *self.add_menu_focus = true;
        ui.close();
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

        self.node_rects.insert(node, ui_rect);

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
