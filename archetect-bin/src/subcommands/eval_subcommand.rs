//! `archetect eval '<lua>'` — one-shot probe of the scripting environment.
//!
//! The autodidact loop's third leg (learn → introspect → EVAL): run one snippet against the real
//! Lua surface — Context, Cases, template filters, format, the model API — without authoring an
//! archetype. Implemented honestly: the snippet IS an archetype (a synthesized one in a temp
//! dir), rendered through the ordinary pipeline into a temp destination, so what eval proves is
//! exactly what a render would do. Always headless (a probe that prompts is an error), and
//! shell-exec stays on the configuration's policy — side-effect modules need `--allow-exec`
//! exactly like a render.

use clap::ArgMatches;
use std::io::Read;

use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;
use archetect_core::system::XdgSystemLayout;
use archetect_core::Archetect;
use archetect_terminal_io::TerminalScriptIoHandle;

/// The wrapper that turns a snippet into an archetype script: run it as a function body and
/// print a non-nil result as YAML through the ordinary output channel.
fn probe_script(code: &str) -> String {
    format!(
        "local __eval = function()\n{code}\nend\nlocal __result = __eval()\nif __result ~= nil then\n  output.print(format.to_yaml(__result))\nend\n"
    )
}

pub fn handle_eval_subcommand(args: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    let code = match args.get_one::<String>("code").map(String::as_str) {
        Some("-") | None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| ArchetectError::GeneralError(format!("eval: reading stdin: {e}")))?;
            buf
        }
        Some(code) => code.to_string(),
    };
    if code.trim().is_empty() {
        return Err(ArchetectError::GeneralError(
            "eval: the snippet is empty (pass Lua as the argument, or on stdin with `-`) — end \
             with `return <value>` to print a result"
                .into(),
        ));
    }

    // The synthesized probe package: manifest + script in a temp root, output in a temp dest.
    // Both live inside one TempDir so teardown is a single drop, even on error.
    let root = tempfile::TempDir::with_prefix("archetect-eval-")
        .map_err(|e| ArchetectError::GeneralError(format!("eval: temp dir: {e}")))?;
    let dest = root.path().join("out");
    std::fs::create_dir_all(&dest)
        .and_then(|_| {
            std::fs::write(
                root.path().join("archetype.yaml"),
                "description: archetect eval probe\n",
            )
        })
        .and_then(|_| std::fs::write(root.path().join("archetype.lua"), probe_script(&code)))
        .map_err(|e| ArchetectError::GeneralError(format!("eval: staging the probe: {e}")))?;

    // Same configuration as the session, but ALWAYS headless: a probe that blocks on a prompt
    // is a bug in the probe (prompted keys error, naming themselves — that error is an answer).
    let configuration = archetect.configuration().clone().with_headless(true);
    let probe = Archetect::builder()
        .with_configuration(configuration)
        .with_driver(TerminalScriptIoHandle::default())
        .with_layout(
            XdgSystemLayout::new()
                .map_err(|e| ArchetectError::GeneralError(format!("eval: system layout: {e}")))?,
        )
        .build()?;

    let source = probe.new_source(root.path().to_str().ok_or_else(|| {
        ArchetectError::GeneralError("eval: non-UTF-8 temp path".to_string())
    })?)?;
    let archetype = Archetype::new(probe, source)?;
    let render_context = RenderContext::new(
        camino::Utf8PathBuf::from(dest.to_string_lossy().to_string()),
        Default::default(),
    );
    archetype.render(render_context).map(|_| ())?;
    Ok(())
}
