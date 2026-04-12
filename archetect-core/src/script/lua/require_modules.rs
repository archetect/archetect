use std::env;
use std::process::{Command, Stdio};

use camino::Utf8PathBuf;
use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, UserData, UserDataMethods};

use archetect_api::ScriptMessage;

/// Run a shell command, capturing stdout/stderr and forwarding them as log
/// messages through the Archetect IO channel.
///
/// This is mandatory for stdio-based IO drivers (MCP): letting subprocesses
/// inherit stdin/stdout/stderr would pollute the JSON-RPC protocol stream and
/// crash the transport. Capturing also gives the `log` module a chance to
/// surface output coherently regardless of driver.
fn run_logged(archetect: &Archetect, cmd: &mut Command, label: &str) -> LuaResult<std::process::ExitStatus> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| LuaError::RuntimeError(format!("{}: {}", label, e)))?;

    if !output.stdout.is_empty() {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let _ = archetect.request(ScriptMessage::LogInfo(line.to_string()));
        }
    }
    if !output.stderr.is_empty() {
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            let _ = archetect.request(ScriptMessage::LogInfo(line.to_string()));
        }
    }
    Ok(output.status)
}

use crate::archetype::render_context::RenderContext;
use crate::Archetect;

/// Register archetect.* modules available via require()
pub fn register_require_modules(
    lua: &Lua,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> LuaResult<()> {
    let preload = lua
        .globals()
        .get::<Table>("package")?
        .get::<Table>("preload")?;

    // archetect.shell
    {
        let arc = archetect.clone();
        let ctx = render_context.clone();
        preload.set(
            "archetect.shell",
            lua.create_function(move |lua, ()| create_shell_module(lua, &arc, &ctx))?,
        )?;
    }

    // archetect.git
    {
        let arc = archetect.clone();
        let ctx = render_context.clone();
        preload.set(
            "archetect.git",
            lua.create_function(move |lua, ()| create_git_module(lua, &arc, &ctx))?,
        )?;
    }

    // archetect.github
    {
        let arc = archetect.clone();
        preload.set(
            "archetect.github",
            lua.create_function(move |lua, ()| create_github_module(lua, &arc))?,
        )?;
    }

    // archetect.archive
    {
        let ctx = render_context.clone();
        preload.set(
            "archetect.archive",
            lua.create_function(move |lua, ()| create_archive_module(lua, &ctx))?,
        )?;
    }

    // archetect.model
    {
        preload.set(
            "archetect.model",
            lua.create_function(|lua, ()| create_model_module(lua))?,
        )?;
    }

    // archetect.model.interactive — Lua-implemented interactive builder
    {
        preload.set(
            "archetect.model.interactive",
            lua.create_function(|lua, ()| {
                let module = lua.create_table()?;
                let func = lua.load(INTERACTIVE_MODEL_LUA).eval::<mlua::Function>()?;
                module.set("build", func)?;
                Ok(module)
            })?,
        )?;
    }

    Ok(())
}

const INTERACTIVE_MODEL_LUA: &str = r#"
-- archetect.model.interactive.build(context)
-- Drives an interactive prompt session to build an AML model.
-- Returns a resolved model (same type as model.load() or model.parse()).
return function(context)
    local model = require("archetect.model")
    local builder = model.builder()

    -- Identity
    context:prompt_text("Organization", "organization")
    builder:set_organization(context:get("organization"))

    context:prompt_text("Solution", "solution")
    builder:set_solution(context:get("solution"))

    context:prompt_text("Description", "description", { optional = true, default = "" })
    local desc = context:get("description")
    if desc and desc ~= "" then
        builder:set_description(desc)
    end

    -- Built-in field types for selection
    local field_types = {
        "String", "Integer", "Long", "Decimal", "Boolean",
        "UUID", "Date", "Timestamp", "Bytes"
    }

    -- Entity loop
    while true do
        context:prompt_confirm("Add an entity?", "__add_entity", { default = true })
        if not context:get("__add_entity") then break end

        context:prompt_text("Entity name", "__entity_name")
        local entity_name = context:get("__entity_name")
        builder:add_entity(entity_name)

        -- Field loop for this entity
        while true do
            context:prompt_confirm("  Add a field to " .. entity_name .. "?", "__add_field", { default = true })
            if not context:get("__add_field") then break end

            context:prompt_text("  Field name", "__field_name")
            local field_name = context:get("__field_name")

            context:prompt_select("  Field type", "__field_type", field_types, { default = "String" })
            local field_type = context:get("__field_type")

            builder:add_field(entity_name, field_name, field_type)
        end
    end

    -- Boundary loop
    local entity_names = {}
    while true do
        context:prompt_confirm("Add a service boundary?", "__add_boundary", { default = true })
        if not context:get("__add_boundary") then break end

        context:prompt_text("  Boundary name", "__boundary_name")
        local boundary_name = context:get("__boundary_name")

        context:prompt_select("  Boundary type", "__boundary_type",
            {"service", "gateway", "library", "orchestrator", "adapter"},
            { default = "service" })
        local boundary_type = context:get("__boundary_type")

        -- TODO: multi-select from entities for "owns" once we can list them
        context:prompt_text("  Owned entities (comma-separated)", "__owns", { optional = true, default = "" })
        local owns_str = context:get("__owns") or ""
        local owns = {}
        for name in owns_str:gmatch("[^,]+") do
            table.insert(owns, name:match("^%s*(.-)%s*$"))
        end

        builder:add_boundary(boundary_name, boundary_type, owns)
    end

    -- Interface loop
    while true do
        context:prompt_confirm("Add an interface between boundaries?", "__add_interface", { default = false })
        if not context:get("__add_interface") then break end

        context:prompt_text("  From boundary", "__iface_from")
        context:prompt_text("  To boundary", "__iface_to")
        context:prompt_select("  Style", "__iface_style", {"sync", "async", "stream"}, { default = "sync" })

        builder:add_interface(
            context:get("__iface_from"),
            context:get("__iface_to"),
            context:get("__iface_style")
        )
    end

    return builder:build()
end
"#;

// --- shell module ---

fn create_shell_module(lua: &Lua, archetect: &Archetect, render_context: &RenderContext) -> LuaResult<Table> {
    let module = lua.create_table()?;
    let default_cwd = render_context.destination().to_string();

    let arc = archetect.clone();
    let cwd = default_cwd.clone();
    module.set(
        "run",
        lua.create_function(move |_, (program, args, opts): (String, Option<Vec<String>>, Option<Table>)| {
            let cwd_override = opts.as_ref().and_then(|o| o.get::<String>("cwd".to_string()).ok());
            let working_dir = cwd_override.unwrap_or_else(|| cwd.clone());
            let args = args.unwrap_or_default();

            authorize_shell_exec(&arc, &program, &args, &working_dir)?;

            let mut cmd = Command::new(&program);
            cmd.args(&args);
            cmd.current_dir(&working_dir);

            let status = run_logged(&arc, &mut cmd, &format!("Failed to run '{}'", program))?;

            if !status.success() {
                return Err(LuaError::RuntimeError(format!(
                    "'{}' exited with status {}",
                    program,
                    status.code().unwrap_or(-1)
                )));
            }
            Ok(())
        })?,
    )?;

    let arc = archetect.clone();
    let cwd = default_cwd.clone();
    module.set(
        "capture",
        lua.create_function(move |_, (program, args, opts): (String, Option<Vec<String>>, Option<Table>)| {
            let cwd_override = opts.as_ref().and_then(|o| o.get::<String>("cwd".to_string()).ok());
            let working_dir = cwd_override.unwrap_or_else(|| cwd.clone());
            let args = args.unwrap_or_default();

            authorize_shell_exec(&arc, &program, &args, &working_dir)?;

            let mut cmd = Command::new(&program);
            cmd.args(&args);
            cmd.current_dir(&working_dir);

            let output = cmd.output().map_err(|e| {
                LuaError::RuntimeError(format!("Failed to run '{}': {}", program, e))
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(LuaError::RuntimeError(format!(
                    "'{}' failed: {}",
                    program, stderr
                )));
            }

            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        })?,
    )?;

    Ok(module)
}

/// Gates a shell exec call against the configured `ShellExecPolicy`.
///
/// - `Forbidden` → immediate error (used by MCP mode).
/// - `Allowed` → proceed without prompting.
/// - `Prompt` → ask the user via the IO channel, showing the exact command.
///   In headless mode, the prompt fails and the call is denied.
fn authorize_shell_exec(
    archetect: &Archetect,
    program: &str,
    args: &[String],
    cwd: &str,
) -> LuaResult<()> {
    use crate::configuration::ShellExecPolicy;
    use archetect_api::{BoolPromptInfo, ClientMessage, ScriptMessage};

    match archetect.configuration().shell_exec_policy() {
        ShellExecPolicy::Allowed => Ok(()),
        ShellExecPolicy::Forbidden => Err(LuaError::RuntimeError(format!(
            "Shell execution is forbidden in this mode. Attempted: '{}'",
            format_command(program, args)
        ))),
        ShellExecPolicy::Prompt => {
            // Send a Display message first so the user sees the command details,
            // then prompt for confirmation.
            let detail = format!(
                "Archetype wants to execute a shell command:\n  Command: {}\n  Working dir: {}",
                format_command(program, args),
                cwd
            );
            let _ = archetect.request(ScriptMessage::Display(detail));

            let prompt = BoolPromptInfo::new("Allow this command?", None::<&str>)
                .with_default(Some(false));
            archetect
                .request(ScriptMessage::PromptForBool(prompt))
                .map_err(|e| {
                    LuaError::RuntimeError(format!("Shell exec confirmation failed: {}", e))
                })?;

            match archetect.response() {
                Ok(ClientMessage::Boolean(true)) => Ok(()),
                Ok(ClientMessage::Boolean(false)) | Ok(ClientMessage::None) => {
                    Err(LuaError::RuntimeError(format!(
                        "Shell execution denied by user: '{}'",
                        format_command(program, args)
                    )))
                }
                Ok(ClientMessage::Abort) => Err(LuaError::RuntimeError(
                    "Shell execution aborted".to_string(),
                )),
                Ok(other) => Err(LuaError::RuntimeError(format!(
                    "Unexpected response to shell confirmation: {:?}",
                    other
                ))),
                Err(e) => Err(LuaError::RuntimeError(format!(
                    "Shell exec confirmation channel error: {}",
                    e
                ))),
            }
        }
    }
}

fn format_command(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

// --- git module ---

#[derive(Clone, Debug)]
struct GitRepo {
    path: String,
    archetect: Archetect,
}

impl UserData for GitRepo {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |_, this, pattern: String| {
            git_cmd(&this.archetect, &this.path, &["add", &pattern])
        });

        methods.add_method("add_all", |_, this, ()| {
            git_cmd(&this.archetect, &this.path, &["add", "-A"])
        });

        methods.add_method("commit", |_, this, message: String| {
            git_cmd(&this.archetect, &this.path, &["commit", "-m", &message])
        });

        methods.add_method("branch", |_, this, name: String| {
            git_cmd(&this.archetect, &this.path, &["branch", &name])
        });

        methods.add_method("checkout", |_, this, name: String| {
            git_cmd(&this.archetect, &this.path, &["checkout", &name])
        });

        methods.add_method("remote_add", |_, this, (name, url): (String, String)| {
            git_cmd(&this.archetect, &this.path, &["remote", "add", &name, &url])
        });

        methods.add_method("push", |_, this, (remote, branch): (String, String)| {
            git_cmd(&this.archetect, &this.path, &["push", &remote, &branch])
        });
    }
}

fn git_cmd(archetect: &Archetect, path: &str, args: &[&str]) -> LuaResult<()> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(path);
    let status = run_logged(archetect, &mut cmd, "git error")?;

    if !status.success() {
        return Err(LuaError::RuntimeError(format!(
            "git {} failed with status {}",
            args.join(" "),
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

fn create_git_module(lua: &Lua, archetect: &Archetect, render_context: &RenderContext) -> LuaResult<Table> {
    let module = lua.create_table()?;
    let default_dest = render_context.destination().to_string();
    let arc = archetect.clone();

    module.set(
        "init",
        lua.create_function(move |_, (path, opts): (Option<String>, Option<Table>)| {
            let repo_path = match path {
                Some(p) => format!("{}/{}", default_dest, p),
                None => default_dest.clone(),
            };

            let branch = opts
                .as_ref()
                .and_then(|o| o.get::<String>("branch".to_string()).ok());

            let mut args = vec!["init"];
            let branch_str;
            if let Some(ref b) = branch {
                args.push("-b");
                branch_str = b.clone();
                args.push(&branch_str);
            }
            args.push(&repo_path);

            let mut cmd = Command::new("git");
            cmd.args(&args);
            let status = run_logged(&arc, &mut cmd, "git init error")?;

            if !status.success() {
                return Err(LuaError::RuntimeError("git init failed".to_string()));
            }

            Ok(GitRepo { path: repo_path, archetect: arc.clone() })
        })?,
    )?;

    Ok(module)
}

// --- github module ---

fn create_github_module(lua: &Lua, archetect: &Archetect) -> LuaResult<Table> {
    let module = lua.create_table()?;

    module.set(
        "repo_exists",
        lua.create_function(|_, repo: String| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| LuaError::RuntimeError(format!("Runtime error: {}", e)))?;

            runtime.block_on(async {
                let token = env::var("GITHUB_TOKEN").map_err(|_| {
                    LuaError::RuntimeError(
                        "GITHUB_TOKEN environment variable not set".to_string(),
                    )
                })?;

                let octocrab = octocrab::Octocrab::builder()
                    .personal_token(token)
                    .build()
                    .map_err(|e| LuaError::RuntimeError(format!("GitHub client error: {}", e)))?;

                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 {
                    return Err(LuaError::RuntimeError(
                        "Repository must be in 'owner/repo' format".to_string(),
                    ));
                }

                match octocrab.repos(parts[0], parts[1]).get().await {
                    Ok(_) => Ok(true),
                    Err(octocrab::Error::GitHub { source, .. })
                        if source.message == "Not Found" =>
                    {
                        Ok(false)
                    }
                    Err(e) => Err(LuaError::RuntimeError(format!(
                        "GitHub API error: {}",
                        e
                    ))),
                }
            })
        })?,
    )?;

    let arc = archetect.clone();
    module.set(
        "create_repo",
        lua.create_function(move |_, (repo, opts): (String, Option<Table>)| {
            let arc = arc.clone();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| LuaError::RuntimeError(format!("Runtime error: {}", e)))?;

            runtime.block_on(async {
                let token = env::var("GITHUB_TOKEN").map_err(|_| {
                    LuaError::RuntimeError(
                        "GITHUB_TOKEN environment variable not set".to_string(),
                    )
                })?;

                let octocrab = octocrab::Octocrab::builder()
                    .personal_token(token)
                    .build()
                    .map_err(|e| LuaError::RuntimeError(format!("GitHub client error: {}", e)))?;

                let parts: Vec<&str> = repo.split('/').collect();
                if parts.len() != 2 {
                    return Err(LuaError::RuntimeError(
                        "Repository must be in 'owner/repo' format".to_string(),
                    ));
                }

                let owner = parts[0];
                let repo_name = parts[1];

                let visibility = opts
                    .as_ref()
                    .and_then(|o| o.get::<String>("visibility".to_string()).ok())
                    .unwrap_or_else(|| "private".to_string());

                let is_private = visibility == "private";

                // Check if already exists
                match octocrab.repos(owner, repo_name).get().await {
                    Ok(_) => {
                        let _ = arc.request(ScriptMessage::LogWarn(format!(
                            "Repository '{}/{}' already exists",
                            owner, repo_name
                        )));
                        return Ok(false);
                    }
                    Err(octocrab::Error::GitHub { source, .. })
                        if source.message == "Not Found" => {}
                    Err(e) => {
                        return Err(LuaError::RuntimeError(format!(
                            "GitHub API error: {}",
                            e
                        )));
                    }
                }

                let current_user = octocrab
                    .current()
                    .user()
                    .await
                    .map_err(|e| LuaError::RuntimeError(format!("Failed to get user: {}", e)))?;

                let body = serde_json::json!({
                    "name": repo_name,
                    "private": is_private,
                    "visibility": visibility,
                    "auto_init": false,
                });

                let endpoint = if current_user.login == owner {
                    "/user/repos".to_string()
                } else {
                    format!("/orgs/{}/repos", owner)
                };

                use http_body_util::BodyExt;
                let response = octocrab
                    ._post(&endpoint, Some(&body))
                    .await
                    .map_err(|e| LuaError::RuntimeError(format!("Create repo failed: {}", e)))?;

                let body_bytes = response
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| LuaError::RuntimeError(format!("Read response failed: {}", e)))?
                    .to_bytes();

                let repo_data: serde_json::Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| LuaError::RuntimeError(format!("Parse response failed: {}", e)))?;

                if repo_data.get("id").is_some() {
                    let _ = arc.request(ScriptMessage::LogInfo(format!(
                        "Created {} repository '{}/{}'",
                        visibility, owner, repo_name
                    )));
                    Ok(true)
                } else {
                    Err(LuaError::RuntimeError(
                        "Failed to create repository: unexpected response".to_string(),
                    ))
                }
            })
        })?,
    )?;

    Ok(module)
}

