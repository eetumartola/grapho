use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use egui::{vec2, Color32, Frame, Pos2, Rect, Stroke, Ui};
use egui_snarl::ui::{BackgroundPattern, SnarlStyle};
use egui_snarl::{InPinId, OutPinId, Snarl};

use grapho_core::{BuiltinNodeKind, Graph, NodeId, PinId, PinKind};

use super::menu::builtin_menu_items;
use super::params::edit_param;
use super::utils::{
    add_builtin_node, find_input_of_type, find_output_of_type, point_snarl_wire_distance,
};
use super::viewer::NodeGraphViewer;

#[derive(Clone, Copy)]
pub(super) struct SnarlNode {
    pub(super) core_id: NodeId,
}

pub struct NodeGraphState {
    snarl: Snarl<SnarlNode>,
    core_to_snarl: HashMap<NodeId, egui_snarl::NodeId>,
    snarl_to_core: HashMap<egui_snarl::NodeId, NodeId>,
    next_pos: Pos2,
    needs_wire_sync: bool,
    selected_node: Option<NodeId>,
    node_ui_rects: HashMap<egui_snarl::NodeId, Rect>,
    prev_node_ui_rects: HashMap<egui_snarl::NodeId, Rect>,
    header_button_rects: HashMap<egui_snarl::NodeId, (Rect, Rect)>,
    dragging_node: Option<egui_snarl::NodeId>,
    add_menu_open: bool,
    add_menu_screen_pos: Pos2,
    add_menu_graph_pos: Pos2,
    add_menu_filter: String,
    add_menu_focus: bool,
    pending_wire: Option<PendingWire>,
    info_request: Option<NodeInfoRequest>,
    graph_transform: GraphTransformState,
    input_pin_positions: Rc<RefCell<HashMap<InPinId, Pos2>>>,
    output_pin_positions: Rc<RefCell<HashMap<OutPinId, Pos2>>>,
    error_nodes: HashSet<NodeId>,
    error_messages: HashMap<NodeId, String>,
    node_menu_request: Option<NodeMenuRequest>,
    node_menu_open: bool,
    node_menu_screen_pos: Pos2,
    node_menu_node: Option<NodeId>,
    last_changed: bool,
    layout_changed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct GraphTransformState {
    pub(super) to_global: egui::emath::TSTransform,
    pub(super) valid: bool,
}

#[derive(Clone, Copy)]
pub struct NodeInfoRequest {
    pub node_id: NodeId,
    pub screen_pos: Pos2,
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
            node_ui_rects: HashMap::new(),
            prev_node_ui_rects: HashMap::new(),
            header_button_rects: HashMap::new(),
            dragging_node: None,
            add_menu_open: false,
            add_menu_screen_pos: Pos2::new(0.0, 0.0),
            add_menu_graph_pos: Pos2::new(0.0, 0.0),
            add_menu_filter: String::new(),
            add_menu_focus: false,
            pending_wire: None,
            info_request: None,
            graph_transform: GraphTransformState {
                to_global: egui::emath::TSTransform::IDENTITY,
                valid: false,
            },
            input_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            output_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            error_nodes: HashSet::new(),
            error_messages: HashMap::new(),
            node_menu_request: None,
            node_menu_open: false,
            node_menu_screen_pos: Pos2::new(0.0, 0.0),
            node_menu_node: None,
            last_changed: false,
            layout_changed: false,
        }
    }
}

pub(super) struct NodeMenuRequest {
    pub(super) node_id: NodeId,
    pub(super) screen_pos: Pos2,
}

#[derive(Clone, Default, PartialEq)]
pub struct NodeGraphLayout {
    pub positions: HashMap<NodeId, Pos2>,
    pub selected: Option<NodeId>,
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

        self.prev_node_ui_rects = std::mem::take(&mut self.node_ui_rects);
        self.node_ui_rects.clear();
        self.header_button_rects.clear();
        self.input_pin_positions.borrow_mut().clear();
        self.output_pin_positions.borrow_mut().clear();
        self.last_changed = false;
        self.layout_changed = false;

