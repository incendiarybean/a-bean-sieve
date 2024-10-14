#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use colored::Colorize;
use eframe::egui;
use service::{proxy::Proxy, traffic_filter::TrafficFilter};
use std::{env::Args, process::exit, sync::Arc, thread::sleep, time::Duration};
use utils::logger::LogLevel;

mod service;
mod ui;
mod utils;

#[derive(PartialEq, Debug)]
enum CliFlag {
    CommandLine,
    Port,
    LogLevel,
    Help,
    Value,
}

impl ToString for CliFlag {
    fn to_string(&self) -> String {
        match self {
            CliFlag::CommandLine => String::from("--no-ui"),
            CliFlag::Port => String::from("--port"),
            CliFlag::LogLevel => String::from("--log-level"),
            CliFlag::Help => String::from("--help"),
            _ => String::from("invalid-flag"),
        }
    }
}

impl From<&String> for CliFlag {
    fn from(value: &String) -> Self {
        if &String::from("--no-ui") == value {
            return CliFlag::CommandLine;
        }
        if &String::from("--port") == value {
            return CliFlag::Port;
        }

        if &String::from("--log-level") == value {
            return CliFlag::LogLevel;
        }

        if &String::from("--help") == value {
            return CliFlag::Help;
        }

        return CliFlag::Value;
    }
}

impl CliFlag {
    fn requires_value(&self) -> bool {
        match self {
            CliFlag::CommandLine => false,
            CliFlag::Port => true,
            CliFlag::LogLevel => true,
            _ => false,
        }
    }
}

#[derive(Default, Debug)]
struct CliAdapter {
    args: Vec<String>,
    command_line: bool,
    port: String,
    log_level: LogLevel,
}

impl CliAdapter {
    pub fn new(args: Args) -> Self {
        // Collect args, and remove path as first arg
        let mut args = args.collect::<Vec<String>>();
        args.remove(0);

        Self {
            args,
            ..Default::default()
        }
    }

    /// Print a usage message in the terminal.
    fn usage(&self) {
        println!("");
        println!("{}", "Available Flags:".blue());
        println!("  --no-ui : Use the tool in CLI mode.");
        println!("  --port : Choose the port to run the proxy on.");
        println!(
            "  --log-level : The logging level, one of ['debug', 'info', 'warning', 'error']."
        );
        println!("  --help : Print usage and flags.");
        println!("");
        println!("{}", "Example Usage:".blue());
        println!(
            "  {}",
            "a-bean-sieve.exe --no-gui --port 8080 --log-level INFO".yellow()
        );
        println!("");
    }

    /// Map arguments passed to the application to CliAdapter values.
    fn map_arg_to_flag(&mut self) -> Result<(), &'static str> {
        let mut skip_parameter = false;

        for (index, argument) in self.args.clone().iter().enumerate() {
            match skip_parameter {
                true => skip_parameter = false,
                false => {
                    let current_flag = CliFlag::from(argument);

                    if current_flag == CliFlag::Value {
                        return Err("Value has been provided without the appropriate flag...");
                    }

                    let current_flag_value = self.args.get(index + 1);
                    if current_flag.requires_value() {
                        if current_flag_value.is_none() {
                            return Err("Flag has been provided without the appropriate value...");
                        }

                        skip_parameter = true;
                    }

                    match current_flag {
                        CliFlag::CommandLine => self.command_line = true,
                        CliFlag::Port => {
                            if let Some(value) = current_flag_value {
                                self.port = value.to_string();
                            }
                        }
                        CliFlag::LogLevel => {
                            if let Some(value) = current_flag_value {
                                self.log_level = LogLevel::from(value);
                            }
                        }
                        _ => {
                            self.usage();
                            exit(0)
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn main() {
    let args = std::env::args();
    let mut cli_adapter = CliAdapter::new(args);
    if let Err(message) = cli_adapter.map_arg_to_flag() {
        return eprintln!("Error: {}", message);
    };

    if cli_adapter.command_line {
        let mut proxy = Proxy::new(
            cli_adapter.port,
            service::proxy::ProxyView::Min,
            TrafficFilter::default(),
            cli_adapter.log_level,
        );

        proxy.run();

        loop {
            sleep(Duration::from_millis(1000));
        }
    } else {
        let icon: &[u8] = include_bytes!("assets/icon.png");
        let img: image::DynamicImage = image::load_from_memory(icon).unwrap();

        let options = eframe::NativeOptions {
            follow_system_theme: true,
            viewport: eframe::egui::ViewportBuilder::default()
                .with_decorations(true)
                .with_min_inner_size(egui::vec2(250.0, 160.0))
                .with_resizable(true)
                .with_icon(Arc::new(egui::viewport::IconData {
                    rgba: img.into_bytes(),
                    width: 288,
                    height: 288,
                })),
            persist_window: true,
            ..Default::default()
        };

        eframe::run_native(
            "Proxy Blocker",
            options,
            Box::new(|cc| {
                egui_extras::install_image_loaders(&cc.egui_ctx);
                Ok(Box::new(ui::default_window::MainWindow::new(cc)))
            }),
        )
        .expect("Could not launch UI.")
    }
}
