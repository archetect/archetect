use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::path_module::Path;
use crate::utils::restrict_path_manipulation;
use crate::Archetect;
use archetect_api::CommandRequest;
use camino::Utf8PathBuf;
use git2::{BranchType, IndexAddOption, Repository, Signature};
use log::info;
use rhai::{Engine, EvalAltResult, NativeCallContext};
use std::env;

pub(crate) fn register(engine: &mut Engine, archetect: Archetect, render_context: RenderContext) {
    // git_init - Initialize a new Git repository
    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_init", move |call: NativeCallContext| {
        git_init_current_dir(&call, render_context_clone.clone(), archetect_clone.clone())
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_init", move |call: NativeCallContext, path: &str| {
        git_init_dir(&call, render_context_clone.clone(), archetect_clone.clone(), path)
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_init", move |call: NativeCallContext, mut path: Path| {
        git_init_path(&call, render_context_clone.clone(), archetect_clone.clone(), &mut path)
    });

    // git_init with custom branch name
    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_init", move |call: NativeCallContext, branch_name: &str| {
        git_init_with_branch(
            &call,
            render_context_clone.clone(),
            archetect_clone.clone(),
            ".",
            branch_name,
        )
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_init",
        move |call: NativeCallContext, path: &str, branch_name: &str| {
            git_init_with_branch(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                path,
                branch_name,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_init",
        move |call: NativeCallContext, mut path: Path, branch_name: &str| {
            git_init_path_with_branch(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                &mut path,
                branch_name,
            )
        },
    );

    // git_add - Add files to the Git index
    let render_context_clone = render_context.clone();
    engine.register_fn("git_add", move |call: NativeCallContext, pattern: &str| {
        git_add_pattern(&call, render_context_clone.clone(), ".", pattern)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("git_add", move |call: NativeCallContext, path: &str, pattern: &str| {
        git_add_pattern(&call, render_context_clone.clone(), path, pattern)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn(
        "git_add",
        move |call: NativeCallContext, mut path: Path, pattern: &str| {
            git_add_pattern_path(&call, render_context_clone.clone(), &mut path, pattern)
        },
    );

    // git_add_all - Add all files to the Git index
    let render_context_clone = render_context.clone();
    engine.register_fn("git_add_all", move |call: NativeCallContext| {
        git_add_all(&call, render_context_clone.clone(), ".")
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("git_add_all", move |call: NativeCallContext, path: &str| {
        git_add_all(&call, render_context_clone.clone(), path)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("git_add_all", move |call: NativeCallContext, mut path: Path| {
        git_add_all_path(&call, render_context_clone.clone(), &mut path)
    });

    // git_commit - Commit changes
    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_commit", move |call: NativeCallContext, message: &str| {
        git_commit(
            &call,
            render_context_clone.clone(),
            archetect_clone.clone(),
            ".",
            message,
        )
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_commit",
        move |call: NativeCallContext, path: &str, message: &str| {
            git_commit(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                path,
                message,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_commit",
        move |call: NativeCallContext, mut path: Path, message: &str| {
            git_commit_path(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                &mut path,
                message,
            )
        },
    );

    // git_remote_add - Add a remote repository
    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_remote_add",
        move |call: NativeCallContext, name: &str, url: &str| {
            git_remote_add(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                ".",
                name,
                url,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_remote_add",
        move |call: NativeCallContext, path: &str, name: &str, url: &str| {
            git_remote_add(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                path,
                name,
                url,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_remote_add",
        move |call: NativeCallContext, mut path: Path, name: &str, url: &str| {
            git_remote_add_path(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                &mut path,
                name,
                url,
            )
        },
    );

    // git_push - Push to remote repository
    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_push", move |call: NativeCallContext| {
        git_push(
            &call,
            render_context_clone.clone(),
            archetect_clone.clone(),
            ".",
            "origin",
            "main",
        )
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_push", move |call: NativeCallContext, path: &str| {
        git_push(
            &call,
            render_context_clone.clone(),
            archetect_clone.clone(),
            path,
            "origin",
            "main",
        )
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn("git_push", move |call: NativeCallContext, mut path: Path| {
        git_push_path(
            &call,
            render_context_clone.clone(),
            archetect_clone.clone(),
            &mut path,
            "origin",
            "main",
        )
    });

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_push",
        move |call: NativeCallContext, remote: &str, branch: &str| {
            git_push(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                ".",
                remote,
                branch,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_push",
        move |call: NativeCallContext, path: &str, remote: &str, branch: &str| {
            git_push(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                path,
                remote,
                branch,
            )
        },
    );

    let render_context_clone = render_context.clone();
    let archetect_clone = archetect.clone();
    engine.register_fn(
        "git_push",
        move |call: NativeCallContext, mut path: Path, remote: &str, branch: &str| {
            git_push_path(
                &call,
                render_context_clone.clone(),
                archetect_clone.clone(),
                &mut path,
                remote,
                branch,
            )
        },
    );

    // git_branch - Create a new branch
    let render_context_clone = render_context.clone();
    engine.register_fn("git_branch", move |call: NativeCallContext, branch_name: &str| {
        git_branch(&call, render_context_clone.clone(), ".", branch_name)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn(
        "git_branch",
        move |call: NativeCallContext, path: &str, branch_name: &str| {
            git_branch(&call, render_context_clone.clone(), path, branch_name)
        },
    );

    let render_context_clone = render_context.clone();
    engine.register_fn(
        "git_branch",
        move |call: NativeCallContext, mut path: Path, branch_name: &str| {
            git_branch_path(&call, render_context_clone.clone(), &mut path, branch_name)
        },
    );

    // git_checkout - Checkout a branch
    let render_context_clone = render_context.clone();
    engine.register_fn("git_checkout", move |call: NativeCallContext, branch_name: &str| {
        git_checkout(&call, render_context_clone.clone(), ".", branch_name)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn(
        "git_checkout",
        move |call: NativeCallContext, path: &str, branch_name: &str| {
            git_checkout(&call, render_context_clone.clone(), path, branch_name)
        },
    );

    let render_context_clone = render_context.clone();
    engine.register_fn(
        "git_checkout",
        move |call: NativeCallContext, mut path: Path, branch_name: &str| {
            git_checkout_path(&call, render_context_clone.clone(), &mut path, branch_name)
        },
    );
}

// Helper function to get the full path
fn get_full_path(
    call: &NativeCallContext,
    render_context: &RenderContext,
    path: &str,
) -> Result<Utf8PathBuf, Box<EvalAltResult>> {
    let restricted_path = restrict_path_manipulation(call, path)?;
    Ok(render_context.destination().join(restricted_path))
}

// git_init implementations
fn git_init_current_dir(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
) -> Result<(), Box<EvalAltResult>> {
    git_init_dir(call, render_context, archetect, ".")
}

fn git_init_dir(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    info!("Initializing Git repository at: {}", full_path);

    match Repository::init(&full_path) {
        Ok(repo) => {
            // Set the default branch name to "main"
            repo.set_head("refs/heads/main").map_err(|e| {
                let error = ArchetypeScriptError::GitError {
                    message: format!("Failed to set default branch to 'main': {}", e),
                };
                ArchetypeScriptErrorWrapper(call, error)
            })?;

            archetect.request(CommandRequest::LogInfo(format!(
                "Initialized Git repository at: {} with default branch 'main'",
                full_path
            )));
            Ok(())
        }
        Err(e) => {
            let error = ArchetypeScriptError::GitError {
                message: format!("Failed to initialize Git repository: {}", e),
            };
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        }
    }
}

fn git_init_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &mut Path,
) -> Result<(), Box<EvalAltResult>> {
    git_init_dir(call, render_context, archetect, path.path())
}

// git_init with custom branch name implementations
fn git_init_with_branch(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &str,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    info!(
        "Initializing Git repository at: {} with branch '{}'",
        full_path, branch_name
    );

    match Repository::init(&full_path) {
        Ok(repo) => {
            // Set the default branch name to the specified branch
            repo.set_head(&format!("refs/heads/{}", branch_name)).map_err(|e| {
                let error = ArchetypeScriptError::GitError {
                    message: format!("Failed to set default branch to '{}': {}", branch_name, e),
                };
                ArchetypeScriptErrorWrapper(call, error)
            })?;

            archetect.request(CommandRequest::LogInfo(format!(
                "Initialized Git repository at: {} with default branch '{}'",
                full_path, branch_name
            )));
            Ok(())
        }
        Err(e) => {
            let error = ArchetypeScriptError::GitError {
                message: format!("Failed to initialize Git repository: {}", e),
            };
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        }
    }
}

fn git_init_path_with_branch(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &mut Path,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_init_with_branch(call, render_context, archetect, path.path(), branch_name)
}

// git_add implementations
fn git_add_pattern(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &str,
    pattern: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    let repo = Repository::open(&full_path).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to open repository: {}. Did you forget to call git_init()?", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    let mut index = repo.index().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to get repository index: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Add files matching the pattern
    let pathspecs = vec![pattern];
    index.add_all(&pathspecs, IndexAddOption::DEFAULT, None).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to add files matching '{}': {}", pattern, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    index.write().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to write index: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    info!("Added files matching pattern '{}' to Git index", pattern);
    Ok(())
}

fn git_add_pattern_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &mut Path,
    pattern: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_add_pattern(call, render_context, path.path(), pattern)
}

fn git_add_all(call: &NativeCallContext, render_context: RenderContext, path: &str) -> Result<(), Box<EvalAltResult>> {
    git_add_pattern(call, render_context, path, ".")
}

fn git_add_all_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &mut Path,
) -> Result<(), Box<EvalAltResult>> {
    git_add_all(call, render_context, path.path())
}

// git_commit implementations
fn git_commit(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &str,
    message: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    let repo = Repository::open(&full_path).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to open repository: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Get the current index
    let mut index = repo.index().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to get repository index: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Write the index as a tree
    let tree_id = index.write_tree().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to write tree: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    let tree = repo.find_tree(tree_id).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to find tree: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Create signature
    let sig = get_signature(call)?;

    // Get parent commit (if any)
    let parent_commit = match repo.head() {
        Ok(head) => {
            let oid = head.target().ok_or_else(|| {
                let error = ArchetypeScriptError::GitError {
                    message: "Failed to get HEAD target".to_string(),
                };
                ArchetypeScriptErrorWrapper(call, error)
            })?;
            Some(repo.find_commit(oid).map_err(|e| {
                let error = ArchetypeScriptError::GitError {
                    message: format!("Failed to find parent commit: {}", e),
                };
                ArchetypeScriptErrorWrapper(call, error)
            })?)
        }
        Err(_) => None, // First commit
    };

    // Create the commit
    let parents = if let Some(ref parent) = parent_commit {
        vec![parent]
    } else {
        vec![]
    };

    let commit_id = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents[..])
        .map_err(|e| {
            let error = ArchetypeScriptError::GitError {
                message: format!("Failed to create commit: {}", e),
            };
            ArchetypeScriptErrorWrapper(call, error)
        })?;

    archetect.request(CommandRequest::LogDebug(format!(
        "Created commit: {}",
        &commit_id.to_string()[..7]
    )));

    Ok(())
}

fn git_commit_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &mut Path,
    message: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_commit(call, render_context, archetect, path.path(), message)
}

// git_remote_add implementations
fn git_remote_add(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &str,
    name: &str,
    url: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    let repo = Repository::open(&full_path).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to open repository: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    repo.remote(name, url).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to add remote '{}': {}", name, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    archetect.request(CommandRequest::LogInfo(format!("Added remote '{}' -> {}", name, url)));

    Ok(())
}

fn git_remote_add_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &mut Path,
    name: &str,
    url: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_remote_add(call, render_context, archetect, path.path(), name, url)
}

// git_push implementations
fn git_push(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &str,
    remote: &str,
    branch: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    // Use the git executable for better authentication support
    // This leverages the user's existing Git configuration and credential helpers
    use std::process::Command;

    archetect.request(CommandRequest::LogInfo(format!(
        "Pushing to '{}/{}' ...",
        remote, branch
    )));

    let output = Command::new("git")
        .arg("push")
        .arg(remote)
        .arg(branch)
        .current_dir(&full_path)
        .output()
        .map_err(|e| {
            let error = ArchetypeScriptError::GitError {
                message: format!("Failed to execute git push: {}", e),
            };
            ArchetypeScriptErrorWrapper(call, error)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to push to '{}/{}': {}", remote, branch, stderr.trim()),
        };
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    archetect.request(CommandRequest::LogInfo(format!(
        "Successfully pushed to '{}/{}'",
        remote, branch
    )));

    Ok(())
}

fn git_push_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    archetect: Archetect,
    path: &mut Path,
    remote: &str,
    branch: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_push(call, render_context, archetect, path.path(), remote, branch)
}

// git_branch implementations
fn git_branch(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &str,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    let repo = Repository::open(&full_path).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to open repository: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Get the current HEAD commit
    let head = repo.head().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to get HEAD: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    let oid = head.target().ok_or_else(|| {
        let error = ArchetypeScriptError::GitError {
            message: "Failed to get HEAD target".to_string(),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    let commit = repo.find_commit(oid).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to find commit: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Create the branch
    repo.branch(branch_name, &commit, false).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to create branch '{}': {}", branch_name, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    info!("Created branch '{}'", branch_name);
    Ok(())
}

fn git_branch_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &mut Path,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_branch(call, render_context, path.path(), branch_name)
}

// git_checkout implementations
fn git_checkout(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &str,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    let full_path = get_full_path(call, &render_context, path)?;

    let repo = Repository::open(&full_path).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to open repository: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Find the branch
    let branch = repo.find_branch(branch_name, BranchType::Local).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to find branch '{}': {}", branch_name, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Get the commit the branch points to
    let _commit = branch.get().peel_to_commit().map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to get commit for branch '{}': {}", branch_name, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Set HEAD to the branch
    repo.set_head(&format!("refs/heads/{}", branch_name)).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to set HEAD to branch '{}': {}", branch_name, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    // Checkout the tree
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
        .map_err(|e| {
            let error = ArchetypeScriptError::GitError {
                message: format!("Failed to checkout branch '{}': {}", branch_name, e),
            };
            ArchetypeScriptErrorWrapper(call, error)
        })?;

    info!("Checked out branch '{}'", branch_name);
    Ok(())
}

fn git_checkout_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    path: &mut Path,
    branch_name: &str,
) -> Result<(), Box<EvalAltResult>> {
    git_checkout(call, render_context, path.path(), branch_name)
}

// Helper function to get Git signature
fn get_signature(call: &NativeCallContext) -> Result<Signature<'static>, Box<EvalAltResult>> {
    // Try to get from environment or Git config
    let name = env::var("GIT_AUTHOR_NAME")
        .or_else(|_| env::var("GIT_COMMITTER_NAME"))
        .unwrap_or_else(|_| "Archetect".to_string());

    let email = env::var("GIT_AUTHOR_EMAIL")
        .or_else(|_| env::var("GIT_COMMITTER_EMAIL"))
        .unwrap_or_else(|_| "archetect@example.com".to_string());

    Signature::now(&name, &email).map_err(|e| {
        let error = ArchetypeScriptError::GitError {
            message: format!("Failed to create Git signature: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error).into()
    })
}

