use crate::Archetect;
use archetect_api::CommandRequest;
use rhai::plugin::*;

pub fn register(engine: &mut Engine, archetect: Archetect) {
    engine.register_global_module(exported_module!(module).into());
    engine.register_fn("log", move |level: LogLevel, message: &str| match level {
        LogLevel::Info => archetect.request(CommandRequest::LogInfo(message.to_string())),
        LogLevel::Trace => archetect.request(CommandRequest::LogTrace(message.to_string())),
        LogLevel::Debug => archetect.request(CommandRequest::LogDebug(message.to_string())),
        LogLevel::Warn => archetect.request(CommandRequest::LogWarn(message.to_string())),
        LogLevel::Error => archetect.request(CommandRequest::LogError(message.to_string())),
    });
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
    pub type LogLevel = crate::script::rhai::modules::log_module::LogLevel;
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
}