// --- archive module ---

fn create_archive_module(lua: &Lua, render_context: &RenderContext) -> LuaResult<Table> {
    let module = lua.create_table()?;
    let dest = render_context.destination().to_string();

    let d = dest.clone();
    module.set(
        "zip",
        lua.create_function(move |_, (source, destination): (String, String)| {
            let source_path = Utf8PathBuf::from(format!("{}/{}", d, source));
            let dest_path = Utf8PathBuf::from(format!("{}/{}", d, destination));
            crate::archive::create_zip_archive(&source_path, &dest_path)
                .map_err(|e| LuaError::RuntimeError(format!("zip error: {}", e)))
        })?,
    )?;

    let d = dest.clone();
    module.set(
        "tar_gz",
        lua.create_function(move |_, (source, destination): (String, String)| {
            let source_path = Utf8PathBuf::from(format!("{}/{}", d, source));
            let dest_path = Utf8PathBuf::from(format!("{}/{}", d, destination));
            crate::archive::create_tar_archive(&source_path, &dest_path, true)
                .map_err(|e| LuaError::RuntimeError(format!("tar_gz error: {}", e)))
        })?,
    )?;

    let d = dest.clone();
    module.set(
        "tar",
        lua.create_function(move |_, (source, destination): (String, String)| {
            let source_path = Utf8PathBuf::from(format!("{}/{}", d, source));
            let dest_path = Utf8PathBuf::from(format!("{}/{}", d, destination));
            crate::archive::create_tar_archive(&source_path, &dest_path, false)
                .map_err(|e| LuaError::RuntimeError(format!("tar error: {}", e)))
        })?,
    )?;

    Ok(module)
}

