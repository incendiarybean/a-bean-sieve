#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

mod default_window;
mod main_body;
mod proxy_handler;
mod task_bar;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        decorated: false,
        transparent: true,
        max_window_size: Some(egui::vec2(650.0, 500.0)),
        min_window_size: Some(egui::vec2(250.0, 140.0)),
        initial_window_size: Some(egui::vec2(250.0, 200.0)),
        resizable: true,
        follow_system_theme: true,
        ..Default::default()
    };
    eframe::run_native(
        "Proxy Blocker",
        options,
        Box::new(|_cc| Box::new(default_window::MainWindow::default())),
    )
}
