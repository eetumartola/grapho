use std::collections::HashMap;

use egui::{Color32, Pos2};
use egui_snarl::Snarl;

use grapho_core::{
    default_params, node_definition, BuiltinNodeKind, Graph, NodeId, PinId, PinType,
};

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

pub(super) fn point_snarl_wire_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let mut frame_size = 30.0;
    frame_size = adjust_frame_size(frame_size, false, true, a, b);
    let points = wire_bezier_5(frame_size, a, b);
    let mut best = f32::MAX;
    let steps = wire_sample_count(a, b);
    let mut prev = points[0];
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let p = sample_bezier_5(points, t);
        let dist = point_segment_distance(point, prev, p);
        if dist < best {
            best = dist;
        }
        prev = p;
    }
    best
}

fn wire_sample_count(a: Pos2, b: Pos2) -> i32 {
    let length = (b - a).length();
    let steps = (length / 20.0).ceil() as i32;
    steps.clamp(12, 64)
}

fn adjust_frame_size(
    mut frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
) -> f32 {
    let length = (from - to).length();
    if upscale {
        frame_size = frame_size.max(length / 6.0);
    }
    if downscale {
        frame_size = frame_size.min(length / 6.0);
    }
    frame_size
}

fn wire_bezier_5(frame_size: f32, from: Pos2, to: Pos2) -> [Pos2; 6] {
    let from_norm_x = frame_size;
    let from_2 = Pos2::new(from.x + from_norm_x, from.y);
    let to_norm_x = -from_norm_x;
    let to_2 = Pos2::new(to.x + to_norm_x, to.y);

    let between = (from_2 - to_2).length();

    if from_2.x <= to_2.x && between >= frame_size * 2.0 {
        let middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if from_2.x <= to_2.x {
        let t =
            (between - (to_2.y - from_2.y).abs()) / (frame_size * 2.0 - (to_2.y - from_2.y).abs());

        let mut middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let mut middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        if from_2.y >= to_2.y + frame_size {
            let u = (from_2.y - to_2.y - frame_size) / frame_size;

            let t0_middle_1 = Pos2::new(
                (1.0 - u) * frame_size + from_2.x,
                -u * frame_size + from_2.y,
            );
            let t0_middle_2 = Pos2::new(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if from_2.y >= to_2.y {
            let u = (from_2.y - to_2.y) / frame_size;

            let t0_middle_1 =
                Pos2::new(u * frame_size + from_2.x, (1.0 - u) * frame_size + from_2.y);
            let t0_middle_2 = Pos2::new(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y + frame_size {
            let u = (to_2.y - from_2.y - frame_size) / frame_size;

            let t0_middle_1 = Pos2::new(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = Pos2::new((1.0 - u) * -frame_size + to_2.x, -u * frame_size + to_2.y);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y {
            let u = (to_2.y - from_2.y) / frame_size;

            let t0_middle_1 = Pos2::new(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = Pos2::new(-frame_size * u + to_2.x, (1.0 - u) * frame_size + to_2.y);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        }

        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if from_2.y >= to_2.y + frame_size * 2.0 {
        let middle_1 = Pos2::new(from_2.x, from_2.y - frame_size);
        let middle_2 = Pos2::new(to_2.x, to_2.y + frame_size);
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if from_2.y >= to_2.y + frame_size {
        let t = (from_2.y - to_2.y - frame_size) / frame_size;

        let middle_1 = Pos2::new(
            (1.0 - t) * frame_size + from_2.x,
            -t * frame_size + from_2.y,
        );
        let middle_2 = Pos2::new(to_2.x, to_2.y + frame_size);
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if from_2.y >= to_2.y {
        let t = (from_2.y - to_2.y) / frame_size;

        let middle_1 = Pos2::new(t * frame_size + from_2.x, (1.0 - t) * frame_size + from_2.y);
        let middle_2 = Pos2::new(to_2.x, to_2.y + frame_size);
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if to_2.y >= from_2.y + frame_size * 2.0 {
        let middle_1 = Pos2::new(from_2.x, from_2.y + frame_size);
        let middle_2 = Pos2::new(to_2.x, to_2.y - frame_size);
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    if to_2.y >= from_2.y + frame_size {
        let t = (to_2.y - from_2.y - frame_size) / frame_size;

        let middle_1 = Pos2::new(from_2.x, from_2.y + frame_size);
        let middle_2 = Pos2::new((1.0 - t) * -frame_size + to_2.x, -t * frame_size + to_2.y);
        return [from, from_2, middle_1, middle_2, to_2, to];
    }

    let t = (to_2.y - from_2.y) / frame_size;

    let middle_1 = Pos2::new(from_2.x, from_2.y + frame_size);
    let middle_2 = Pos2::new(-frame_size * t + to_2.x, (1.0 - t) * frame_size + to_2.y);
    [from, from_2, middle_1, middle_2, to_2, to]
}

fn sample_bezier_5(points: [Pos2; 6], t: f32) -> Pos2 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let ttt = tt * t;
    let uuu = uu * u;
    let tttt = ttt * t;
    let uuuu = uuu * u;
    let ttttt = tttt * t;
    let uuuuu = uuuu * u;

    let mut p = points[0].to_vec2() * uuuuu;
    p += points[1].to_vec2() * (5.0 * uuuu * t);
    p += points[2].to_vec2() * (10.0 * uuu * tt);
    p += points[3].to_vec2() * (10.0 * uu * ttt);
    p += points[4].to_vec2() * (5.0 * u * tttt);
    p += points[5].to_vec2() * ttttt;
    Pos2::new(p.x, p.y)
}
