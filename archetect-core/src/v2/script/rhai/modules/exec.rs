use log::warn;
use rhai::{Dynamic, Engine, EvalAltResult, Map};
use std::process::Command;

pub fn register(engine: &mut Engine) {
    engine.register_fn("execute", execute);
    engine.register_fn("capture", capture);
}

fn execute(program: &str, settings: Map) -> Result<(), Box<EvalAltResult>> {
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

fn capture(program: &str, settings: Map) -> Result<Dynamic, Box<EvalAltResult>> {
    let mut command = create_command(program, settings);

    match command.output() {
        Ok(output) => {
            if let Some(code) = output.status.code() {
                if code != 0 {
                    warn!("Command {:?} exited with error code {}", command.get_program(), code);
                }
            }

            match String::from_utf8(output.stdout) {
                Ok(result) => Ok(result.into()),
                Err(err) => {
                    Err(Box::new(EvalAltResult::ErrorSystem("exec UTF8 Error".into(), Box::new(err))))
                }
            }
        }
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem("exec Error".into(), Box::new(err)))),
    }
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
                command = command.current_dir(directory);
            }
        }
    }

    command
}
