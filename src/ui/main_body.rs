use std::path::PathBuf;

use colored::Colorize;
use eframe::{
    egui::{self, CentralPanel, Layout, RichText, TextEdit},
    emath::Align,
    epaint::{Color32, Vec2},
};

use crate::service::{
    proxy::{
        Proxy, ProxyEvent, ProxyExclusionList, ProxyExclusionRow, ProxyExclusionUpdateKind,
        ProxyRequestLog,
    },
    traffic_filter::TrafficFilterType,
};
use crate::utils::csv_handler::{read_from_csv, write_csv_from_vec};

use super::custom_widgets::toggle_ui;

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

struct StartAvailable {
    allowed: bool,
    error: Option<String>,
}

// Run this on every frame to check if the port is valid
fn check_startup_capability(port: &String) -> StartAvailable {
    let mut error: Option<String> = None;

    if port.len() > 5 || port.len() < 1 {
        error = Some(String::from("Invalid Port Length."))
    } else if let Err(_) = port.trim().parse::<u16>() {
        error = Some(String::from("Invalid Characters in Port."))
    } else if port == "0" {
        error = Some(String::from("Port cannot be 0."))
    } else if *port != port.trim().parse::<u16>().unwrap().to_string() {
        error = Some(String::from("Port cannot begin with a 0."))
    }

    StartAvailable {
        allowed: error.is_none(),
        error,
    }
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
                match current_proxy_status {
                    ProxyEvent::Running => {
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

                        if proxy.get_traffic_filter().get_enabled() {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new("Events Blocked: "));
                                ui.add(egui::Label::new(
                                    RichText::new(format!(
                                        "{}",
                                        proxy
                                            .get_requests()
                                            .iter()
                                            .filter(|proxy_request_item| proxy_request_item.blocked
                                                != false)
                                            .collect::<Vec<_>>()
                                            .len()
                                    ))
                                    .color(Color32::LIGHT_GREEN),
                                ));
                            });

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new("Session Duration: "));
                                ui.add(egui::Label::new(
                                    RichText::new(format!("{}s", proxy.get_run_time()))
                                        .color(Color32::LIGHT_GREEN),
                                ));
                            });
                        }
                    }
                    ProxyEvent::Stopped | ProxyEvent::Error(_) => {
                        ui.label(RichText::new("Enter a Port to run on:").size(13.0));
                        ui.add_space(2.0);

                        ui.add(
                            TextEdit::singleline(&mut proxy.port)
                                .hint_text("Port, e.g. 8000")
                                .vertical_align(eframe::emath::Align::Center)
                                .min_size(Vec2 {
                                    x: ui.available_width(),
                                    y: 20.0,
                                }),
                        );

                        let startup = check_startup_capability(&proxy.port);
                        proxy.start_enabled = startup.allowed;
                        proxy.port_error = startup.error.unwrap_or(String::default());
                    }
                    ProxyEvent::Terminating => {
                        proxy.start_enabled = false;
                    }
                    ProxyEvent::Terminated => {
                        proxy.start_enabled = true;
                    }
                    _ => {}
                }

                if !proxy.port_error.is_empty() {
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(&proxy.port_error)
                            .size(11.0)
                            .color(Color32::LIGHT_RED),
                    );
                }

                // Proxy Control buttons
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        if current_proxy_status == ProxyEvent::Running {
                            let stop_button = egui::Button::new("Stop Proxy").min_size(Vec2 {
                                x: ui.available_width() / 2.,
                                y: 18.,
                            });

                            if ui
                                .add_enabled(
                                    current_proxy_status == ProxyEvent::Running,
                                    stop_button,
                                )
                                .clicked()
                            {
                                println!("{}", "Terminating Service...".yellow());
                                proxy.event.send(ProxyEvent::Terminating).unwrap();
                            }
                        } else {
                            let start_button_text = RichText::new(match current_proxy_status {
                                ProxyEvent::Error(_) => "Retry Proxy",
                                ProxyEvent::Terminating => "Please Wait",
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
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                    egui::vec2(650., 500.),
                                ));
                            }
                        }
                    });

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        ui.add(egui::Label::new("Process is currently:"));
                        ui.add(egui::Label::new(
                            RichText::new(current_proxy_status.to_string()).color(
                                match current_proxy_status {
                                    ProxyEvent::Running => Color32::LIGHT_GREEN,
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
        ui.vertical(|ui| {
            let mut is_blocking = proxy.get_traffic_filter().get_enabled();
            let mut allow_requests_by_default = match proxy.get_traffic_filter().get_filter_type() {
                TrafficFilterType::Allow => true,
                TrafficFilterType::Deny => false,
            };

            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut is_blocking, "Enable Proxy Filtering")
                    .clicked()
                {
                    proxy.toggle_traffic_filtering();
                }

                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.menu_button("Options", |ui| {
                        if ui.button("Import Exclusion List").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                match read_from_csv::<String, PathBuf>(path) {
                                    Ok(list) => {
                                        proxy.set_exclusion_list(list);
                                    }
                                    Err(error) => println!("{}", error),
                                }
                            }
                        }

                        if ui.button("Export Exclusion List").clicked() {
                            let exclusion_list = proxy.get_traffic_filter().get_filter_list();
                            let mut exclusion_list_export = Vec::<ProxyExclusionList>::new();
                            for request in exclusion_list {
                                exclusion_list_export.push(ProxyExclusionList { request });
                            }

                            if let Some(path) = rfd::FileDialog::new().save_file() {
                                if let Err(error) = write_csv_from_vec::<String, PathBuf>(
                                    path.clone(),
                                    vec!["REQUEST"],
                                    proxy.get_traffic_filter().get_filter_list(),
                                ) {
                                    println!("{} -> {}", "There was an error".red(), error);
                                } else {
                                    println!(
                                        "{} -> {}",
                                        "Exported Exclusions to file".blue(),
                                        path.display().to_string().green()
                                    );
                                }
                            }
                        }

                        if ui.button("Export Request List").clicked() {
                            if let Some(path) = rfd::FileDialog::new().save_file() {
                                let request_list = proxy.get_requests();

                                let mut request_list_export = Vec::<ProxyRequestLog>::new();

                                for request in request_list {
                                    request_list_export.push(request);
                                }

                                if let Err(error) = write_csv_from_vec::<ProxyRequestLog, PathBuf>(
                                    path.clone(),
                                    vec!["METHOD", "REQUEST", "BLOCKED"],
                                    request_list_export,
                                ) {
                                    println!("{}", error);
                                } else {
                                    println!(
                                        "{}: {}",
                                        "Exported Requests to file".blue(),
                                        path.display().to_string().green()
                                    );
                                }
                            };
                        }
                    });
                });
            });

            let request_logs_id = egui::Id::new("Request_Logs");
            let request_logs_open =
                ui.memory_mut(|m| m.data.get_temp::<bool>(request_logs_id).unwrap_or_default());

            if is_blocking {
                ui.horizontal(|ui| {
                    ui.label("Deny Incoming");
                    if toggle_ui(ui, &mut allow_requests_by_default).changed() {
                        proxy.switch_exclusion_list();
                    }
                    ui.label("Allow Incoming");
                });

                egui::CollapsingHeader::new(format!(
                    "{} List",
                    proxy.get_traffic_filter().get_opposing_filter_type().to_string()
                ))
                .default_open(false)
                .show_unindented(ui, |ui| {
                    ui.group(|ui| {
                        ui.push_id("request_exclusion_list_scrollarea", |ui| {
                            let exclusion_list = proxy.get_traffic_filter().get_filter_list();
                            let num_rows = exclusion_list.len();

                            egui::ScrollArea::new([true, true])
                                .auto_shrink([false, false])
                                .max_height(if request_logs_open {
                                    ui.available_height() / 3.
                                } else {
                                    ui.available_height() - 20.
                                })
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
                                                    && row == proxy.selected_exclusion_row.index
                                                {
                                                    ui.text_edit_singleline(
                                                        &mut proxy.selected_exclusion_row.value,
                                                    );

                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Min),
                                                        |ui| {
                                                            if ui.button("Save").clicked() {
                                                                proxy.update_exclusion_list(
                                                                    ProxyExclusionUpdateKind::Edit,
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
                                                                proxy.selected_value =
                                                                    uri.to_string();
                                                                proxy.update_exclusion_list(
                                                                    ProxyExclusionUpdateKind::Remove,
                                                                );
                                                            };

                                                            if ui.button("Edit").clicked() {
                                                                proxy.selected_exclusion_row =
                                                                    ProxyExclusionRow {
                                                                        updating: true,
                                                                        index: row,
                                                                        value: uri.to_string(),
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
                });
            }

            let request_logs_dropdown = egui::CollapsingHeader::new("Request Logs")
                .default_open(false)
                .show_unindented(ui, |ui| {
                    ui.group(|ui| {
                        ui.push_id("request_logs_scrollarea", |ui| {
                            let request_list = proxy.get_requests();
                            let num_rows = request_list.len();

                            egui::ScrollArea::new([true, true])
                                .auto_shrink([false, false])
                                .max_height(ui.available_height())
                                .show_rows(ui, 18.0, num_rows, |ui, row_range| {
                                    for row in row_range {
                                        match request_list.get(row) {
                                            Some(proxy_request_log) => ui.horizontal(|ui| {
                                                let method = proxy_request_log.method.clone();
                                                let request = proxy_request_log.request.clone();
                                                let blocked = proxy_request_log.blocked;

                                                let mut uri_truncated = request.clone();
                                                if uri_truncated.len() > 35 {
                                                    uri_truncated.truncate(35);
                                                    uri_truncated += "...";
                                                }

                                                ui.with_layout(
                                                    Layout::left_to_right(Align::Center),
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
                                                                            .size(13.),
                                                                    );
                                                                    ui.label(uri_truncated)
                                                                        .on_hover_text_at_pointer(
                                                                            &request,
                                                                        );
                                                                },
                                                            );
                                                        });
                                                    },
                                                );

                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        let exclusion_values = if blocked {
                                                            (
                                                                "Unblock",
                                                                "Blocked",
                                                                Color32::LIGHT_RED,
                                                                ProxyExclusionUpdateKind::Remove,
                                                            )
                                                        } else {
                                                            (
                                                                "Block",
                                                                "Allowed",
                                                                Color32::LIGHT_GREEN,
                                                                ProxyExclusionUpdateKind::Add,
                                                            )
                                                        };

                                                        if ui.button(exclusion_values.0).clicked() {
                                                            proxy.selected_value =
                                                                request.to_string();
                                                            proxy.update_exclusion_list(
                                                                exclusion_values.3,
                                                            );
                                                        }

                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{}",
                                                                exclusion_values.1
                                                            ))
                                                            .color(exclusion_values.2),
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

            ui.memory_mut(|m| {
                let open = m.data.get_temp_mut_or_default::<bool>(request_logs_id);
                *open = request_logs_dropdown.fully_open();
            });
        });
    }
}
