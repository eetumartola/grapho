use std::collections::HashMap;

use egui::{Color32, Pos2};
use egui_snarl::Snarl;

use grapho_core::{default_params, node_definition, BuiltinNodeKind, Graph, NodeId, PinId, PinType};

use super::state::SnarlNode;

pub(super) fn pin_color(pin_type: PinType) -> Color32 {
    match pin_type {
        PinType::Mesh => Color32::from_rgb(80, 160, 255),
        PinType::Float => Color32::from_rgb(220, 180, 90),
        PinType::Int => Color32::from_rgb(200, 120, 220),
        PinType::Bool => Color32::from_rgb(140, 220, 140),
        PinType::Vec2 => Color32::from_rgb(255, 160, 90),
        PinType::Vec3 => Color32::from_rgb(90, 210, 210),
    }
}

pub(super) fn add_builtin_node(
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

pub(super) fn find_input_of_type(
    graph: &Graph,
    node: &grapho_core::Node,
    pin_type: PinType,
) -> Option<(PinId, usize)> {
    node.inputs.iter().enumerate().find_map(|(idx, pin_id)| {
        let data = graph.pin(*pin_id)?;
        if data.pin_type == pin_type {
            Some((*pin_id, idx))
        } else {
            None
        }
    })
}

pub(super) fn find_output_of_type(
    graph: &Graph,
    node: &grapho_core::Node,
    pin_type: PinType,
) -> Option<(PinId, usize)> {
    node.outputs.iter().enumerate().find_map(|(idx, pin_id)| {
        let data = graph.pin(*pin_id)?;
        if data.pin_type == pin_type {
            Some((*pin_id, idx))
        } else {
            None
        }
    })
}

pub(super) fn point_segment_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ab_len = ab.length_sq();
    if ab_len <= f32::EPSILON {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / ab_len).clamp(0.0, 1.0);
    let proj = a + ab * t;
    point.distance(proj)
}

pub(super) fn point_bezier_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let dx = (b.x - a.x).abs() * 0.5 + 40.0;
    let c1 = Pos2::new(a.x + dx, a.y);
    let c2 = Pos2::new(b.x - dx, b.y);
    let segments = 12;
    let mut best = f32::MAX;
    let mut prev = a;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let p = cubic_bezier(a, c1, c2, b, t);
        let dist = point_segment_distance(point, prev, p);
        if dist < best {
            best = dist;
        }
        prev = p;
    }
    best
}

fn cubic_bezier(a: Pos2, b: Pos2, c: Pos2, d: Pos2, t: f32) -> Pos2 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;
    let mut p = a.to_vec2() * uuu;
    p += b.to_vec2() * (3.0 * uu * t);
    p += c.to_vec2() * (3.0 * u * tt);
    p += d.to_vec2() * ttt;
    Pos2::new(p.x, p.y)
}