// --- model module ---

use archetect_aml::{
    BoundarySlice, CaseVariants, ExpandedEntity, ExpandedField, ModelBuilder,
    ResolvedModel, RemoteReference,
};

/// Wrapper for ResolvedModel exposed as Lua userdata.
struct LuaModel(ResolvedModel);

impl UserData for LuaModel {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("entity", |lua, this, name: String| {
            match this.0.entity(&name) {
                Some(entity) => {
                    let expanded = this.0.expand_entity(entity);
                    expanded_entity_to_lua(lua, &expanded)
                }
                None => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("boundary", |lua, this, name: String| {
            match this.0.boundary(&name) {
                Some(b) => boundary_to_lua(lua, b),
                None => Ok(mlua::Value::Nil),
            }
        });

        methods.add_method("all_boundaries", |lua, this, ()| {
            let table = lua.create_table()?;
            for (i, b) in this.0.all_boundaries().iter().enumerate() {
                table.set(i + 1, boundary_to_lua(lua, b)?)?;
            }
            Ok(mlua::Value::Table(table))
        });

        methods.add_method("boundaries_of_type", |lua, this, btype: String| {
            let table = lua.create_table()?;
            for (i, b) in this.0.boundaries_of_type(&btype).iter().enumerate() {
                table.set(i + 1, boundary_to_lua(lua, b)?)?;
            }
            Ok(mlua::Value::Table(table))
        });

        methods.add_method("entities_for", |lua, this, boundary_name: String| {
            let entities = this.0.entities_for(&boundary_name);
            let table = lua.create_table()?;
            for (i, e) in entities.iter().enumerate() {
                table.set(i + 1, expanded_entity_to_lua(lua, e)?)?;
            }
            Ok(mlua::Value::Table(table))
        });

        methods.add_method("outbound_interfaces", |lua, this, name: String| {
            let ifaces = this.0.outbound_interfaces(&name);
            interfaces_to_lua(lua, ifaces)
        });

        methods.add_method("inbound_interfaces", |lua, this, name: String| {
            let ifaces = this.0.inbound_interfaces(&name);
            interfaces_to_lua(lua, ifaces)
        });

        methods.add_method("dependencies", |lua, this, name: String| {
            let deps = this.0.dependencies(&name);
            let table = lua.create_table()?;
            for (i, d) in deps.iter().enumerate() {
                table.set(i + 1, d.as_str())?;
            }
            Ok(mlua::Value::Table(table))
        });

        methods.add_method("remote_references", |lua, this, name: String| {
            let refs = this.0.remote_references(&name);
            let table = lua.create_table()?;
            for (i, r) in refs.iter().enumerate() {
                table.set(i + 1, remote_ref_to_lua(lua, r)?)?;
            }
            Ok(mlua::Value::Table(table))
        });

        methods.add_method("slice", |lua, this, name: String| {
            match this.0.slice(&name) {
                Some(slice) => slice_to_lua(lua, &slice),
                None => Err(LuaError::RuntimeError(format!("Unknown boundary: {}", name))),
            }
        });

        methods.add_method("org_solution", |lua, this, ()| {
            cases_to_lua(lua, &this.0.org_solution())
        });

        methods.add_method("organization", |_, this, ()| Ok(this.0.organization().to_string()));
        methods.add_method("solution", |_, this, ()| Ok(this.0.solution().to_string()));
    }
}

/// Wrapper for ModelBuilder exposed as Lua userdata.
struct LuaModelBuilder(std::cell::RefCell<ModelBuilder>);

impl UserData for LuaModelBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("set_organization", |_, this, org: String| {
            this.0.borrow_mut().set_organization(org);
            Ok(())
        });

        methods.add_method("set_solution", |_, this, sol: String| {
            this.0.borrow_mut().set_solution(sol);
            Ok(())
        });

        methods.add_method("set_description", |_, this, desc: String| {
            this.0.borrow_mut().set_description(desc);
            Ok(())
        });

        methods.add_method("add_entity", |_, this, name: String| {
            this.0.borrow_mut().add_entity(name);
            Ok(())
        });

        // add_field(entity, field_name, type_string)
        methods.add_method("add_field", |_, this, (entity, field_name, field_type): (String, String, String)| {
            this.0.borrow_mut().add_simple_field(&entity, field_name, field_type);
            Ok(())
        });

        // add_relation(entity, field_name, target_entity, relation, required)
        methods.add_method("add_relation", |_, this, (entity, field_name, target, relation, required): (String, String, String, String, bool)| {
            this.0.borrow_mut().add_relation_field(&entity, field_name, target, relation, required);
            Ok(())
        });

        // add_boundary(name, type, owns_list)
        methods.add_method("add_boundary", |_, this, (name, btype, owns): (String, String, Vec<String>)| {
            this.0.borrow_mut().add_boundary(name, btype, owns);
            Ok(())
        });

        // add_interface(from, to, style)
        methods.add_method("add_interface", |_, this, (from, to, style): (String, String, String)| {
            this.0.borrow_mut().add_interface(from, to, style);
            Ok(())
        });

        // build() → LuaModel
        methods.add_method("build", |_, this, ()| {
            let builder = this.0.replace(ModelBuilder::new());
            Ok(LuaModel(builder.build()))
        });
    }
}

