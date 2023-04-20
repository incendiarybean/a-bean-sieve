use std::thread;

use colored::Colorize;
use eframe::{
    egui::{
        self, CentralPanel, Id,  Layout,
        Margin, RichText, TextEdit, 
    },
    emath::Align,
    epaint::{Color32, Vec2},
};

use crate::{proxy::{Proxy, ProxyEvent}, custom_widgets};

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

    // Main window, split, control_panel left and logs_panel right
    CentralPanel::default()
        .frame(panel_frame)
        .show(ui.ctx(), |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
                control_panel(proxy, ui);
                logs_panel(proxy, ui);
            });
        });
}

// Run this on every frame to check if the port is valid
fn check_startup_capability(port: &String) -> (bool, String) {
    if let Err(_) = port.trim().parse::<u16>() {
        return (false, String::from("Invalid Characters in Port."));
    }

    if port.len() > 5 || port.len() < 1 {
        return (false, String::from("Invalid Port Length."));
    }

    (true, String::default())
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
                        ui.add(input);

                        let (start_status, error_message) = check_startup_capability(&proxy.port);
                        proxy.start_enabled = start_status;
                        proxy.port_error = error_message;
                    }

                    if !proxy.port_error.is_empty() {
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(&proxy.port_error)
                                .size(11.0)
                                .color(Color32::LIGHT_RED),
                        );
                    }

                    // Block start-up until the service is completely terminated
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
                                let proxy_clone = proxy.clone();
                                thread::spawn(move || proxy_clone.proxy_service());
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

                    if ui
                        .checkbox(&mut blocking, "Enable Proxy Filtering")
                        .clicked()
                    {
                        proxy.enable_exclusion();
                    }

                    if is_blocking {
                        ui.horizontal(|ui| {
                            ui.label("Deny Incoming");
                            if custom_widgets::toggle_ui(ui, &mut allow_requests_by_default).changed() {
                                proxy.switch_exclusion();
                            }
                            ui.label("Allow Incoming");
                        });

                        egui::CollapsingHeader::new("Request Exclusion List").default_open(false).show_unindented(ui, |ui| {
                            ui.add_space(4.);
                            let drop_response = custom_widgets::drop_target(ui, |ui| {
                                ui.group(|ui| {
                                    let exclusion_list = proxy.get_current_list();
                                    let num_rows = exclusion_list.len();
    
                                    egui::ScrollArea::new([true, true])
                                        .auto_shrink([false, false])
                                        .max_height(ui.available_height() / 3.0)
                                        .max_width(ui.available_width())
                                        .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                            for row in row_range {
                                                if let Some(uri) = exclusion_list.get(row) {
                                                    // Truncate value so it fits better
                                                    let mut uri_truncated = uri.to_string();
                                                    if uri_truncated.len() > 35 {
                                                        uri_truncated.truncate(32);
                                                        uri_truncated += "...";
                                                    }
    
                                                    // TODO: Make this nicer!
                                                    ui.horizontal(|ui| {
                                                        if proxy.editing_row.0
                                                            && row == proxy.editing_row.1
                                                        {
                                                            ui.text_edit_singleline(
                                                                &mut proxy.editing_row.2,
                                                            );
    
                                                            ui.with_layout(
                                                                Layout::right_to_left(Align::Min),
                                                                |ui| {
                                                                    if ui.button("Save").clicked() {
                                                                        proxy.update_exclusion_list_value(
                                                                            uri.to_string(),
                                                                        );
                                                                    }
                                                                },
                                                            );
                                                        } else {
                                                            ui.label(
                                                                RichText::new(uri_truncated).size(12.5),
                                                            )
                                                            .on_hover_text_at_pointer(uri);
    
                                                            ui.with_layout(
                                                                Layout::right_to_left(Align::Min),
                                                                |ui| {
                                                                    if ui.button("Remove").clicked() {
                                                                        println!(
                                                                            "{} - {}",
                                                                            "Deleting item".green(),
                                                                            uri.red()
                                                                        );
                                                                        proxy.dragging_value =
                                                                            uri.to_string();
                                                                        proxy.add_exclusion();
                                                                    };
    
                                                                    if ui.button("Edit").clicked() {
                                                                        proxy.editing_row = (
                                                                            true,
                                                                            row,
                                                                            uri.to_string(),
                                                                        );
                                                                    }
                                                                },
                                                            );
                                                        }
                                                    });
                                                    ui.add(egui::Separator::default());
                                                }
                                            }
                                        });
                                });
                            })
                            .response;
    
                            // Check that an item is being dragged, it's over the drop zone and the mouse button is released
                            let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());
                            if is_being_dragged
                                && drop_response.hovered()
                                && proxy.editing_row.2.is_empty()
                            {
                                if ui.input(|i| i.pointer.any_released()) {
                                    println!("DOING SOMETHING BAD");
                                    proxy.add_exclusion();
                                }
                            }
                           
                        });                        
                    }
                    ui.add_space(6.);
                });

                egui::CollapsingHeader::new("Request Log").default_open(true).show_unindented(ui, |ui| {
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
                                                let mut uri_truncated = uri.clone();
                                                if uri_truncated.len() > 35 {
                                                    uri_truncated.truncate(35);
                                                    uri_truncated += "...";
                                                }

                                                let item_id = Id::new(format!(
                                                    "{}-{}-{}-{}",
                                                    method, uri, blocked, row
                                                ));
                                                ui.with_layout(
                                                    Layout::left_to_right(eframe::emath::Align::Center),
                                                    |ui| {
                                                        custom_widgets::drag_source(ui, item_id, |ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.with_layout(
                                                                    Layout::left_to_right(Align::Max),
                                                                    |ui| {
                                                                        ui.label(
                                                                            RichText::new(method)
                                                                                .color(
                                                                                    Color32::LIGHT_BLUE,
                                                                                )
                                                                                .size(12.5),
                                                                        );
                                                                        ui.label(uri_truncated)
                                                                            .on_hover_text_at_pointer(
                                                                                uri,
                                                                            );
                                                                    },
                                                                );
                                                            });
                                                        });
                                                    },
                                                );

                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        let button_text =
                                                            if *blocked { "Unblock" } else { "Block" };

                                                        if ui.button(button_text).clicked() {
                                                            proxy.dragging_value = uri.to_string();
                                                            proxy.add_exclusion();
                                                        }

                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{}",
                                                                if *blocked {
                                                                    "Blocked"
                                                                } else {
                                                                    "Allowed"
                                                                }
                                                            ))
                                                            .color(if *blocked {
                                                                Color32::LIGHT_RED
                                                            } else {
                                                                Color32::LIGHT_GREEN
                                                            }),
                                                        );
                                                    },
                                                );

                                                if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                                                    proxy.dragging_value = uri.to_string()
                                                }
                                            }),
                                            _ => ui.horizontal(|ui| {
                                                ui.label("No values Found");
                                            }),
                                        };

                                        ui.add(egui::Separator::default());
                                    }
                                });
                        });
                    });
                });
            },
        );
    }
}
