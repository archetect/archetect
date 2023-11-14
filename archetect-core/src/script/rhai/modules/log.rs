use rhai::plugin::*;

pub fn register(engine: &mut Engine) {
    engine.register_global_module(exported_module!(module).into());
}

#[derive(Clone, Debug)]
pub enum LogLevel {
    Info,
    Trace,
    Debug,
    Warn,
    Error,
}

#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use log::{debug, error, info, trace, warn};

    pub type LogLevel = crate::script::rhai::modules::log::LogLevel;

    pub const Info: LogLevel = LogLevel::Info;
    pub const Trace: LogLevel = LogLevel::Trace;
    pub const Debug: LogLevel = LogLevel::Debug;
    pub const Warn: LogLevel = LogLevel::Warn;
    pub const Error: LogLevel = LogLevel::Error;

    pub const INFO: LogLevel = LogLevel::Info;
    pub const TRACE: LogLevel = LogLevel::Trace;
    pub const DEBUG: LogLevel = LogLevel::Debug;
    pub const WARN: LogLevel = LogLevel::Warn;
    pub const ERROR: LogLevel = LogLevel::Error;

    pub fn log(level: LogLevel, message: &str) {
        match level {
            LogLevel::Info => {
                info!("{}", message)
            }
            LogLevel::Trace => {
                trace!("{}", message)
            }
            LogLevel::Debug => {
                debug!("{}", message)
            }
            LogLevel::Warn => {
                warn!("{}", message)
            }
            LogLevel::Error => {
                error!("{}", message)
            }
        }

    }

}