fn create_model_module(lua: &Lua) -> LuaResult<Table> {
    let module = lua.create_table()?;

    // model.load(path) → LuaModel
    module.set(
        "load",
        lua.create_function(|_, path: String| {
            let model = archetect_aml::load_file(std::path::Path::new(&path))
                .map_err(|e| LuaError::RuntimeError(format!("AML load error: {}", e)))?;
            Ok(LuaModel(model))
        })?,
    )?;

    // model.parse(yaml_string) → LuaModel
    module.set(
        "parse",
        lua.create_function(|_, yaml: String| {
            let model = archetect_aml::parse_yaml(&yaml)
                .map_err(|e| LuaError::RuntimeError(format!("AML parse error: {}", e)))?;
            Ok(LuaModel(model))
        })?,
    )?;

    // model.builder() → LuaModelBuilder
    module.set(
        "builder",
        lua.create_function(|_, ()| {
            Ok(LuaModelBuilder(std::cell::RefCell::new(ModelBuilder::new())))
        })?,
    )?;

    // model.from_context(context) → LuaModel
    // Reads model from pre-supplied answers:
    //   - "model_path" answer → load from file
    //   - "model_yaml" answer → parse YAML string
    module.set(
        "from_context",
        lua.create_function(|lua, context_ud: mlua::AnyUserData| {
            use super::context::Context;
            let context = context_ud.borrow::<Context>()?;

            // Try model_path first
            if let Some(path) = get_context_string(&context, lua, "model_path")? {
                let model = archetect_aml::load_file(std::path::Path::new(&path))
                    .map_err(|e| LuaError::RuntimeError(format!("AML load error: {}", e)))?;
                return Ok(LuaModel(model));
            }

            // Try model_yaml
            if let Some(yaml) = get_context_string(&context, lua, "model_yaml")? {
                let model = archetect_aml::parse_yaml(&yaml)
                    .map_err(|e| LuaError::RuntimeError(format!("AML parse error: {}", e)))?;
                return Ok(LuaModel(model));
            }

            Err(LuaError::RuntimeError(
                "model.from_context: no 'model_path' or 'model_yaml' found in context answers".to_string()
            ))
        })?,
    )?;

    Ok(module)
}

