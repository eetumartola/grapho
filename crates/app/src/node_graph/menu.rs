use grapho_core::BuiltinNodeKind;

pub(super) struct MenuItem {
    pub(super) kind: BuiltinNodeKind,
    pub(super) name: &'static str,
    pub(super) category: &'static str,
}

pub(super) fn builtin_menu_items() -> Vec<MenuItem> {
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
            kind: BuiltinNodeKind::File,
            name: "File",
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
            kind: BuiltinNodeKind::CopyTransform,
            name: "Copy/Transform",
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
            kind: BuiltinNodeKind::Color,
            name: "Color",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Noise,
            name: "Noise/Mountain",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::AttributeMath,
            name: "Attribute Math",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::Wrangle,
            name: "Wrangle",
            category: "Operators",
        },
        MenuItem {
            kind: BuiltinNodeKind::ObjOutput,
            name: "OBJ Output",
            category: "Outputs",
        },
        MenuItem {
            kind: BuiltinNodeKind::Output,
            name: "Output",
            category: "Outputs",
        },
    ]
}
