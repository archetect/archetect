use rhai::plugin::*;
use archetect_api::CommandRequest;
use crate::runtime::context::RuntimeContext;

pub fn register(engine: &mut Engine, runtime_context: RuntimeContext) {
    engine.register_global_module(exported_module!(module).into());
    engine.register_fn("log", move| level: LogLevel, message: &str| {
        match level {
            LogLevel::Info => {
                runtime_context.request(CommandRequest::LogInfo(message.to_string()))
            }
            LogLevel::Trace => {
                runtime_context.request(CommandRequest::LogTrace(message.to_string()))
            }
            LogLevel::Debug => {
                runtime_context.request(CommandRequest::LogDebug(message.to_string()))
            }
            LogLevel::Warn => {
                runtime_context.request(CommandRequest::LogWarn(message.to_string()))
            }
            LogLevel::Error => {
                runtime_context.request(CommandRequest::LogError(message.to_string()))
            }
        }
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
}
