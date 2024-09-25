use colored::Colorize;
use eframe::{
    egui::{
        self, CentralPanel, Layout,
        RichText, TextEdit, 
    },
    emath::Align,
    epaint::{Color32, Vec2},
};

use crate::{csv_handler, custom_widgets, proxy::{Proxy, ProxyEvent, ProxyExclusionList, ProxyExclusionRow, ProxyRequestLog}};

pub fn main_body(proxy: &mut Proxy, ui: &mut egui::Ui) {
    let panel_frame = egui::Frame {
        fill: ui.ctx().style().visuals.window_fill(),
        outer_margin: 5.0.into(),
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
    ui.allocate_ui_with_layout(
        Vec2 {
            x: 230.,
            y: ui.available_height(),
        },
        egui::Layout::top_down_justified(egui::Align::Min),
        |ui| {
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

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.add(egui::Label::new("Proxy Events: "));
                        ui.add(egui::Label::new(
                            RichText::new(format!("{}", proxy.get_requests().len()))
                                .color(Color32::LIGHT_GREEN),
                        ));
                    });

                    if proxy.get_blocking_status().0 {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            ui.add(egui::Label::new("Events Blocked: "));
                            ui.add(egui::Label::new(
                                RichText::new(format!("{}", proxy.get_requests().into_iter().map(|item| !item.2).len()))
                                    .color(Color32::LIGHT_GREEN),
                            ));
                        });
                    }

                    if proxy.get_blocking_status().0 {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            ui.add(egui::Label::new("Session Duration: "));
                            ui.add(egui::Label::new(
                                RichText::new(format!("{}s", proxy.get_run_time()))
                                    .color(Color32::LIGHT_GREEN),
                            ));
                        });
                    }
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
                                println!("{}", "Terminating Service...".yellow());
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
                                std::thread::spawn(move || proxy_clone.proxy_service());
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

                            #[cfg(target_os = "windows")]
                            if proxy.logs {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(650., 500.)));
                            }
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
        // TODO: Make the exclusion area take up more space if requests are closed
        ui.allocate_ui_with_layout(
            Vec2 {
                x: ui.available_width(),
                y: ui.available_height(),
            },
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let (is_blocking, allow_requests_by_default) = proxy.get_blocking_status();

                let mut blocking = is_blocking;
                let mut allow_requests_by_default = allow_requests_by_default;

                ui.horizontal(|ui| {

                    if ui
                    .checkbox(&mut blocking, "Enable Proxy Filtering")
                    .clicked()
                    {
                        proxy.enable_exclusion();
                    }
                        
                    
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.menu_button("Options", |ui| {

                            if ui.button("Import Exclusion List").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    println!("{}", path.display().to_string());

                                    if let Err(error) = csv_handler::read_from_csv::<String>(path.display().to_string()) {
                                        println!("{}", error);
                                    }
                                }
                            }

                            if ui.button("Export Exclusion List").clicked() {
                                let exclusion_list = proxy.get_current_list();
                                let mut exclusion_list_export = Vec::<ProxyExclusionList>::new();
                                for request in exclusion_list {
                                    exclusion_list_export.push(ProxyExclusionList { request });
                                }

                                if let Some(path) = rfd::FileDialog::new().save_file() {                                     
                                    if let Err(error) = csv_handler::write_csv_from_vec::<String>(path.display().to_string(), vec!["REQUEST".to_string()], proxy.get_current_list()) {
                                        println!("{} -> {}", "There was an error".red(), error);
                                    } else {
                                        println!("{} -> {}", "Exported Exclusions to file".blue(), path.display().to_string().green());
                                    }
                                }
                            }

                            if ui.button("Export Request List").clicked() {
                                if let Some(path) = rfd::FileDialog::new().save_file() {

                                    let request_list = proxy.get_requests();
                                    let mut request_list_export = Vec::<ProxyRequestLog>::new();
                                    for request in request_list {
                                        request_list_export.push(ProxyRequestLog { method: request.0, request: request.1, blocked: request.2 });
                                    }

                                    if let Err(error) = csv_handler::write_csv_from_vec::<ProxyRequestLog>(path.display().to_string(), vec!["METHOD".to_string(), "REQUEST".to_string(), "BLOCKED".to_string()], request_list_export) {
                                        println!("{}", error);
                                    } else {
                                        println!("{}: {}", "Exported Requests to file".blue(), path.display().to_string().green());
                                    }
                                };
                            }
                        });
                    });
                });

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
                        ui.group(|ui| {
                            let exclusion_list = proxy.get_current_list();
                            let num_rows = exclusion_list.len();

                            egui::ScrollArea::new([true, true])
                                .auto_shrink([false, false])
                                .max_height(ui.available_height() / 3.0)
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
                                                // Show save button if row is being edited, else edit and remove button
                                                if proxy.selected_exclusion_row.updating
                                                    && row == proxy.selected_exclusion_row.row_index
                                                {
                                                    ui.text_edit_singleline(
                                                        &mut proxy.selected_exclusion_row.row_value,
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
                                                                proxy.selected_exclusion_row = ProxyExclusionRow {
                                                                    updating: true,
                                                                    row_index: row,
                                                                    row_value: uri.to_string()
                                                                }
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
                    }); 
                }
                
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

                                                ui.with_layout(
                                                    Layout::left_to_right(eframe::emath::Align::Center),
                                                    |ui| {
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