/// Helper to get a string value from Context data.
fn get_context_string(
    context: &super::context::Context,
    lua: &Lua,
    key: &str,
) -> LuaResult<Option<String>> {
    let table = context.to_lua_table(lua)?;
    match table.get::<mlua::Value>(key)? {
        mlua::Value::String(s) => Ok(Some(s.to_string_lossy().to_string())),
        mlua::Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

// ── Lua conversion helpers ──────────────────────────────────────

fn cases_to_lua(lua: &Lua, cv: &CaseVariants) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("raw", cv.raw.as_str())?;
    table.set("snake", cv.snake.as_str())?;
    table.set("pascal", cv.pascal.as_str())?;
    table.set("camel", cv.camel.as_str())?;
    table.set("kebab", cv.kebab.as_str())?;
    table.set("train", cv.train.as_str())?;
    table.set("constant", cv.constant.as_str())?;
    table.set("title", cv.title.as_str())?;
    Ok(mlua::Value::Table(table))
}

fn expanded_field_to_lua(lua: &Lua, f: &ExpandedField) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("name", cases_to_lua(lua, &f.name)?)?;
    if let Some(ref t) = f.field_type {
        table.set("type", t.as_str())?;
    }
    table.set("required", f.required)?;
    table.set("unique", f.unique)?;
    table.set("key", f.key)?;
    if let Some(ref d) = f.default {
        table.set("default", d.as_str())?;
    }
    table.set("is_relation", f.is_relation)?;
    if let Some(ref r) = f.relation {
        table.set("relation", r.as_str())?;
    }
    if let Some(ref te) = f.target_entity {
        table.set("target_entity", te.as_str())?;
    }
    if let Some(ref t) = f.target {
        table.set("target", cases_to_lua(lua, t)?)?;
    }
    Ok(mlua::Value::Table(table))
}

