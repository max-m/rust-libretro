use super::*;
use log::{Level, Metadata, Record};
use std::io::Write;

pub struct RetroLogger {
    callback: retro_log_callback,
    log_level: Level,
}

impl RetroLogger {
    pub fn new(callback: retro_log_callback) -> Self {
        Self {
            callback,
            log_level: Level::Debug,
        }
    }

    fn get_retro_log_level(level: Level) -> retro_log_level {
        match level {
            Level::Error => retro_log_level::RETRO_LOG_ERROR,
            Level::Warn => retro_log_level::RETRO_LOG_WARN,
            Level::Info => retro_log_level::RETRO_LOG_INFO,
            Level::Debug => retro_log_level::RETRO_LOG_DEBUG,
            Level::Trace => retro_log_level::RETRO_LOG_DEBUG,
        }
    }
}

impl log::Log for RetroLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let target = if !record.target().is_empty() {
            record.target()
        } else {
            record.module_path().unwrap_or_default()
        };

        if let Some(cb) = self.callback.log {
            let mut args: Vec<u8> = Vec::new();

            if writeln!(args, "{}\0", record.args()).is_ok() {
                let level = Self::get_retro_log_level(record.level());
                let target = CString::new(target).unwrap();

                unsafe {
                    let args = CString::from_vec_unchecked(args);

                    // The callback works like `printf`
                    (cb)(
                        level,
                        "[%s] %s\n\0".as_ptr() as *const c_char,
                        target.as_ptr() as *const c_char,
                        args.as_ptr() as *const c_char,
                    )
                }
            }
        } else {
            let level = match record.level() {
                Level::Debug => "DEBUG",
                Level::Info => "INFO",
                Level::Warn => "WARN",
                Level::Error => "ERROR",
                Level::Trace => "TRACE",
            };

            let stderr = std::io::stderr();
            let mut stderr_lock = stderr.lock();

            let _ = writeln!(
                stderr_lock,
                "[libretro {}] [{}] {}",
                level,
                target,
                record.args()
            );
        }
    }

    fn flush(&self) {
        // Do nothing
    }
}
