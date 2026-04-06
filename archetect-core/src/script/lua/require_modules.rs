use std::env;
use std::process::Command;

use camino::Utf8PathBuf;
use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, UserData, UserDataMethods};

use archetect_api::ScriptMessage;

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

    Ok(())
}

// --- shell module ---

fn create_shell_module(lua: &Lua, _archetect: &Archetect, render_context: &RenderContext) -> LuaResult<Table> {
    let module = lua.create_table()?;
    let default_cwd = render_context.destination().to_string();

    let cwd = default_cwd.clone();
    module.set(
        "run",
        lua.create_function(move |_, (program, args, opts): (String, Option<Vec<String>>, Option<Table>)| {
            let cwd_override = opts.as_ref().and_then(|o| o.get::<String>("cwd".to_string()).ok());
            let working_dir = cwd_override.unwrap_or_else(|| cwd.clone());

            let mut cmd = Command::new(&program);
            if let Some(args) = args {
                cmd.args(&args);
            }
            cmd.current_dir(&working_dir);

            let status = cmd.status().map_err(|e| {
                LuaError::RuntimeError(format!("Failed to run '{}': {}", program, e))
            })?;

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

    let cwd = default_cwd.clone();
    module.set(
        "capture",
        lua.create_function(move |_, (program, args, opts): (String, Option<Vec<String>>, Option<Table>)| {
            let cwd_override = opts.as_ref().and_then(|o| o.get::<String>("cwd".to_string()).ok());
            let working_dir = cwd_override.unwrap_or_else(|| cwd.clone());

            let mut cmd = Command::new(&program);
            if let Some(args) = args {
                cmd.args(&args);
            }
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

// --- git module ---

#[derive(Clone, Debug)]
struct GitRepo {
    path: String,
}

impl UserData for GitRepo {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |_, this, pattern: String| {
            git_cmd(&this.path, &["add", &pattern])
        });

        methods.add_method("add_all", |_, this, ()| {
            git_cmd(&this.path, &["add", "-A"])
        });

        methods.add_method("commit", |_, this, message: String| {
            git_cmd(&this.path, &["commit", "-m", &message])
        });

        methods.add_method("branch", |_, this, name: String| {
            git_cmd(&this.path, &["branch", &name])
        });

        methods.add_method("checkout", |_, this, name: String| {
            git_cmd(&this.path, &["checkout", &name])
        });

        methods.add_method("remote_add", |_, this, (name, url): (String, String)| {
            git_cmd(&this.path, &["remote", "add", &name, &url])
        });

        methods.add_method("push", |_, this, (remote, branch): (String, String)| {
            git_cmd(&this.path, &["push", &remote, &branch])
        });
    }
}

fn git_cmd(path: &str, args: &[&str]) -> LuaResult<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(path)
        .status()
        .map_err(|e| LuaError::RuntimeError(format!("git error: {}", e)))?;

    if !status.success() {
        return Err(LuaError::RuntimeError(format!(
            "git {} failed with status {}",
            args.join(" "),
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

fn create_git_module(lua: &Lua, _archetect: &Archetect, render_context: &RenderContext) -> LuaResult<Table> {
    let module = lua.create_table()?;
    let default_dest = render_context.destination().to_string();

    module.set(
        "init",
        lua.create_function(move |_, (path, opts): (Option<String>, Option<Table>)| {
            let repo_path = match path {
                Some(p) => {
                    let full = format!("{}/{}", default_dest, p);
                    full
                }
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

            let status = Command::new("git")
                .args(&args)
                .status()
                .map_err(|e| LuaError::RuntimeError(format!("git init error: {}", e)))?;

            if !status.success() {
                return Err(LuaError::RuntimeError("git init failed".to_string()));
            }

            Ok(GitRepo { path: repo_path })
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
            crate::script::rhai::modules::archive_module::create_zip_archive(&source_path, &dest_path)
                .map_err(|e| LuaError::RuntimeError(format!("zip error: {}", e)))
        })?,
    )?;

    let d = dest.clone();
    module.set(
        "tar_gz",
        lua.create_function(move |_, (source, destination): (String, String)| {
            let source_path = Utf8PathBuf::from(format!("{}/{}", d, source));
            let dest_path = Utf8PathBuf::from(format!("{}/{}", d, destination));
            crate::script::rhai::modules::archive_module::create_tar_archive(&source_path, &dest_path, true)
                .map_err(|e| LuaError::RuntimeError(format!("tar_gz error: {}", e)))
        })?,
    )?;

    let d = dest.clone();
    module.set(
        "tar",
        lua.create_function(move |_, (source, destination): (String, String)| {
            let source_path = Utf8PathBuf::from(format!("{}/{}", d, source));
            let dest_path = Utf8PathBuf::from(format!("{}/{}", d, destination));
            crate::script::rhai::modules::archive_module::create_tar_archive(&source_path, &dest_path, false)
                .map_err(|e| LuaError::RuntimeError(format!("tar error: {}", e)))
        })?,
    )?;

    Ok(module)
}
