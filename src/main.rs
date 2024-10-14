#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use std::sync::Arc;
pub mod service;
pub mod ui;
pub mod utils;

fn main() -> Result<(), eframe::Error> {
    let icon: &[u8] = include_bytes!("assets/icon.png");
    let img: image::DynamicImage = image::load_from_memory(icon).unwrap();

    let options = eframe::NativeOptions {
        follow_system_theme: true,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_min_inner_size(egui::vec2(250.0, 160.0))
            .with_resizable(true)
            .with_icon(Arc::new(egui::viewport::IconData {
                rgba: img.into_bytes(),
                width: 288,
                height: 288,
            })),
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "Proxy Blocker",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ui::default_window::MainWindow::new(cc)))
        }),
    )
}
