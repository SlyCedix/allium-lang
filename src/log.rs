use std::fmt::Display;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Verbose,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    fn as_ansi(self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[38;5;6m",
            LogLevel::Verbose => "\x1b[38;5;8m",
            LogLevel::Info => "\x1b[39m",
            LogLevel::Warn => "\x1b[38;5;3m",
            LogLevel::Error => "\x1b[38;5;1m",
            LogLevel::Fatal => "\x1b[38;5;0;48;5;1m",
        }
    }

    fn as_pre(self) -> &'static str {
        match self {
            LogLevel::Debug => "[DBG] ",
            LogLevel::Verbose => "[VRB] ",
            LogLevel::Info => "[INF] ",
            LogLevel::Warn => "[WRN] ",
            LogLevel::Error => "[ERR] ",
            LogLevel::Fatal => "[FTL] ",
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Logger {
    use_ansi_color: bool,
    minimum_level: LogLevel,
}

impl Logger {
    pub fn new(use_ansi_color: bool, minimum_level: LogLevel) -> Self {
        Self {
            use_ansi_color,
            minimum_level,
        }
    }

    fn put_display<T: Display>(self, level: LogLevel, message: T) {
        match level {
            LogLevel::Debug | LogLevel::Verbose | LogLevel::Info => print!("{}", message),
            LogLevel::Warn | LogLevel::Error | LogLevel::Fatal => eprint!("{}", message),
        }
    }

    fn put_color(self, level: LogLevel) {
        if !self.use_ansi_color {
            return;
        }

        self.put_display(level, level.as_ansi());
    }

    pub fn log<T: Display>(self, level: LogLevel, message: T) {
        if self.minimum_level > level {
            return;
        }

        self.put_color(level);
        self.put_display(level, level.as_pre());
        self.put_display(level, message);
    }

    pub fn debug(self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn verbose(self, message: &str) {
        self.log(LogLevel::Verbose, message);
    }

    pub fn info(self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn fatal(self, message: &str) {
        self.log(LogLevel::Fatal, message);
    }
}