        let mut viewer = NodeGraphViewer {
            graph,
            core_to_snarl: &mut self.core_to_snarl,
            snarl_to_core: &mut self.snarl_to_core,
            next_pos: &mut self.next_pos,
            selected_node: &mut self.selected_node,
            node_rects: &mut self.node_ui_rects,
            header_button_rects: &mut self.header_button_rects,
            graph_transform: &mut self.graph_transform,
            input_pin_positions: Rc::clone(&self.input_pin_positions),
            output_pin_positions: Rc::clone(&self.output_pin_positions),
            add_menu_open: &mut self.add_menu_open,
            add_menu_screen_pos: &mut self.add_menu_screen_pos,
            add_menu_graph_pos: &mut self.add_menu_graph_pos,
            add_menu_filter: &mut self.add_menu_filter,
            add_menu_focus: &mut self.add_menu_focus,
            pending_wire: &mut self.pending_wire,
            node_menu_request: &mut self.node_menu_request,
            error_nodes: &self.error_nodes,
            error_messages: &self.error_messages,
            changed: false,
        };
        let style = SnarlStyle {
            pin_size: Some(10.0),
            bg_frame: Some(Frame::NONE.fill(Color32::from_rgb(18, 18, 18))),
            bg_pattern: Some(BackgroundPattern::grid(vec2(64.0, 64.0), 0.0)),
            bg_pattern_stroke: Some(Stroke::new(1.0, Color32::from_rgb(26, 26, 26))),
            collapsible: Some(false),
            ..SnarlStyle::default()
        };
        self.snarl.show(&mut viewer, &style, "node_graph", ui);
        let viewer_changed = viewer.changed;
        self.last_changed |= viewer_changed;
        drop(viewer);
        for (node, rect) in &self.node_ui_rects {
            let Some(prev) = self.prev_node_ui_rects.get(node) else {
                continue;
            };
            if (rect.center() - prev.center()).length_sq() > 0.5 {
                self.layout_changed = true;
                break;
            }
        }

