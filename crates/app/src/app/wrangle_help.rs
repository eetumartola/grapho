use eframe::egui;

use crate::app::GraphoApp;

pub(super) struct WrangleHelpPanel {
    pub(super) screen_pos: egui::Pos2,
    pub(super) open: bool,
}

impl GraphoApp {
    pub(super) fn show_wrangle_help_panel(
        &mut self,
        ctx: &egui::Context,
        panel_slot: &mut Option<WrangleHelpPanel>,
    ) {
        let Some(mut panel) = panel_slot.take() else {
            return;
        };
        if !panel.open {
            return;
        }

        let mut open = panel.open;
        let window = egui::Window::new("Wrangle Help")
            .collapsible(true)
            .resizable(true)
            .default_pos(panel.screen_pos)
            .open(&mut open);

        window.show(ctx, |ui| {
            ui.heading("Wrangle Cheat Sheet");
            ui.separator();
            ui.label("Assignments (use @ for attributes):");
            ui.monospace("@Cd = vec3(1.0, 0.0, 0.0);");
            ui.monospace("@P = @P + vec3(0.0, 1.0, 0.0);");
            ui.separator();
            ui.label("Swizzles:");
            ui.monospace("@P.x");
            ui.monospace("@N.xy");
            ui.monospace("@Cd.rgb");
            ui.separator();
            ui.label("Functions:");
            ui.monospace("sin cos tan abs floor ceil pow min max clamp lerp len dot normalize");
            ui.separator();
            ui.label("Constructors:");
            ui.monospace("vec2(x, y) vec3(x, y, z) vec4(x, y, z, w)");
            ui.separator();
            ui.label("Operators:");
            ui.monospace("+ - * / ( )");
            ui.separator();
            ui.label("Notes:");
            ui.label("Mode selects vertex/point/prim/detail evaluation.");
            ui.label("@P writes only in Point mode; @N writes in Point/Vertex.");
            ui.label("Missing attributes read as 0.");
        });

        panel.open = open;
        if panel.open {
            *panel_slot = Some(panel);
        }
    }
}
