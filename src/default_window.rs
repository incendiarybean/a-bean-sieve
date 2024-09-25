use eframe::{
    egui::{self, CentralPanel, Rounding},
    epaint::{Color32, Stroke},
};

use crate::{main_body, proxy::Proxy};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MainWindow {
    pub proxy: Proxy,
}

impl Default for MainWindow {
    fn default() -> Self {
        let proxy = Proxy::default();

        Self { proxy }
    }
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            // Handle our own state here:
            // The basic state is ok being managed by the app
            // The Proxy state needs adjusting as it contains Mutex state which doesn't reimplement well
            let previous_values: MainWindow =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            let allow_blocking = match previous_values.proxy.allow_blocking.lock() {
                Ok(allow_blocking) => *allow_blocking,
                Err(poisoned) => *poisoned.into_inner(),
            };

            let allow_requests_by_default =
                match previous_values.proxy.allow_requests_by_default.lock() {
                    Ok(allow_requests_by_default) => *allow_requests_by_default,
                    Err(poisoned) => *poisoned.into_inner(),
                };

            // Create new proxy to generate mutables
            return Self {
                proxy: Proxy::default().restore_previous(
                    previous_values.proxy.port,
                    previous_values.proxy.port_error,
                    previous_values.proxy.logs,
                    previous_values.proxy.allow_list,
                    previous_values.proxy.block_list,
                    allow_blocking,
                    allow_requests_by_default,
                ),
            };
        }

        Default::default()
    }
}

impl eframe::App for MainWindow {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.proxy.logs {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(650., 500.)));
        } else if !self.proxy.logs {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(250., 160.)));
        }

        #[cfg(target_os = "macos")]
        let rounding = Rounding {
            nw: 0.,
            ne: 0.,
            sw: 10.,
            se: 10.,
        };

        #[cfg(not(target_os = "macos"))]
        let rounding = Rounding {
            nw: 0.,
            ne: 0.,
            sw: 5.,
            se: 5.,
        };

        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding,
            stroke: Stroke::new(0., Color32::LIGHT_GRAY),
            outer_margin: 0.1.into(),
            ..Default::default()
        };

        // Main layout of UI, task_bar top and main_body bottom
        CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            main_body::main_body(&mut self.proxy, ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
