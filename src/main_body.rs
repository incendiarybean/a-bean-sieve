use std::thread;

use eframe::{
    egui::{self, CentralPanel, Margin, RichText, TextEdit},
    epaint::{Color32, Vec2},
};

use crate::proxy::{Proxy, ProxyEvent};

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

// Toggle button from example Widgets
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

// Left hand side panel
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
                                let prox = proxy.clone();
                                thread::spawn(move || prox.proxy_service());
                            }
                        }

                        let logs_button_text = if proxy.logs {
                            "Close Logs"
                        } else {
                            "View Logs"
                        };
                        let logs_button_text = RichText::new(logs_button_text).size(13.0);
                        let logs_button = egui::Button::new(logs_button_text).min_size(Vec2 {
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
                            RichText::new(current_proxy_status.clone()).color(
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

// Right side panel
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
                    let (is_blocking, allow_requests_by_default) = proxy.get_blocking_status();

                    let mut blocking = is_blocking;
                    let mut allow_requests_by_default = allow_requests_by_default;

                    // Get the default/current list for displaying
                    let default_list = if allow_requests_by_default {
                        &proxy.allow_list
                    } else {
                        &proxy.block_list
                    };

                    if ui
                        .checkbox(&mut blocking, "Enable Proxy Filtering")
                        .clicked()
                    {
                        // Need to switch these around as allowing_all_traffic value doesn't change until event has been sent
                        let updated_list = if allow_requests_by_default {
                            &proxy.block_list
                        } else {
                            &proxy.allow_list
                        };

                        proxy
                            .event
                            .send(ProxyEvent::Blocking(blocking, updated_list.to_vec()))
                            .unwrap();
                    }

                    if is_blocking {
                        ui.horizontal(|ui| {
                            ui.label("Deny Incoming");
                            if toggle_ui(ui, &mut allow_requests_by_default).changed() {
                                proxy
                                    .event
                                    .send(ProxyEvent::UpdateList(default_list.to_vec()))
                                    .unwrap();
                            }
                            ui.label("Allow Incoming");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Exclusion List:");
                            if ui.button("options").clicked() {}
                        });

                        ui.add_space(4.);
                        ui.group(|ui| {
                            let exclusion_list = proxy.get_current_list();
                            let num_rows = exclusion_list.len();

                            egui::ScrollArea::new([true, true])
                                .auto_shrink([false, false])
                                .max_height(ui.available_height() / 3.0)
                                .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                    for row in row_range {
                                        let string_value = match exclusion_list.get(row) {
                                            Some(value) => value,
                                            _ => "No value found",
                                        };
                                        ui.label(string_value);
                                    }
                                });
                        });
                    }
                    ui.add_space(6.);
                });

                ui.label("Request Log:");
                ui.add_space(4.);
                ui.push_id("request_logger", |ui| {
                    ui.group(|ui| {
                        let request_list = proxy.get_requests();
                        let num_rows = request_list.len();
                        egui::ScrollArea::new([true, true])
                            .auto_shrink([false, false])
                            .max_height(ui.available_height())
                            .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                for row in row_range {
                                    match request_list.get(row) {
                                        Some((method, uri, blocked)) => ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(method).color(Color32::LIGHT_BLUE),
                                            );
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
                                            // let button_text =
                                            //     if *blocked { "Unblock" } else { "Block" };

                                            // if ui.button(button_text).clicked() {
                                            //     println!("REQUEST TO {}, {}", button_text, uri);
                                            // }
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
