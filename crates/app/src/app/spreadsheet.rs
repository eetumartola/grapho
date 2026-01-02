use egui::{Color32, RichText, Ui};
use grapho_core::{AttributeDomain, AttributeRef, AttributeType, Mesh};

pub(super) fn show_spreadsheet(
    ui: &mut Ui,
    mesh: Option<&Mesh>,
    domain: &mut AttributeDomain,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Spreadsheet")
                .color(Color32::from_rgb(220, 220, 220))
                .strong(),
        );
        for (label, value) in [
            ("Point", AttributeDomain::Point),
            ("Vertex", AttributeDomain::Vertex),
            ("Prim", AttributeDomain::Primitive),
            ("Detail", AttributeDomain::Detail),
        ] {
            if ui.selectable_label(*domain == value, label).clicked() {
                *domain = value;
            }
        }
    });
    ui.separator();

    let Some(mesh) = mesh else {
        ui.label("No mesh selected.");
        return;
    };

    let count = mesh.attribute_domain_len(*domain);
    if count == 0 {
        ui.label("No elements in this domain.");
        return;
    }

    let mut attrs: Vec<_> = mesh
        .list_attributes()
        .into_iter()
        .filter(|attr| attr.domain == *domain)
        .collect();
    attrs.sort_by(|a, b| a.name.cmp(&b.name));

    if attrs.is_empty() {
        ui.label("No attributes in this domain.");
        return;
    }

    let max_rows = count.min(64);
    let header_text = format!(
        "{} elements, {} attributes (showing {} rows)",
        count,
        attrs.len(),
        max_rows
    );
    ui.label(header_text);
    ui.add_space(6.0);

    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("attribute_spreadsheet")
            .striped(true)
            .spacing([14.0, 6.0])
            .show(ui, |ui| {
                ui.label("idx");
                for attr in &attrs {
                    let label = format!("{} {}", attr.name, attr_type_label(attr.data_type));
                    ui.label(label);
                }
                ui.end_row();

                for row in 0..max_rows {
                    ui.monospace(format!("{row}"));
                    for attr in &attrs {
                        let value = mesh
                            .attribute(*domain, &attr.name)
                            .and_then(|values| format_attr_value(values, row));
                        if let Some(value) = value {
                            ui.monospace(value);
                        } else {
                            ui.label("-");
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn attr_type_label(attr_type: AttributeType) -> &'static str {
    match attr_type {
        AttributeType::Float => "f",
        AttributeType::Int => "i",
        AttributeType::Vec2 => "v2",
        AttributeType::Vec3 => "v3",
        AttributeType::Vec4 => "v4",
    }
}

fn format_attr_value(values: AttributeRef<'_>, index: usize) -> Option<String> {
    match values {
        AttributeRef::Float(data) => data.get(index).map(|v| format!("{v:.3}")),
        AttributeRef::Int(data) => data.get(index).map(|v| v.to_string()),
        AttributeRef::Vec2(data) => data
            .get(index)
            .map(|v| format!("{:.3}, {:.3}", v[0], v[1])),
        AttributeRef::Vec3(data) => data
            .get(index)
            .map(|v| format!("{:.3}, {:.3}, {:.3}", v[0], v[1], v[2])),
        AttributeRef::Vec4(data) => data.get(index).map(|v| {
            format!("{:.3}, {:.3}, {:.3}, {:.3}", v[0], v[1], v[2], v[3])
        }),
    }
}
