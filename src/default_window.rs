use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use eframe::{
    egui::{self, CentralPanel},
    epaint::{Color32, Stroke},
};

use crate::{main_body, task_bar};

pub struct MainWindow {
    pub close_button_tint: Color32,
    pub minimise_button_tint: Color32,
    pub maximise_button_tint: Color32,

    pub port: String,
    pub port_error: String,
    pub start_server_capable: bool,
    pub proxy_event_sender: Sender<ProxyEvent>,
    pub proxy_status: Arc<Mutex<ProxyEvent>>,
}

#[derive(Debug)]
pub enum ProxyEvent {
    Running,
    Stopped,
    Error,
    Terminating,
    Terminated,
}

impl Default for MainWindow {
    fn default() -> Self {
        let (proxy_event_sender, proxy_event_receiver) = channel::<ProxyEvent>();
        let proxy_event = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let proxy_event_clone = Arc::clone(&proxy_event);

        thread::spawn(move || loop {
            match proxy_event_receiver.recv() {
                Ok(event) => match event {
                    ProxyEvent::Running => {
                        let mut status = proxy_event_clone.lock().unwrap();
                        *status = ProxyEvent::Running;
                    }
                    ProxyEvent::Error => {
                        let mut status = proxy_event_clone.lock().unwrap();
                        *status = ProxyEvent::Error;
                    }
                    ProxyEvent::Terminating => {
                        let mut status = proxy_event_clone.lock().unwrap();
                        *status = ProxyEvent::Terminating;
                    }
                    ProxyEvent::Terminated | ProxyEvent::Stopped => {
                        let mut status = proxy_event_clone.lock().unwrap();
                        *status = ProxyEvent::Stopped;
                    }
                },
                Err(_) => {
                    // This will likely run multiple times as it's closing down
                    // Don't log here
                }
            }
        });

        Self {
            close_button_tint: Color32::WHITE,
            minimise_button_tint: Color32::WHITE,
            maximise_button_tint: Color32::WHITE,

            port: String::default(),
            port_error: String::default(),
            start_server_capable: false,
            proxy_event_sender: proxy_event_sender.clone(),
            proxy_status: proxy_event,
        }
    }
}

impl eframe::App for MainWindow {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 7.0.into(),
            stroke: Stroke::new(1.0, Color32::LIGHT_GRAY),
            outer_margin: 0.1.into(),
            ..Default::default()
        };

        CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                task_bar::task_bar(self, ui, frame);
                main_body::main_body(
                    self,
                    ui,
                    self.proxy_event_sender.clone(),
                    // self.request_event_sender.clone(),
                );
            });
        });
    }
}
