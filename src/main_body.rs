use std::{net::SocketAddr, thread};

use eframe::{
    egui::{self, CentralPanel, Margin, RichText, TextEdit},
    epaint::{Color32, Vec2},
};

use crate::{
    default_window::{Proxy, ProxyEvent},
    proxy_handler::{proxy_service, read_from_csv},
};

pub fn main_body(proxy: &mut Proxy, ui: &mut egui::Ui) {
    let panel_frame = egui::Frame {
        fill: ui.ctx().style().visuals.window_fill(),
        outer_margin: Margin {
            left: 5.0.into(),
            right: 5.0.into(),
            top: 27.0.into(),
            bottom: 5.0.into(),
        },
        inner_margin: 5.0.into(),
        ..Default::default()
    };

    CentralPanel::default()
        .frame(panel_frame)
        .show(ui.ctx(), |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
                control_panel(proxy, ui);
                logs_panel(proxy, ui);
            });
        });
}

fn control_panel(proxy: &mut Proxy, ui: &mut egui::Ui) {
    // Get the current status of the Proxy to display functional components
    let current_proxy_status = proxy.get_status();

    // Create UI in downward direction
    // use height of base app as we don't want to full up the entire space horizontally
    // Use current height as we want to fill up the entire space vertically
    ui.allocate_ui_with_layout(
        Vec2 {
            x: 230.,
            y: ui.available_height(),
        },
        egui::Layout::top_down_justified(egui::Align::Min),
        |ui| {
            // TODO: Do I want this in a group? Does it look dumb?
            ui.group(|ui| {
                // Label and Port input
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    if current_proxy_status == "STOPPED" {
                        ui.label(RichText::new("Enter a Port to run on:").size(13.0));
                        ui.add_space(2.0);

                        let input = TextEdit::singleline(&mut proxy.port)
                            .hint_text("Port, e.g. 8000")
                            .vertical_align(eframe::emath::Align::Center)
                            .min_size(Vec2 {
                                x: ui.available_width(),
                                y: 20.0,
                            });

                        if ui.add(input).changed() {
                            // TODO: Please handle error handling better
                            // This should be checked constantly, when it's first painted & when it's changed

                            if let Err(_) = proxy.port.trim().parse::<u16>() {
                                proxy.port_error = String::from("Invalid Characters in Port.");
                                return proxy.start_enabled = false;
                            }

                            if proxy.port.len() > 5 || proxy.port.len() < 1 {
                                proxy.port_error = String::from("Invalid Port Length.");
                                return proxy.start_enabled = false;
                            }

                            proxy.start_enabled = true;
                            proxy.port_error = String::default();
                        }
                    }

                    if !proxy.port_error.is_empty() {
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(&proxy.port_error)
                                .size(11.0)
                                .color(Color32::LIGHT_RED),
                        );
                    }

                    if current_proxy_status == "TERMINATING" {
                        proxy.start_enabled = false;
                    }
                });

                // Display Address Proxy is running on
                if current_proxy_status == "RUNNING" {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.add(egui::Label::new("Hosting on: "));
                        ui.add(egui::Label::new(
                            RichText::new(format!("127.0.0.1:{}", proxy.port))
                                .color(Color32::LIGHT_GREEN),
                        ));
                    });
                }

                // Proxy Control buttons
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        if current_proxy_status == "RUNNING" {
                            let stop_button = egui::Button::new("Stop Proxy").min_size(Vec2 {
                                x: ui.available_width() / 2.,
                                y: 18.,
                            });

                            if ui
                                .add_enabled(current_proxy_status == "RUNNING", stop_button)
                                .clicked()
                            {
                                proxy.event.send(ProxyEvent::Terminating).unwrap();
                            }
                        } else {
                            let start_button_text =
                                RichText::new(match current_proxy_status.as_str() {
                                    "ERROR" => "Retry Proxy",
                                    "TERMINATING" => "Please Wait",
                                    _ => "Start Proxy",
                                })
                                .size(13.0);

                            let start_button =
                                egui::Button::new(start_button_text).min_size(Vec2 {
                                    x: ui.available_width() / 2.,
                                    y: 18.,
                                });

                            if ui.add_enabled(proxy.start_enabled, start_button).clicked() {
                                let port_copy = proxy.port.trim().parse::<u16>().unwrap().clone();
                                let proxy_status = proxy.status.clone();

                                // Create a thread and assign the server to it
                                // This stops the UI from freezing
                                let event_sender_clone = proxy.event.clone();
                                thread::spawn(move || {
                                    proxy_service(
                                        SocketAddr::from(([127, 0, 0, 1], port_copy)),
                                        event_sender_clone,
                                        proxy_status,
                                    )
                                });
                            }
                        }

                        let logs_button = egui::Button::new(RichText::new("View Logs").size(13.0))
                            .min_size(Vec2 {
                                x: ui.available_width(),
                                y: 18.,
                            });

                        if ui.add_enabled(true, logs_button).clicked() {
                            proxy.logs = !proxy.logs;
                        }
                    });

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        ui.add(egui::Label::new("Process is currently:"));
                        ui.add(egui::Label::new(
                            RichText::new(format!("{}", current_proxy_status)).color(
                                match current_proxy_status.as_str() {
                                    "RUNNING" => Color32::LIGHT_GREEN,
                                    _ => Color32::LIGHT_RED,
                                },
                            ),
                        ));
                    });
                });
            });
        },
    );
}

fn logs_panel(proxy: &mut Proxy, ui: &mut egui::Ui) {
    if proxy.logs {
        ui.allocate_ui_with_layout(
            Vec2 {
                x: ui.available_width(),
                y: ui.available_height(),
            },
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.vertical(|ui| {
                    ui.label("Allow List:");
                    ui.add_space(4.);
                    ui.group(|ui| {
                        // TODO: Create struct for CSV reading
                        // Struct { values }
                        // Impl { new -> get values to self, save -> rewrite new values to file, clear -> remove values from file }
                        let whitelist = read_from_csv::<String>("./src/whitelist.csv").unwrap();
                        let num_rows = whitelist.clone().into_iter().count();

                        let mut checked = false;
                        egui::ScrollArea::new([false, true])
                            .auto_shrink([false, false])
                            .max_height(ui.available_height() / 3.0)
                            .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                for row in row_range {
                                    let string_value = match whitelist.get(row) {
                                        Some(value) => value,
                                        _ => "No value found",
                                    };
                                    ui.checkbox(&mut checked, format!("{}", string_value));
                                }
                            });
                    });
                });

                ui.add_space(6.);
                ui.label("Request Log:");
                ui.add_space(4.);
                ui.push_id("poop", |ui| {
                    ui.group(|ui| {
                        let request_list = proxy.get_requests();
                        let num_rows = request_list.len();
                        // let mut _checked = false;
                        egui::ScrollArea::new([false, true])
                            .auto_shrink([false, false])
                            .max_height(ui.available_height())
                            .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                // TODO: Loop through Vec<RequestList>

                                for row in row_range {
                                    let string_value = match request_list.get(row) {
                                        Some(value) => value,
                                        _ => "No value found",
                                    };

                                    ui.label(string_value);
                                }
                            });
                    });
                });
            },
        );
    }
}
