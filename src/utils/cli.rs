use colored::Colorize;

use service::{proxy::Proxy, traffic_filter::TrafficFilter};
use std::{process::exit, thread::sleep, time::Duration};

use crate::service;

use super::logger::LogLevel;

#[derive(PartialEq, Debug)]
enum Flag {
    CommandLine,
    Port,
    LogLevel,
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
            Flag::Port => true,
            Flag::LogLevel => true,
            _ => false,
        }
    }
}

pub struct CommandLineAdapter {
    command_line: bool,
    port: String,
    log_level: LogLevel,
}

impl Default for CommandLineAdapter {
    fn default() -> Self {
        Self {
            command_line: false,
            port: String::from("8080"),
            log_level: LogLevel::Info,
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
        let mut proxy = Proxy::new(
            self.port.clone(),
            service::proxy::ProxyView::Min,
            TrafficFilter::default(),
            self.log_level.clone(),
        );

        proxy.run();

        loop {
            sleep(Duration::from_millis(1000));
        }
    }
}