        if viewer_changed {
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        if self.add_menu_open && self.show_add_menu(ui, graph) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        self.update_drag_state(ui);

        if self.handle_drop_on_wire(ui, graph) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        if let Some(request) = self.node_menu_request.take() {
            self.node_menu_open = true;
            self.node_menu_node = Some(request.node_id);
            self.node_menu_screen_pos = request.screen_pos;
        }
        if self.node_menu_open && self.show_node_menu(ui, graph) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }
    }

    pub fn take_changed(&mut self) -> bool {
        let changed = self.last_changed;
        self.last_changed = false;
        changed
    }

    pub fn take_layout_changed(&mut self) -> bool {
        let changed = self.layout_changed;
        self.layout_changed = false;
        changed
    }

    pub fn layout_snapshot(&self) -> NodeGraphLayout {
        let mut positions = HashMap::new();
        for (_, pos, node) in self.snarl.nodes_pos_ids() {
            positions.insert(node.core_id, pos);
        }
        NodeGraphLayout {
            positions,
            selected: self.selected_node,
        }
    }

    pub fn restore_layout(&mut self, graph: &Graph, layout: &NodeGraphLayout) {
        self.reset();
        for node in graph.nodes() {
            let pos = layout
                .positions
                .get(&node.id)
                .copied()
                .unwrap_or(self.next_pos);
            let snarl_id = self.snarl.insert_node(pos, SnarlNode { core_id: node.id });
            self.core_to_snarl.insert(node.id, snarl_id);
            self.snarl_to_core.insert(snarl_id, node.id);
            self.advance_pos();
        }
        self.selected_node = layout
            .selected
            .filter(|selected| graph.node(*selected).is_some());
        self.needs_wire_sync = true;
    }

    fn show_node_menu(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        let mut close_menu = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut menu_rect = None;
        let mut changed = false;
        let node_id = self.node_menu_node;

        let response = egui::Window::new("node_context_menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, self.node_menu_screen_pos.to_vec2())
            .frame(Frame::popup(ui.style()))
            .show(ui.ctx(), |ui| {
                if ui.button("Node info").clicked() {
                    if let Some(node_id) = node_id {
                        self.info_request = Some(NodeInfoRequest {
                            node_id,
                            screen_pos: self.node_menu_screen_pos,
                        });
                    }
                    close_menu = true;
                }
                if ui.button("Delete node").clicked() {
                    if let Some(node_id) = node_id {
                        graph.remove_node(node_id);
                        if let Some(snarl_id) = self.core_to_snarl.remove(&node_id) {
                            self.snarl_to_core.remove(&snarl_id);
                            let _ = self.snarl.remove_node(snarl_id);
                        }
                        if self.selected_node.as_ref() == Some(&node_id) {
                            self.selected_node = None;
                        }
                        changed = true;
                    }
                    close_menu = true;
                }
            });

        if let Some(inner) = response {
            menu_rect = Some(inner.response.rect);
        }

        if !close_menu {
            if let Some(rect) = menu_rect {
                if ui.input(|i| i.pointer.any_pressed()) {
                    let hover = ui.input(|i| i.pointer.hover_pos());
                    if hover.is_none_or(|pos| !rect.contains(pos)) {
                        close_menu = true;
                    }
                }
            }
        }

        if close_menu {
            self.node_menu_open = false;
            self.node_menu_node = None;
        }

        changed
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
            .and_then(|node| node.outputs.first().copied());
        let transform_in = graph
            .node(transform_id)
            .and_then(|node| node.inputs.first().copied());
        let transform_out = graph
            .node(transform_id)
            .and_then(|node| node.outputs.first().copied());
        let output_in = graph
            .node(output_id)
            .and_then(|node| node.inputs.first().copied());

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

        let params: Vec<(String, grapho_core::ParamValue)> = node
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
            if did_change && graph.set_param(node_id, key, next_value).is_ok() {
                changed = true;
            }
        }

        changed
    }

    pub fn inspector_row_count(&self, graph: &Graph) -> usize {
        let Some(node_id) = self.selected_node else {
            return 1;
        };
        let Some(node) = graph.node(node_id) else {
            return 1;
        };
        let count = node.params.values.len();
        if count == 0 {
            1
        } else {
            count
        }
    }

    pub fn set_error_state(&mut self, nodes: HashSet<NodeId>, messages: HashMap<NodeId, String>) {
        self.error_nodes = nodes;
        self.error_messages = messages;
    }

    pub fn selected_node_id(&self) -> Option<NodeId> {
        self.selected_node
    }

    pub fn node_at_screen_pos(&self, pos: Pos2) -> Option<NodeId> {
        let snarl_node = self.node_at_pos(pos)?;
        self.snarl_to_core.get(&snarl_node).copied()
    }

    pub fn take_info_request(&mut self) -> Option<NodeInfoRequest> {
        self.info_request.take()
    }

    fn show_add_menu(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        let mut close_menu = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut menu_rect = None;
        let activate_first = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut changed = false;

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
                        if let Some(core_id) =
                            self.try_add_node(graph, item.kind, self.add_menu_graph_pos)
                        {
                            changed = true;
                            if let Some(pending) = self.pending_wire.take() {
                                if self.connect_pending_wire(graph, core_id, pending) {
                                    changed = true;
                                }
                            }
                        }
                        close_menu = true;
                    }
                }
                if !matched {
                    ui.label("No matches.");
                } else if activate_first {
                    if let Some(kind) = first_match {
                        if let Some(core_id) =
                            self.try_add_node(graph, kind, self.add_menu_graph_pos)
                        {
                            changed = true;
                            if let Some(pending) = self.pending_wire.take() {
                                if self.connect_pending_wire(graph, core_id, pending) {
                                    changed = true;
                                }
                            }
                        }
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
                    if hover.is_none_or(|pos| !rect.contains(pos)) {
                        close_menu = true;
                    }
                }
            }
        }

        if close_menu {
            self.add_menu_open = false;
            self.pending_wire = None;
        }

        changed
    }

    fn try_add_node(
        &mut self,
        graph: &mut Graph,
        kind: BuiltinNodeKind,
        pos: Pos2,
    ) -> Option<NodeId> {
        if kind == BuiltinNodeKind::Output && graph.nodes().any(|node| node.name == "Output") {
            tracing::warn!("Only one Output node is supported right now.");
            return None;
        }

        let core_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            kind,
            pos,
        );
        self.needs_wire_sync = true;
        Some(core_id)
    }

    fn update_drag_state(&mut self, ui: &Ui) {
        let pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if pressed {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                if let Some(node) = self.node_at_pos(pos) {
                    self.dragging_node = Some(node);
                }
            }
        }
        if pointer_down {
            if let Some(node) = self.find_moved_node() {
                self.dragging_node = Some(node);
            }
        } else if !ui.input(|i| i.pointer.any_released()) {
            self.dragging_node = None;
        }
    }

    fn handle_drop_on_wire(&mut self, ui: &Ui, graph: &mut Graph) -> bool {
        if !ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            return false;
        }
        let Some(drop_pos) =
            ui.input(|i| i.pointer.latest_pos().or_else(|| i.pointer.interact_pos()))
        else {
            return false;
        };
        let moved_node = self.dragging_node.or_else(|| self.find_moved_node());
        if moved_node.is_none() {
            return false;
        }
        let moved_node = moved_node.unwrap();
        self.dragging_node = None;
        let (wire_hit, _wire_dist) = self.find_wire_hit_with_dist(graph, drop_pos);
        let Some((out_pin, in_pin)) = wire_hit else {
            return false;
        };
        if out_pin.node == moved_node || in_pin.node == moved_node {
            return false;
        }
        self.insert_node_between_wire(graph, moved_node, out_pin, in_pin)
    }

    fn find_moved_node(&self) -> Option<egui_snarl::NodeId> {
        let mut best = None;
        let mut best_dist = 0.0;
        for (node, rect) in &self.node_ui_rects {
            let Some(prev) = self.prev_node_ui_rects.get(node) else {
                continue;
            };
            let delta = rect.center() - prev.center();
            let dist = delta.length_sq();
            if dist > best_dist {
                best_dist = dist;
                best = Some(*node);
            }
        }
        if best_dist > 0.5 {
            best
        } else {
            None
        }
    }

    fn node_at_pos(&self, pos: Pos2) -> Option<egui_snarl::NodeId> {
        let mut best = None;
        let mut best_area = f32::MAX;
        for (node, rect) in &self.node_ui_rects {
            if rect.contains(pos) {
                let area = rect.width() * rect.height();
                if area < best_area {
                    best_area = area;
                    best = Some(*node);
                }
            }
        }
        best
    }

    fn find_wire_hit_with_dist(
        &self,
        graph: &Graph,
        pos: Pos2,
    ) -> (Option<(OutPinId, InPinId)>, f32) {
        let mut best = None;
        let mut best_dist = f32::MAX;
        let threshold = 28.0;
        for (out_pin, in_pin) in self.snarl.wires() {
            let Some(out_pos) = self.pin_pos_for_output(graph, out_pin) else {
                continue;
            };
            let Some(in_pos) = self.pin_pos_for_input(graph, in_pin) else {
                continue;
            };
            let dist = point_snarl_wire_distance(pos, out_pos, in_pos);
            if dist < threshold && dist < best_dist {
                best = Some((out_pin, in_pin));
                best_dist = dist;
            }
        }
        (best, best_dist)
    }

    fn pin_pos_for_output(&self, graph: &Graph, pin: OutPinId) -> Option<Pos2> {
        if let Some(pos) = self.output_pin_positions.borrow().get(&pin).copied() {
            return Some(pos);
        }
        let rect = self.node_ui_rects.get(&pin.node)?;
        let core_id = *self.snarl_to_core.get(&pin.node)?;
        let node = graph.node(core_id)?;
        let count = node.outputs.len().max(1);
        let t = (pin.output as f32 + 1.0) / (count as f32 + 1.0);
        Some(Pos2::new(rect.right(), rect.top() + rect.height() * t))
    }

    fn pin_pos_for_input(&self, graph: &Graph, pin: InPinId) -> Option<Pos2> {
        if let Some(pos) = self.input_pin_positions.borrow().get(&pin).copied() {
            return Some(pos);
        }
        let rect = self.node_ui_rects.get(&pin.node)?;
        let core_id = *self.snarl_to_core.get(&pin.node)?;
        let node = graph.node(core_id)?;
        let count = node.inputs.len().max(1);
        let t = (pin.input as f32 + 1.0) / (count as f32 + 1.0);
        Some(Pos2::new(rect.left(), rect.top() + rect.height() * t))
    }

    fn insert_node_between_wire(
        &mut self,
        graph: &mut Graph,
        node: egui_snarl::NodeId,
        out_pin: OutPinId,
        in_pin: InPinId,
    ) -> bool {
        let Some(core_out) = self.core_pin_for_output(graph, out_pin) else {
            return false;
        };
        let Some(core_in) = self.core_pin_for_input(graph, in_pin) else {
            return false;
        };
        let Some(core_node) = self.snarl_to_core.get(&node).copied() else {
            return false;
        };
        let Some(out_pin_data) = graph.pin(core_out) else {
            return false;
        };
        let Some(in_pin_data) = graph.pin(core_in) else {
            return false;
        };
        let Some(node_data) = graph.node(core_node) else {
            return false;
        };
        let Some((new_in_pin, new_in_idx)) =
            find_input_of_type(graph, node_data, out_pin_data.pin_type)
        else {
            return false;
        };
        let Some((new_out_pin, new_out_idx)) =
            find_output_of_type(graph, node_data, in_pin_data.pin_type)
        else {
            return false;
        };

        let _ = graph.remove_link_between(core_out, core_in);
        let _ = self.snarl.disconnect(out_pin, in_pin);

        let new_in_snarl = InPinId {
            node,
            input: new_in_idx,
        };
        let new_out_snarl = OutPinId {
            node,
            output: new_out_idx,
        };

        if graph.add_link(core_out, new_in_pin).is_err() {
            let _ = graph.remove_links_for_pin(new_in_pin);
            self.snarl.drop_inputs(new_in_snarl);
            let _ = graph.add_link(core_out, new_in_pin);
        }
        let _ = self.snarl.connect(out_pin, new_in_snarl);

        if graph.add_link(new_out_pin, core_in).is_err() {
            let _ = graph.remove_links_for_pin(core_in);
            self.snarl.drop_inputs(in_pin);
            let _ = graph.add_link(new_out_pin, core_in);
        }
        let _ = self.snarl.connect(new_out_snarl, in_pin);
        true
    }

    fn connect_pending_wire(
        &mut self,
        graph: &mut Graph,
        new_node: NodeId,
        pending: PendingWire,
    ) -> bool {
        let Some(snarl_node) = self.core_to_snarl.get(&new_node).copied() else {
            return false;
        };
        let Some(node_data) = graph.node(new_node) else {
            return false;
        };
        match pending {
            PendingWire::FromOutputs(out_pins) => {
                for out_pin in out_pins {
                    let Some(core_out) = self.core_pin_for_output(graph, out_pin) else {
                        continue;
                    };
                    let Some(pin_data) = graph.pin(core_out) else {
                        continue;
                    };
                    let Some((new_in_pin, new_in_idx)) =
                        find_input_of_type(graph, node_data, pin_data.pin_type)
                    else {
                        continue;
                    };
                    let new_in_snarl = InPinId {
                        node: snarl_node,
                        input: new_in_idx,
                    };
                    if graph.add_link(core_out, new_in_pin).is_err() {
                        let _ = graph.remove_links_for_pin(new_in_pin);
                        self.snarl.drop_inputs(new_in_snarl);
                        let _ = graph.add_link(core_out, new_in_pin);
                    }
                    let _ = self.snarl.connect(out_pin, new_in_snarl);
                    return true;
                }
                false
            }
            PendingWire::FromInputs(in_pins) => {
                let mut target_type = None;
                for in_pin in &in_pins {
                    if let Some(core_in) = self.core_pin_for_input(graph, *in_pin) {
                        if let Some(pin_data) = graph.pin(core_in) {
                            target_type = Some(pin_data.pin_type);
                            break;
                        }
                    }
                }
                let Some(target_type) = target_type else {
                    return false;
                };
                let Some((new_out_pin, new_out_idx)) =
                    find_output_of_type(graph, node_data, target_type)
                else {
                    return false;
                };
                let new_out_snarl = OutPinId {
                    node: snarl_node,
                    output: new_out_idx,
                };
                let mut connected = false;
                for in_pin in in_pins {
                    let Some(core_in) = self.core_pin_for_input(graph, in_pin) else {
                        continue;
                    };
                    let Some(pin_data) = graph.pin(core_in) else {
                        continue;
                    };
                    if pin_data.pin_type != target_type {
                        continue;
                    }
                    let _ = graph.remove_links_for_pin(core_in);
                    self.snarl.drop_inputs(in_pin);
                    if graph.add_link(new_out_pin, core_in).is_ok() {
                        let _ = self.snarl.connect(new_out_snarl, in_pin);
                        connected = true;
                    }
                }
                connected
            }
        }
    }

    fn core_pin_for_input(&self, graph: &Graph, pin: InPinId) -> Option<PinId> {
        let core_node = self.snarl_to_core.get(&pin.node).copied()?;
        let node = graph.node(core_node)?;
        node.inputs.get(pin.input).copied()
    }

    fn core_pin_for_output(&self, graph: &Graph, pin: OutPinId) -> Option<PinId> {
        let core_node = self.snarl_to_core.get(&pin.node).copied()?;
        let node = graph.node(core_node)?;
        node.outputs.get(pin.output).copied()
    }
}

pub(super) enum PendingWire {
    FromOutputs(Vec<OutPinId>),
    FromInputs(Vec<InPinId>),
}
