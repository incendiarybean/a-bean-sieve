use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use colored::Colorize;
use eframe::{
    egui::{self, CentralPanel},
    epaint::{Color32, Stroke, Vec2},
};

use crate::{main_body, task_bar};

#[derive(Debug)]
pub enum ProxyEvent {
    Running,
    Stopped,
    Error,
    Terminating,
    Terminated,
    RequestEvent((String, bool)),
}

pub struct Proxy {
    pub port: String,
    pub port_error: String,
    pub start_enabled: bool,
    pub event: Sender<ProxyEvent>,
    pub status: Arc<Mutex<ProxyEvent>>,
    pub logs: bool,

    pub requests: Arc<Mutex<Vec<String>>>,
}

impl Default for Proxy {
    fn default() -> Self {
        let (event_sender, event_receiver) = channel::<ProxyEvent>();
        let status = Arc::new(Mutex::new(ProxyEvent::Stopped));
        let status_clone = Arc::clone(&status);

        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let requests_clone = Arc::clone(&requests);

        thread::spawn(move || loop {
            match event_receiver.recv() {
                Ok(event) => match event {
                    ProxyEvent::Terminated | ProxyEvent::Stopped => {
                        let mut status = status_clone.lock().unwrap();
                        *status = ProxyEvent::Stopped;
                    }
                    ProxyEvent::RequestEvent((uri, blocked)) => {
                        println!(
                            "{} {} {}",
                            if blocked {
                                "ADVERT:".red()
                            } else {
                                "REQUEST:".green()
                            },
                            uri,
                            if blocked {
                                "-> BLOCKED".red()
                            } else {
                                "-> ALLOWED".green()
                            }
                        );

                        let mut status = requests_clone.lock().unwrap();
                        status.push(uri);
                    }
                    _ => {
                        let mut status = status_clone.lock().unwrap();
                        *status = event;
                    }
                },
                Err(_) => {
                    // This will likely run multiple times as it's closing down
                    // Don't log here
                }
            }
        });

        Self {
            port: String::from("8000"),
            port_error: String::default(),
            start_enabled: false,
            event: event_sender.clone(),
            status,
            logs: false,
            requests,
        }
    }
}

impl Proxy {
    pub fn get_status(&mut self) -> String {
        let proxy_state = match self.status.lock() {
            Ok(proxy_event) => proxy_event,
            Err(poisoned) => poisoned.into_inner(),
        };

        let current_proxy_status = match *proxy_state {
            ProxyEvent::Running => "RUNNING",
            ProxyEvent::Stopped => "STOPPED",
            ProxyEvent::Error => "ERROR",
            ProxyEvent::Terminating => "TERMINATING",
            ProxyEvent::Terminated => "TERMINATED",
            _ => "NOT COVERED",
        };

        current_proxy_status.to_string()
    }

    pub fn get_requests(&mut self) -> Vec<String> {
        let requests_list = match self.requests.lock() {
            Ok(requests_list) => requests_list,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests_list.to_vec()
    }
}

pub struct MainWindow {
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

impl eframe::App for MainWindow {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.proxy.logs && !frame.info().window_info.maximized {
            frame.set_window_size(Vec2 { x: 650.0, y: 500.0 });
        } else if !self.proxy.logs && !frame.info().window_info.maximized {
            frame.set_window_size(Vec2 { x: 250.0, y: 160.0 });
        }

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
                main_body::main_body(&mut self.proxy, ui);
            });
        });
    }
}