fn expanded_entity_to_lua(lua: &Lua, e: &ExpandedEntity) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("name", cases_to_lua(lua, &e.name)?)?;

    let fields = lua.create_table()?;
    for (i, f) in e.fields.iter().enumerate() {
        fields.set(i + 1, expanded_field_to_lua(lua, f)?)?;
    }
    table.set("fields", fields)?;

    let local_fields = lua.create_table()?;
    for (i, f) in e.local_fields.iter().enumerate() {
        local_fields.set(i + 1, expanded_field_to_lua(lua, f)?)?;
    }
    table.set("local_fields", local_fields)?;

    let relations = lua.create_table()?;
    for (i, f) in e.relations.iter().enumerate() {
        relations.set(i + 1, expanded_field_to_lua(lua, f)?)?;
    }
    table.set("relations", relations)?;

    let events = lua.create_table()?;
    for (i, ev) in e.events.iter().enumerate() {
        events.set(i + 1, ev.as_str())?;
    }
    table.set("events", events)?;

    let ops = lua.create_table()?;
    for (i, op) in e.operations.iter().enumerate() {
        ops.set(i + 1, op.as_str())?;
    }
    table.set("operations", ops)?;

    Ok(mlua::Value::Table(table))
}

fn boundary_to_lua(lua: &Lua, b: &archetect_aml::Boundary) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("name", b.name.as_str())?;
    table.set("type", b.boundary_type.as_str())?;
    let owns = lua.create_table()?;
    for (i, e) in b.owns.iter().enumerate() {
        owns.set(i + 1, e.as_str())?;
    }
    table.set("owns", owns)?;
    if let Some(ref lang) = b.language {
        table.set("language", lang.as_str())?;
    }
    if let Some(ref desc) = b.description {
        table.set("description", desc.as_str())?;
    }
    Ok(mlua::Value::Table(table))
}

