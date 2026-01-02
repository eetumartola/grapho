use egui::{Align2, Color32, FontId, RichText, Ui};
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

    let max_rows = count.min(128);

    let font_id = FontId::monospace(13.0);
    let char_width = ui.fonts_mut(|f| f.glyph_width(&font_id, '0'));

    let mut columns = build_columns(mesh, *domain, &attrs, max_rows);
    for column in &mut columns {
        column.finalize();
    }

    let idx_width = (max_rows.saturating_sub(1)).to_string().len().max(3);
    let idx_width = (idx_width as f32 * char_width + 12.0).max(36.0);
    let row_height = 24.0;

    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        egui::Grid::new("attribute_spreadsheet_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                draw_cell(ui, "idx", idx_width, row_height, Align2::LEFT_CENTER, true, &font_id);
                for column in &columns {
                    draw_cell(
                        ui,
                        &column.header,
                        column.pixel_width(char_width),
                        row_height,
                        Align2::LEFT_CENTER,
                        true,
                        &font_id,
                    );
                }
                ui.end_row();

                for row in 0..max_rows {
                    draw_cell(
                        ui,
                        &row.to_string(),
                        idx_width,
                        row_height,
                        Align2::RIGHT_CENTER,
                        false,
                        &font_id,
                    );
                    for column in &columns {
                        let value = column
                            .formatted
                            .get(row)
                            .map(String::as_str)
                            .unwrap_or("-");
                        draw_cell(
                            ui,
                            value,
                            column.pixel_width(char_width),
                            row_height,
                            Align2::RIGHT_CENTER,
                            false,
                            &font_id,
                        );
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

struct Column {
    header: String,
    kind: ColumnKind,
    formatted: Vec<String>,
    width_chars: usize,
}

enum ColumnKind {
    Float(Vec<Option<f32>>),
    Int(Vec<Option<i32>>),
}

impl Column {
    fn finalize(&mut self) {
        match &self.kind {
            ColumnKind::Float(values) => {
                let mut max_int = 1usize;
                let mut has_negative = false;
                for value in values.iter().flatten() {
                    if *value < 0.0 {
                        has_negative = true;
                    }
                    let formatted = format!("{:.3}", value.abs());
                    let int_len = formatted
                        .split('.')
                        .next()
                        .map(|part| part.len())
                        .unwrap_or(1);
                    max_int = max_int.max(int_len);
                }
                self.formatted = values
                    .iter()
                    .map(|value| match value {
                        Some(value) => format_float_cell(*value, max_int, has_negative),
                        None => "-".to_string(),
                    })
                    .collect();
            }
            ColumnKind::Int(values) => {
                let mut max_int = 1usize;
                let mut has_negative = false;
                for value in values.iter().flatten() {
                    if *value < 0 {
                        has_negative = true;
                    }
                    let len = value.abs().to_string().len();
                    max_int = max_int.max(len);
                }
                self.formatted = values
                    .iter()
                    .map(|value| match value {
                        Some(value) => format_int_cell(*value, max_int, has_negative),
                        None => "-".to_string(),
                    })
                    .collect();
            }
        }
        self.width_chars = self
            .formatted
            .iter()
            .map(|value| value.len())
            .chain(std::iter::once(self.header.len()))
            .max()
            .unwrap_or(4)
            .max(4);
    }

    fn pixel_width(&self, char_width: f32) -> f32 {
        self.width_chars as f32 * char_width + 14.0
    }
}

fn build_columns(
    mesh: &Mesh,
    domain: AttributeDomain,
    attrs: &[grapho_core::AttributeInfo],
    max_rows: usize,
) -> Vec<Column> {
    let mut columns = Vec::new();
    for attr in attrs {
        let Some(values) = mesh.attribute(domain, &attr.name) else {
            continue;
        };
        match values {
            AttributeRef::Float(data) => {
                columns.push(Column {
                    header: format!("{} {}", attr.name, attr_type_label(attr.data_type)),
                    kind: ColumnKind::Float(
                        (0..max_rows).map(|idx| data.get(idx).copied()).collect(),
                    ),
                    formatted: Vec::new(),
                    width_chars: 0,
                });
            }
            AttributeRef::Int(data) => {
                columns.push(Column {
                    header: format!("{} {}", attr.name, attr_type_label(attr.data_type)),
                    kind: ColumnKind::Int(
                        (0..max_rows).map(|idx| data.get(idx).copied()).collect(),
                    ),
                    formatted: Vec::new(),
                    width_chars: 0,
                });
            }
            AttributeRef::Vec2(data) => {
                for (axis, idx) in [('x', 0usize), ('y', 1)] {
                    columns.push(Column {
                        header: format!("{}{}", attr.name, axis),
                        kind: ColumnKind::Float(
                            (0..max_rows).map(|row| data.get(row).map(|v| v[idx])).collect(),
                        ),
                        formatted: Vec::new(),
                        width_chars: 0,
                    });
                }
            }
            AttributeRef::Vec3(data) => {
                for (axis, idx) in [('x', 0usize), ('y', 1), ('z', 2)] {
                    columns.push(Column {
                        header: format!("{}{}", attr.name, axis),
                        kind: ColumnKind::Float(
                            (0..max_rows).map(|row| data.get(row).map(|v| v[idx])).collect(),
                        ),
                        formatted: Vec::new(),
                        width_chars: 0,
                    });
                }
            }
            AttributeRef::Vec4(data) => {
                for (axis, idx) in [('x', 0usize), ('y', 1), ('z', 2), ('w', 3)] {
                    columns.push(Column {
                        header: format!("{}{}", attr.name, axis),
                        kind: ColumnKind::Float(
                            (0..max_rows).map(|row| data.get(row).map(|v| v[idx])).collect(),
                        ),
                        formatted: Vec::new(),
                        width_chars: 0,
                    });
                }
            }
        }
    }
    columns
}

fn format_float_cell(value: f32, int_width: usize, has_negative: bool) -> String {
    let sign = if value < 0.0 {
        "-"
    } else if has_negative {
        " "
    } else {
        ""
    };
    let formatted = format!("{:.3}", value.abs());
    let (int_part, frac_part) = formatted
        .split_once('.')
        .unwrap_or((&formatted, "000"));
    let pad = int_width.saturating_sub(int_part.len());
    format!("{sign}{}{}.{frac_part}", " ".repeat(pad), int_part)
}

fn format_int_cell(value: i32, int_width: usize, has_negative: bool) -> String {
    let sign = if value < 0 {
        "-"
    } else if has_negative {
        " "
    } else {
        ""
    };
    let int_part = value.abs().to_string();
    let pad = int_width.saturating_sub(int_part.len());
    format!("{sign}{}{}", " ".repeat(pad), int_part)
}

fn draw_cell(
    ui: &mut Ui,
    text: &str,
    width: f32,
    height: f32,
    align: Align2,
    header: bool,
    font: &FontId,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let bg = if header {
        Color32::from_rgb(48, 48, 48)
    } else {
        Color32::from_rgb(42, 42, 42)
    };
    let stroke = Color32::from_rgb(60, 60, 60);
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    let padding = egui::vec2(6.0, 0.0);
    let pos = match align {
        Align2::LEFT_CENTER => rect.left_center() + padding,
        Align2::RIGHT_CENTER => rect.right_center() - padding,
        _ => rect.center(),
    };
    painter.text(pos, align, text, font.clone(), Color32::from_rgb(230, 230, 230));
}
