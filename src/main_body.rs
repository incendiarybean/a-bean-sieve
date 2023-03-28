use std::{net::SocketAddr, thread};

use eframe::{
    egui::{self, CentralPanel, Margin, RichText, TextEdit},
    epaint::{Color32, Vec2},
};

use crate::proxy::{proxy_service, Proxy, ProxyEvent};

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
                                let blocking = proxy.allow_blocking.clone();
                                thread::spawn(move || {
                                    proxy_service(
                                        SocketAddr::from(([127, 0, 0, 1], port_copy)),
                                        event_sender_clone,
                                        proxy_status,
                                        blocking,
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

fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
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
                    let is_blocking = match proxy.allow_blocking.lock() {
                        Ok(is_blocking) => is_blocking,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    let mut blocking = *is_blocking;

                    if ui
                        .checkbox(&mut blocking, "Enable Proxy Filtering")
                        .clicked()
                    {
                        println!("button {}, {}", !*is_blocking, blocking);

                        proxy.event.send(ProxyEvent::Blocking(blocking)).unwrap();
                    }

                    if *is_blocking {
                        ui.horizontal(|ui| {
                            ui.label("Allow Incoming");
                            if toggle_ui(ui, &mut proxy.blocking_by_allow).changed() {}
                            ui.label("Block Incoming");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Exclusion List:");
                            if ui.button("options").clicked() {}
                        });

                        ui.add_space(4.);
                        ui.group(|ui| {
                            let list = if proxy.blocking_by_allow {
                                proxy.allow_list.clone()
                            } else {
                                proxy.block_list.clone()
                            };

                            let num_rows = list.len();

                            let mut checked = false;
                            egui::ScrollArea::new([false, true])
                                .auto_shrink([false, false])
                                .max_height(ui.available_height() / 3.0)
                                .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                    for row in row_range {
                                        let string_value = match list.get(row) {
                                            Some(value) => value,
                                            _ => "No value found",
                                        };
                                        ui.checkbox(&mut checked, format!("{}", string_value));
                                    }
                                });
                        });
                    }
                });

                ui.add_space(6.);
                ui.label("Request Log:");
                ui.add_space(4.);
                ui.push_id("poop", |ui| {
                    ui.group(|ui| {
                        let request_list = proxy.get_requests();
                        let num_rows = request_list.len();
                        egui::ScrollArea::new([false, true])
                            .auto_shrink([false, false])
                            .max_height(ui.available_height())
                            .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                for row in row_range {
                                    match request_list.get(row) {
                                        Some((uri, blocked)) => ui.horizontal(|ui| {
                                            ui.label(uri);
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}",
                                                    if *blocked { "Blocked" } else { "Allowed" }
                                                ))
                                                .color(if *blocked {
                                                    Color32::LIGHT_RED
                                                } else {
                                                    Color32::LIGHT_GREEN
                                                }),
                                            );
                                        }),
                                        _ => ui.horizontal(|ui| {
                                            ui.label("No values Found");
                                        }),
                                    };
                                }
                            });
                    });
                });
            },
        );
    }
}
