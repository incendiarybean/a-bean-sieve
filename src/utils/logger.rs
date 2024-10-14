use colored::{ColoredString, Colorize};
use eframe::egui::Color32;
use std::sync::{Arc, Mutex};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug = 3,
    #[default]
    Info = 2,
    Warning = 1,
    Error = 0,
    Global = -1,
}

impl ToString for LogLevel {
    fn to_string(&self) -> String {
        match self {
            LogLevel::Debug => String::from("DEBUG"),
            LogLevel::Info => String::from("INFO"),
            LogLevel::Warning => String::from("WARNING"),
            LogLevel::Error => String::from("ERROR"),
            LogLevel::Global => String::from("GLOBAL"),
        }
    }
}

impl LogLevel {
    pub fn to_colored_string(&self) -> ColoredString {
        match self {
            LogLevel::Info | LogLevel::Global => self.to_string().green(),
            LogLevel::Warning => self.to_string().yellow(),
            LogLevel::Error => self.to_string().red(),
            LogLevel::Debug => self.to_string().cyan(),
        }
    }

    pub fn to_color32(&self) -> Color32 {
        match self {
            LogLevel::Info | LogLevel::Global => Color32::GREEN,
            LogLevel::Warning => Color32::YELLOW,
            LogLevel::Error => Color32::RED,
            LogLevel::Debug => Color32::from_rgb(17, 168, 205),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Log {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Logger {
    level: Arc<Mutex<LogLevel>>,
    logs: Arc<Mutex<Vec<Log>>>,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: Arc::new(Mutex::new(LogLevel::default())),
            logs: Arc::new(Mutex::new(Vec::<Log>::default())),
        }
    }
}

impl Logger {
    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.level.lock().unwrap().clone() {
            let timestamp = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();

            let log = format!(
                "{} :: {} :: {}",
                timestamp.magenta(),
                level.to_colored_string(),
                message
            );
            println!("{}", log);

            self.logs.lock().unwrap().push(Log {
                level: level,
                message: message.to_string(),
                timestamp,
            });
        }
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn global(&self, message: &str) {
        self.log(LogLevel::Global, message);
    }

    pub fn get_logs(&self) -> Vec<Log> {
        self.logs.lock().unwrap().clone()
    }

    pub fn level(&self) -> LogLevel {
        self.level.lock().unwrap().clone()
    }

    pub fn set_level(&mut self, value: LogLevel) {
        *self.level.lock().unwrap() = value.clone();

        let message = format!("Log level has been set to: {}", value.to_string());
        self.log(LogLevel::Global, &message);
    }
}
