use colored::Colorize;

use service::{proxy::Proxy, traffic_filter::TrafficFilter};
use std::{path::PathBuf, process::exit, thread::sleep, time::Duration};

use crate::service::{self, traffic_filter::TrafficFilterType};

use super::{csv_handler::read_from_csv, logger::LogLevel};

#[derive(PartialEq, Debug)]
enum Flag {
    CommandLine,
    Port,
    LogLevel,
    TrafficFilter,
    TrafficFilterType,
    TrafficFilterList,
    Help,
    Value,
}

/// Convert the provided flag to one of the Flag enums
impl From<&String> for Flag {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "--no-ui" => Flag::CommandLine,
            "--port" | "-p" => Flag::Port,
            "--log-level" | "-l" => Flag::LogLevel,
            "--filter" | "-f" => Flag::TrafficFilter,
            "--filter-type" | "-ft" => Flag::TrafficFilterType,
            "--filter-list" | "-fl" => Flag::TrafficFilterList,
            "--help" | "-h" => Flag::Help,
            // Default to Value, if it isn't a flag
            _ => Flag::Value,
        }
    }
}

impl Flag {
    /// Check whether the provided flag requires a value to be passed with it
    fn requires_value(&self) -> bool {
        match self {
            Flag::Port | Flag::LogLevel | Flag::TrafficFilterType | Flag::TrafficFilterList => true,
            _ => false,
        }
    }
}

pub struct CommandLineAdapter {
    command_line: bool,
    port: String,
    log_level: LogLevel,
    traffic_filter: bool,
    traffic_filter_type: TrafficFilterType,
    traffic_filter_list: Vec<String>,
}

impl Default for CommandLineAdapter {
    fn default() -> Self {
        Self {
            command_line: false,
            port: String::from("8080"),
            log_level: LogLevel::Info,
            traffic_filter: false,
            traffic_filter_type: TrafficFilterType::Allow,
            traffic_filter_list: Vec::<String>::new(),
        }
    }
}

impl CommandLineAdapter {
    /// Returns if the --no-ui flag is present.
    pub fn cmd_only(&self) -> bool {
        self.command_line
    }

    /// Print a usage message in the terminal.
    fn usage(&self) {
        println!("");
        println!("{}", "Example Usage:".blue());
        println!(
            "  {}",
            "a-bean-sieve.exe --no-gui --port 8080 --log-level INFO".yellow()
        );
        println!("");
        println!("{}", "Available Flags:".blue());
        println!("  --no-ui : Use the tool in CLI mode.");
        println!("  --port | -p : Choose the port to run the proxy on.");
        println!(
            "  --log-level | -l : The logging level, one of ['debug', 'info', 'warning', 'error']."
        );
        println!(
            "  --filter | -f : Whether the traffic filter is enabled or not (default is true when --filter-type or --filter-list flags are provided."
        );
        println!("  --filter-type | -l : The filter type, one of ['allow', 'deny'].");
        println!("  --filter-list | -fl : The path to the exclusion list to import, e.g. './exclusion-list.csv'.");
        println!("  --help | -h : Print usage and flags.");
        println!("");

        println!("{}", "Further Information:".blue());
        println!("  Find more on the GitHub: https://github.com/incendiarybean/a-bean-sieve");
    }

    /// Map arguments passed to the application to CommandLineAdapter values.
    pub fn map_arg_to_flag(&mut self) -> Result<(), String> {
        // Collect args, and remove path as first arg
        let mut arguments = std::env::args().collect::<Vec<String>>();
        arguments.remove(0);

        // If previous value was a flag, we've already processed the parameter
        let mut skip_parameter = false;

        // Loop through each argument, if a Flag is the current argument find the value
        for (index, argument) in arguments.iter().enumerate() {
            match skip_parameter {
                true => skip_parameter = false,
                false => {
                    let current_flag = Flag::from(argument);
                    if current_flag == Flag::Value {
                        return Err(format!(
                            "Value: {:?}, has been provided without the appropriate flag...",
                            current_flag
                        ));
                    }

                    let current_flag_value = arguments.get(index + 1);
                    if current_flag.requires_value() {
                        if current_flag_value.is_none() {
                            return Err(String::from(format!(
                                "Flag: {:?}, has been provided without the appropriate value...",
                                current_flag
                            )));
                        }

                        skip_parameter = true;
                    }

                    match current_flag {
                        Flag::CommandLine => self.command_line = true,
                        Flag::Port => {
                            if let Some(value) = current_flag_value {
                                self.port = value.to_string();
                            }
                        }
                        Flag::LogLevel => {
                            if let Some(value) = current_flag_value {
                                self.log_level = LogLevel::from(value);
                            }
                        }
                        Flag::TrafficFilter => self.traffic_filter = true,
                        Flag::TrafficFilterType => {
                            if !self.traffic_filter {
                                self.traffic_filter = true;
                            }

                            if let Some(value) = current_flag_value {
                                match value.to_lowercase().as_str() {
                                    "allow" => self.traffic_filter_type = TrafficFilterType::Allow,
                                    "deny" => self.traffic_filter_type = TrafficFilterType::Deny,
                                    _ => {
                                        return Err(format!(
                                            "Value: {value} is not one of ['ALLOW', 'DENY']."
                                        ))
                                    }
                                }
                            }
                        }
                        Flag::TrafficFilterList => {
                            if !self.traffic_filter {
                                self.traffic_filter = true;
                            }

                            if let Some(value) = current_flag_value {
                                match read_from_csv::<String, PathBuf>(value.into()) {
                                    Ok(list) => {
                                        self.traffic_filter_list = list;
                                    }
                                    Err(message) => {
                                        return Err(format!(
                                            "CSV could not be imported - {}",
                                            message.to_string()
                                        ))
                                    }
                                }
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

    /// Run the Proxy as a CMD process.
    pub fn run(&self) {
        let mut traffic_filter = TrafficFilter::default();
        traffic_filter.set_enabled(self.traffic_filter);
        traffic_filter.set_filter_type(self.traffic_filter_type);
        traffic_filter.set_filter_list(self.traffic_filter_list.clone());

        let mut proxy = Proxy::new(
            self.port.clone(),
            service::proxy::ProxyView::Min,
            traffic_filter,
            self.log_level.clone(),
        );

        proxy.run();

        loop {
            sleep(Duration::from_millis(1000));
        }
    }
}
