use egui::Ui;

use core::ParamValue;

pub(super) fn edit_param(ui: &mut Ui, label: &str, value: ParamValue) -> (ParamValue, bool) {
    match value {
        ParamValue::Float(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let value_width = 72.0;
                let height = ui.spacing().interact_size.y;
                if ui
                    .add_sized(
                        [value_width, height],
                        egui::DragValue::new(&mut v).speed(0.1),
                    )
                    .changed()
                {
                    changed = true;
                }
                let range = float_slider_range(label, v);
                ui.add_space(spacing);
                let slider_width = ui.available_width().max(120.0);
                if ui
                    .add_sized(
                        [slider_width, height],
                        egui::Slider::new(&mut v, range).show_value(false),
                    )
                    .changed()
                {
                    changed = true;
                }
                changed
            });
            (ParamValue::Float(v), changed)
        }
        ParamValue::Int(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let value_width = 64.0;
                let height = ui.spacing().interact_size.y;
                if ui
                    .add_sized(
                        [value_width, height],
                        egui::DragValue::new(&mut v).speed(1.0),
                    )
                    .changed()
                {
                    changed = true;
                }
                let range = int_slider_range(label, v);
                ui.add_space(spacing);
                let slider_width = ui.available_width().max(120.0);
                if ui
                    .add_sized(
                        [slider_width, height],
                        egui::Slider::new(&mut v, range).show_value(false),
                    )
                    .changed()
                {
                    changed = true;
                }
                changed
            });
            (ParamValue::Int(v), changed)
        }
        ParamValue::Bool(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let checkbox = egui::Checkbox::without_text(&mut v);
                ui.add(checkbox).changed()
            });
            (ParamValue::Bool(v), changed)
        }
        ParamValue::Vec2(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let available = ui.available_width();
                let value_width = ((available - spacing) / 2.0).clamp(56.0, 120.0);
                let height = ui.spacing().interact_size.y;
                for idx in 0..2 {
                    if ui
                        .add_sized(
                            [value_width, height],
                            egui::DragValue::new(&mut v[idx]).speed(0.1),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if idx < 1 {
                        ui.add_space(spacing);
                    }
                }
                changed
            });
            (ParamValue::Vec2(v), changed)
        }
        ParamValue::Vec3(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let available = ui.available_width();
                let value_width = ((available - spacing * 2.0) / 3.0).clamp(52.0, 110.0);
                let height = ui.spacing().interact_size.y;
                for idx in 0..3 {
                    if ui
                        .add_sized(
                            [value_width, height],
                            egui::DragValue::new(&mut v[idx]).speed(0.1),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if idx < 2 {
                        ui.add_space(spacing);
                    }
                }
                changed
            });
            (ParamValue::Vec3(v), changed)
        }
        ParamValue::String(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let height = ui.spacing().interact_size.y;
                ui.add_sized(
                    [ui.available_width().max(160.0), height],
                    egui::TextEdit::singleline(&mut v),
                )
                .changed()
            });
            (ParamValue::String(v), changed)
        }
    }
}

fn param_row(ui: &mut Ui, label: &str, add_controls: impl FnOnce(&mut Ui) -> bool) -> bool {
    let total_width = ui.available_width();
    let row_height = 36.0;
    let label_width = (total_width * 0.2).clamp(80.0, 160.0);
    let controls_width = (total_width - label_width).max(120.0);
    let mut changed = false;
    ui.allocate_ui_with_layout(
        egui::vec2(total_width, row_height),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, row_height),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.set_min_height(row_height);
                    ui.label(label);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(controls_width, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_height(row_height);
                    if add_controls(ui) {
                        changed = true;
                    }
                },
            );
        },
    );
    changed
}

fn float_slider_range(label: &str, _value: f32) -> std::ops::RangeInclusive<f32> {
    match label {
        "threshold_deg" => 0.0..=180.0,
        _ => -1000.0..=1000.0,
    }
}

fn int_slider_range(label: &str, _value: i32) -> std::ops::RangeInclusive<i32> {
    match label {
        "domain" => 0..=3,
        "rows" | "cols" => 2..=64,
        _ => -1000..=1000,
    }
}
