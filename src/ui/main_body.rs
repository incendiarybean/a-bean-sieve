use std::path::PathBuf;

use colored::Colorize;
use eframe::{
    egui::{self, vec2, CentralPanel, Layout, RichText, TextEdit},
    emath::Align,
    epaint::{Color32, Vec2},
};

use crate::service::{
    proxy::{
        Proxy, ProxyEvent, ProxyExclusionRow, ProxyExclusionUpdateKind, ProxyRequestLog, ProxyView,
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
                main_panel(proxy, ui);
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                        let expand_text = if proxy.view == ProxyView::Min {
                            "Detail View"
                        } else {
                            "Close Detail View"
                        };
                        let expand_btn = egui::Button::new(expand_text).min_size(Vec2 {
                            x: ui.available_width() / 2.,
                            y: 18.,
                        });

                        if ui.add(expand_btn).clicked() {
                            if proxy.view == ProxyView::Min {
                                proxy.view = ProxyView::Filter;
                            } else {
                                proxy.view = ProxyView::Min
                            }

                            #[cfg(target_os = "windows")]
                            match proxy.view {
                                ProxyView::Min => {}
                                ProxyView::Logs | ProxyView::Filter => {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        egui::vec2(650., 500.),
                                    ));
                                }
                            }
                        }

                        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                            let launch_btn_text = RichText::new(match current_proxy_status {
                                ProxyEvent::Running => "Stop Proxy",
                                ProxyEvent::Error(_) => "Retry Proxy",
                                ProxyEvent::Terminating => "Please Wait",
                                _ => "Start Proxy",
                            })
                            .size(13.0);

                            let launch_btn = egui::Button::new(launch_btn_text).min_size(Vec2 {
                                x: ui.available_width(),
                                y: 18.,
                            });

                            match current_proxy_status {
                                ProxyEvent::Running => {
                                    if ui.add(launch_btn).clicked() {
                                        proxy.stop()
                                    }
                                }
                                _ => {
                                    if ui.add_enabled(proxy.start_enabled, launch_btn).clicked() {
                                        proxy.run();
                                    }
                                }
                            }
                        });
                    });

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        ui.add(egui::Label::new("Process is currently:"));
                        ui.add(egui::Label::new(
                            RichText::new(current_proxy_status.to_string()).color(
                                match current_proxy_status {
                                    ProxyEvent::Running | ProxyEvent::Starting => {
                                        Color32::LIGHT_GREEN
                                    }
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
fn main_panel(proxy: &mut Proxy, ui: &mut egui::Ui) {
    if proxy.view != ProxyView::Min {
        ui.vertical(|ui| {
            let mut current_value = proxy.view.clone();
            egui::ComboBox::new("view_options", "Select the View")
                .truncate()
                .selected_text(format!("{}", current_value.to_string()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut current_value, ProxyView::Logs, "Log View");
                    ui.selectable_value(&mut current_value, ProxyView::Filter, "Filter View");
                });
            proxy.view = current_value;

            ui.add_space(5.);
            ui.separator();
            ui.add_space(5.);

            match proxy.view {
                ProxyView::Logs => logs_panel(proxy, ui),
                ProxyView::Filter => filter_panel(proxy, ui),
                _ => {}
            }
        });
    }
}

fn filter_panel(proxy: &mut Proxy, ui: &mut egui::Ui) {
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
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            match write_csv_from_vec::<String, PathBuf>(
                                path.clone(),
                                vec!["REQUEST"],
                                proxy.get_traffic_filter().get_filter_list(),
                            ) {
                                Ok(_) => println!(
                                    "{} -> {}",
                                    "Exported Exclusions to file".blue(),
                                    path.display().to_string().green()
                                ),
                                Err(error) => println!(
                                    "{} -> {}",
                                    "There was an error during the export".red(),
                                    error.to_string().red()
                                ),
                            };
                        }
                    }

                    if ui.button("Export Request List").clicked() {
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            match write_csv_from_vec::<ProxyRequestLog, PathBuf>(
                                path.clone(),
                                vec!["METHOD", "REQUEST", "BLOCKED"],
                                proxy.get_requests(),
                            ) {
                                Ok(_) => println!(
                                    "{} -> {}",
                                    "Exported Requests to file".blue(),
                                    path.display().to_string().green()
                                ),
                                Err(error) => println!(
                                    "{} -> {}",
                                    "There was an error during the export".red(),
                                    error.to_string().red()
                                ),
                            };
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
                proxy
                    .get_traffic_filter()
                    .get_opposing_filter_type()
                    .to_string()
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
                                        ui.horizontal(|ui| {
                                            if proxy.selected_exclusion_row.updating
                                                && row == proxy.selected_exclusion_row.index
                                            {
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Min),
                                                    |ui| {
                                                        if ui.button("Save").clicked() {
                                                            proxy.update_exclusion_list(
                                                                ProxyExclusionUpdateKind::Edit,
                                                            );
                                                        }

                                                        let single_line_edit =
                                                            egui::TextEdit::singleline(
                                                                &mut proxy
                                                                    .selected_exclusion_row
                                                                    .value,
                                                            )
                                                            .min_size(vec2(
                                                                ui.available_width(),
                                                                18.,
                                                            ));

                                                        ui.add(single_line_edit);
                                                    },
                                                );
                                            } else {
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Min),
                                                    |ui| {
                                                        if ui.button("Remove").clicked() {
                                                            println!(
                                                                "{} - {}",
                                                                "Deleting item".green(),
                                                                uri.red()
                                                            );
                                                            proxy.selected_value = uri.to_string();
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

                                                        ui.with_layout(
                                                            Layout::left_to_right(Align::Min),
                                                            |ui| {
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(uri)
                                                                            .size(12.5),
                                                                    )
                                                                    .truncate(),
                                                                )
                                                                .on_hover_text_at_pointer(uri);
                                                            },
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                        ui.separator();
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
                                                                        .color(Color32::LIGHT_BLUE)
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
                                                        proxy.selected_value = request.to_string();
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

fn logs_panel(_proxy: &mut Proxy, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Log Filters:");
            let _ = ui.button("All");
            let _ = ui.button("Info");
            let _ = ui.button("Error");
            let _ = ui.button("Warning");
        });
        ui.add_space(2.);
        ui.group(|ui| {
            ui.allocate_space(ui.available_size());
        });
    });
}
