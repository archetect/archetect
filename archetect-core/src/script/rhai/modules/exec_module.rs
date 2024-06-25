use std::process::Command;

use rhai::{Dynamic, Engine, EvalAltResult, Map, NativeCallContext};
use tracing::warn;

use archetect_inquire::{Confirm, InquireError};

use crate::Archetect;
use crate::archetype::archetype::Archetype;

pub fn register(engine: &mut Engine, archetect: Archetect, archetype: Archetype) {
    let archetect_clone = archetect.clone();
    let archetype_clone = archetype.clone();
    engine.register_fn("execute", move |call: NativeCallContext, program: &str| {
        execute(call, archetect_clone.clone(), archetype_clone.clone(), program)
    });
    let archetect_clone = archetect.clone();
    let archetype_clone = archetype.clone();
    engine.register_fn(
        "execute",
        move |call: NativeCallContext, program: &str, settings: Map| {
            execute_with_settings(
                call,
                archetect_clone.clone(),
                archetype_clone.clone(),
                program,
                settings,
            )
        },
    );
    let archetect_clone = archetect.clone();
    let archetype_clone = archetype.clone();
    engine.register_fn("capture", move |call: NativeCallContext, program: &str| {
        capture(call, archetect_clone.clone(), archetype_clone.clone(), program)
    });
    let archetect_clone = archetect.clone();
    let archetype_clone = archetype.clone();
    engine.register_fn(
        "capture",
        move |call: NativeCallContext, program: &str, settings: Map| {
            capture_with_settings(
                call,
                archetect_clone.clone(),
                archetype_clone.clone(),
                program,
                settings,
            )
        },
    );
}

fn execute(
    call: NativeCallContext,
    archetect: Archetect,
    archetype: Archetype,
    program: &str,
) -> Result<(), Box<EvalAltResult>> {
    execute_with_settings(call, archetect, archetype, program, Map::new())
}

fn execute_with_settings(
    _call: NativeCallContext,
    _archetect: Archetect,
    _archetype: Archetype,
    program: &str,
    settings: Map,
) -> Result<(), Box<EvalAltResult>> {
    let mut command = create_command(program, settings);

    match command.status() {
        Ok(exit) => {
            if let Some(code) = exit.code() {
                if code != 0 {
                    warn!("Command {:?} exited with error code {}", command.get_program(), code);
                }
            }
        }
        Err(err) => {
            return Err(Box::new(EvalAltResult::ErrorSystem("exec Error".into(), Box::new(err))));
        }
    }

    Ok(())
}

fn capture(
    call: NativeCallContext,
    archetect: Archetect,
    archetype: Archetype,
    program: &str,
) -> Result<Dynamic, Box<EvalAltResult>> {
    capture_with_settings(call, archetect, archetype, program, Map::new())
}

fn capture_with_settings(
    _call: NativeCallContext,
    archetect: Archetect,
    archetype: Archetype,
    program: &str,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let mut command = create_command(program, settings);
    match allow_exec(&archetect, &archetype, &command) {
        Ok(allow) => {
            if allow {
                match command.output() {
                    Ok(output) => {
                        if let Some(code) = output.status.code() {
                            if code != 0 {
                                warn!("Command {:?} exited with error code {}", command.get_program(), code);
                            }
                        }

                        match String::from_utf8(output.stdout) {
                            Ok(result) => {
                                return Ok(result.into());
                            }
                            Err(err) => {
                                return Err(Box::new(EvalAltResult::ErrorSystem(
                                    "exec UTF8 Error".into(),
                                    Box::new(err),
                                )));
                            }
                        }
                    }
                    Err(err) => return Err(Box::new(EvalAltResult::ErrorSystem("exec Error".into(), Box::new(err)))),
                }
            } else {
                return Ok(Dynamic::UNIT);
            }
        }
        Err(err) => {
            return Err(err);
        }
    }
}

fn allow_exec(archetect: &Archetect, _archetype: &Archetype, command: &Command) -> Result<bool, Box<EvalAltResult>> {
    let _foo = command.get_args();

    if let Some(allow_exec) = archetect.configuration().security().allow_exec() {
        return Ok(allow_exec);
    }

    // TODO: Handle IO Backends
    let prompt = Confirm::new("This archetype wants to execute a command on this machine.\nAllow?").with_default(true);

    match prompt.prompt() {
        Ok(value) => return Ok(value),
        Err(err) => match err {
            InquireError::OperationCanceled => {}
            InquireError::OperationInterrupted => {}
            _ => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Exec Error".to_string(),
                    Box::new(err),
                )))
            }
        },
    }
    Ok(true)
}

fn create_command(program: &str, settings: Map) -> Command {
    let mut command = Command::new(program);

    {
        let mut command = &mut command;

        if let Some(args) = settings.get("args") {
            let maybe_args: Option<Vec<Dynamic>> = args.clone().try_cast::<Vec<Dynamic>>();
            if let Some(args) = maybe_args {
                let args = args
                    .into_iter()
                    .filter_map(|arg| arg.try_cast::<String>())
                    .collect::<Vec<String>>();
                command = command.args(args);
            }
        }

        if let Some(env) = settings.get("env") {
            let maybe_env: Option<Map> = env.clone().try_cast::<Map>();
            if let Some(env) = maybe_env {
                for (k, v) in env {
                    if let (Some(key), Some(value)) = (Some(k.to_string()), v.try_cast::<String>()) {
                        command = command.env(key, value);
                    }
                }
            }
        }

        if let Some(directory) = settings.get("directory") {
            if let Some(directory) = directory.clone().try_cast::<String>() {
                match shellexpand::full(directory.as_str()) {
                    Ok(directory) => {
                        command.current_dir(directory.as_ref());
                    }
                    Err(_) => {
                        // TODO: throw error
                    }
                }
            }
        }
    }

    command
}
