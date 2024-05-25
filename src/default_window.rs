use eframe::{
    egui::{self, CentralPanel},
    epaint::{Color32, Stroke, Vec2},
};

use crate::{main_body, proxy::Proxy, task_bar};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MainWindow {
    // Handle colour change of hovering over TaskBar buttons
    pub close_button_tint: Color32,
    pub minimise_button_tint: Color32,
    pub maximise_button_tint: Color32,

    // Handle all Proxy Details
    pub proxy: Proxy,
}

impl Default for MainWindow {
    fn default() -> Self {
        let proxy = Proxy::default();

        Self {
            close_button_tint: Color32::WHITE,
            minimise_button_tint: Color32::WHITE,
            maximise_button_tint: Color32::WHITE,

            proxy,
        }
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
                close_button_tint: previous_values.close_button_tint,
                minimise_button_tint: previous_values.minimise_button_tint,
                maximise_button_tint: previous_values.maximise_button_tint,
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
        let is_maximised = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let is_mac = std::env::consts::OS == "macos";

        // Start Window enlarged if the Log Window is open
        if self.proxy.logs && !is_maximised {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2 {
                x: 650.0,
                y: 500.0,
            }));
        } else if !self.proxy.logs && !is_maximised {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2 {
                x: 250.0,
                y: 160.0,
            }));
        }

        let width = if is_mac { 1. } else { 0. };
        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 7.0.into(),
            stroke: Stroke::new(width, Color32::LIGHT_GRAY),
            outer_margin: 0.1.into(),
            ..Default::default()
        };

        // Main layout of UI, task_bar top and main_body bottom
        CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                if std::env::consts::OS == "macos" {
                    task_bar::task_bar(self, ui);
                }
                main_body::main_body(&mut self.proxy, ui);
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