fn interfaces_to_lua(lua: &Lua, ifaces: &[archetect_aml::Interface]) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    for (i, iface) in ifaces.iter().enumerate() {
        let it = lua.create_table()?;
        it.set("from", iface.from.as_str())?;
        it.set("to", iface.to.as_str())?;
        it.set("style", iface.style.as_str())?;
        table.set(i + 1, it)?;
    }
    Ok(mlua::Value::Table(table))
}

fn remote_ref_to_lua(lua: &Lua, r: &RemoteReference) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("source_entity", r.source_entity.as_str())?;
    table.set("field_name", r.field_name.as_str())?;
    table.set("target_entity", r.target_entity.as_str())?;
    table.set("target_boundary", r.target_boundary.as_str())?;
    table.set("relation", r.relation.as_str())?;
    Ok(mlua::Value::Table(table))
}

fn slice_to_lua(lua: &Lua, s: &BoundarySlice) -> LuaResult<mlua::Value> {
    let table = lua.create_table()?;
    table.set("organization", s.organization.as_str())?;
    table.set("solution", s.solution.as_str())?;
    table.set("boundary", boundary_to_lua(lua, &s.boundary)?)?;

    let entities = lua.create_table()?;
    for (i, e) in s.entities.iter().enumerate() {
        entities.set(i + 1, expanded_entity_to_lua(lua, e)?)?;
    }
    table.set("entities", entities)?;

    let refs = lua.create_table()?;
    for (i, r) in s.remote_references.iter().enumerate() {
        refs.set(i + 1, remote_ref_to_lua(lua, r)?)?;
    }
    table.set("remote_references", refs)?;

    table.set("outbound", interfaces_to_lua(lua, &s.outbound)?)?;
    table.set("inbound", interfaces_to_lua(lua, &s.inbound)?)?;

    let deps = lua.create_table()?;
    for (i, d) in s.dependencies.iter().enumerate() {
        deps.set(i + 1, d.as_str())?;
    }
    table.set("dependencies", deps)?;

    Ok(mlua::Value::Table(table))
}
