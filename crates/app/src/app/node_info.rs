use eframe::egui;

use grapho_core::{AttributeDomain, AttributeInfo, AttributeType, Mesh};

use crate::app::GraphoApp;

pub(super) struct NodeInfoPanel {
    pub(super) node_id: grapho_core::NodeId,
    pub(super) screen_pos: egui::Pos2,
    pub(super) open: bool,
}

impl GraphoApp {
    pub(super) fn show_node_info_panel(
        &mut self,
        ctx: &egui::Context,
        panel_slot: &mut Option<NodeInfoPanel>,
    ) {
        let Some(mut panel) = panel_slot.take() else {
            return;
        };
        if !panel.open {
            return;
        }

        let Some(node) = self.project.graph.node(panel.node_id) else {
            return;
        };

        let title = format!("Node Info - {}", node.name);
        let mut open = panel.open;
        let node_id = panel.node_id;
        let window = egui::Window::new(title)
            .collapsible(true)
            .resizable(true)
            .default_pos(panel.screen_pos)
            .open(&mut open);

        window.show(ctx, |ui| {
            if let Some(mesh) = self.eval_state.mesh_for_node(node_id) {
                self.show_mesh_info(ui, mesh);
            } else {
                ui.label("No geometry available for this node.");
            }
        });

        panel.open = open;
        if panel.open {
            *panel_slot = Some(panel);
        }
    }

    fn show_mesh_info(&self, ui: &mut egui::Ui, mesh: &Mesh) {
        let point_count = mesh.positions.len();
        let vertex_count = mesh.indices.len();
        let prim_count = mesh.indices.len() / 3;
        let detail_count = if point_count == 0 && vertex_count == 0 {
            0
        } else {
            1
        };

        ui.heading("Counts");
        egui::Grid::new("node_info_counts")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Points");
                ui.label(point_count.to_string());
                ui.end_row();

                ui.label("Vertices");
                ui.label(vertex_count.to_string());
                ui.end_row();

                ui.label("Primitives");
                ui.label(prim_count.to_string());
                ui.end_row();

                ui.label("Detail");
                ui.label(detail_count.to_string());
                ui.end_row();
            });

        ui.separator();
        ui.heading("Bounds");
        if let Some(bounds) = mesh.bounds() {
            let center = [
                (bounds.min[0] + bounds.max[0]) * 0.5,
                (bounds.min[1] + bounds.max[1]) * 0.5,
                (bounds.min[2] + bounds.max[2]) * 0.5,
            ];
            let size = [
                bounds.max[0] - bounds.min[0],
                bounds.max[1] - bounds.min[1],
                bounds.max[2] - bounds.min[2],
            ];
            egui::Grid::new("node_info_bounds")
                .num_columns(4)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Center");
                    ui.label(format!("{:.3}", center[0]));
                    ui.label(format!("{:.3}", center[1]));
                    ui.label(format!("{:.3}", center[2]));
                    ui.end_row();

                    ui.label("Size");
                    ui.label(format!("{:.3}", size[0]));
                    ui.label(format!("{:.3}", size[1]));
                    ui.label(format!("{:.3}", size[2]));
                    ui.end_row();

                    ui.label("Min");
                    ui.label(format!("{:.3}", bounds.min[0]));
                    ui.label(format!("{:.3}", bounds.min[1]));
                    ui.label(format!("{:.3}", bounds.min[2]));
                    ui.end_row();

                    ui.label("Max");
                    ui.label(format!("{:.3}", bounds.max[0]));
                    ui.label(format!("{:.3}", bounds.max[1]));
                    ui.label(format!("{:.3}", bounds.max[2]));
                    ui.end_row();
                });
        } else {
            ui.label("No bounds available.");
        }

        ui.separator();
        ui.heading("Attributes");
        let mut attrs = mesh.list_attributes();
        attrs.sort_by(|a, b| {
            let domain_order = |domain| match domain {
                AttributeDomain::Vertex => 0,
                AttributeDomain::Point => 1,
                AttributeDomain::Primitive => 2,
                AttributeDomain::Detail => 3,
            };
            domain_order(a.domain)
                .cmp(&domain_order(b.domain))
                .then_with(|| a.name.cmp(&b.name))
        });
        for domain in [
            AttributeDomain::Point,
            AttributeDomain::Vertex,
            AttributeDomain::Primitive,
            AttributeDomain::Detail,
        ] {
            let group: Vec<&AttributeInfo> = attrs.iter().filter(|a| a.domain == domain).collect();
            if group.is_empty() {
                continue;
            }
            ui.label(format!(
                "{} Attributes ({})",
                attribute_domain_label(domain),
                group.len()
            ));
            egui::Grid::new(format!("node_info_attr_{:?}", domain))
                .num_columns(3)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    for attr in group {
                        let mut name = attr.name.clone();
                        if attr.implicit {
                            name.push_str(" (implicit)");
                        }
                        ui.label(name);
                        ui.label(attribute_type_label(attr.data_type));
                        ui.label(attr.len.to_string());
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
        }
    }
}

fn attribute_type_label(ty: AttributeType) -> &'static str {
    match ty {
        AttributeType::Float => "Flt",
        AttributeType::Int => "Int",
        AttributeType::Vec2 => "2-Flt",
        AttributeType::Vec3 => "3-Flt",
        AttributeType::Vec4 => "4-Flt",
    }
}

fn attribute_domain_label(domain: AttributeDomain) -> &'static str {
    match domain {
        AttributeDomain::Point => "Point",
        AttributeDomain::Vertex => "Vertex",
        AttributeDomain::Primitive => "Primitive",
        AttributeDomain::Detail => "Detail",
    }
}
