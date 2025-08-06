use crate::Archetect;
use archetect_api::CommandRequest;
use octocrab::Octocrab;
use rhai::plugin::*;
use rhai::{Engine, EvalAltResult};
use std::env;
use http_body_util::BodyExt;

#[derive(Clone, Debug)]
pub enum RepoVisibility {
    Public,
    Private,
    Internal,
}

#[allow(non_upper_case_globals)]
#[export_module]
pub mod visibility_module {
    pub type RepoVisibility = crate::script::rhai::modules::github_module::RepoVisibility;
    pub const Public: RepoVisibility = RepoVisibility::Public;
    pub const Private: RepoVisibility = RepoVisibility::Private;
    pub const Internal: RepoVisibility = RepoVisibility::Internal;
    
    pub const PUBLIC: RepoVisibility = RepoVisibility::Public;
    pub const PRIVATE: RepoVisibility = RepoVisibility::Private;
    pub const INTERNAL: RepoVisibility = RepoVisibility::Internal;
}

pub(crate) fn register(engine: &mut Engine, archetect: Archetect) {
    engine.register_global_module(exported_module!(visibility_module).into());
    engine.register_fn("gh_repo_exists", move |repo: &str| -> Result<bool, Box<EvalAltResult>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        
        runtime.block_on(async {
            let token = env::var("GITHUB_TOKEN")
                .map_err(|_| "GITHUB_TOKEN environment variable not found. Please set GITHUB_TOKEN to authenticate with GitHub.")?;
            
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|e| format!("Failed to create GitHub client: {}", e))?;
            
            let parts: Vec<&str> = repo.split('/').collect();
            if parts.len() != 2 {
                return Err("Repository must be in the format 'owner/repo'".into());
            }
            
            let owner = parts[0];
            let repo_name = parts[1];
            
            match octocrab.repos(owner, repo_name).get().await {
                Ok(_) => Ok(true),
                Err(octocrab::Error::GitHub { source, .. }) if source.message == "Not Found" => Ok(false),
                Err(e) => Err(format!("Failed to check repository existence: {}", e).into()),
            }
        })
    });

    let archetect_clone = archetect.clone();
    engine.register_fn("gh_repo_create", move |repo: &str| -> Result<bool, Box<EvalAltResult>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        
        runtime.block_on(async {
            let token = env::var("GITHUB_TOKEN")
                .map_err(|_| "GITHUB_TOKEN environment variable not found. Please set GITHUB_TOKEN to authenticate with GitHub.")?;
            
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|e| format!("Failed to create GitHub client: {}", e))?;
            
            let parts: Vec<&str> = repo.split('/').collect();
            if parts.len() != 2 {
                return Err("Repository must be in the format 'owner/repo'".into());
            }
            
            let owner = parts[0];
            let repo_name = parts[1];
            
            // Check if repo already exists
            match octocrab.repos(owner, repo_name).get().await {
                Ok(_) => {
                    archetect_clone.request(CommandRequest::LogWarn(
                        format!("Repository '{}/{}' already exists", owner, repo_name)
                    ));
                    Ok(false)
                },
                Err(octocrab::Error::GitHub { source, .. }) if source.message == "Not Found" => {
                    // Repository doesn't exist, try to create it
                    let current_user = octocrab.current()
                        .user()
                        .await
                        .map_err(|e| format!("Failed to get current user: {}", e))?;
                    
                    // Create the repository
                    // We need to use the POST /user/repos or POST /orgs/{org}/repos endpoint
                    
                    // Create a JSON body for the request
                    let body = serde_json::json!({
                        "name": repo_name,
                        "private": false,
                        "auto_init": false
                    });
                    
                    // Determine the API endpoint based on owner
                    let (endpoint, display_owner) = if current_user.login == owner {
                        // Creating under the authenticated user
                        ("/user/repos".to_string(), owner.to_string())
                    } else {
                        // Creating under an organization
                        // First check if the user has access to the organization
                        let org_endpoint = format!("/orgs/{}", owner);
                        match octocrab._get(&org_endpoint).await {
                            Ok(_) => {
                                // User has access to the organization
                                (format!("/orgs/{}/repos", owner), owner.to_string())
                            },
                            Err(e) => {
                                return Err(format!(
                                    "Cannot create repository under '{}'. Either the organization doesn't exist or you don't have permission to create repositories in it. Error: {}", 
                                    owner, e
                                ).into());
                            }
                        }
                    };
                    
                    let response = octocrab
                        ._post(&endpoint, Some(&body))
                        .await
                        .map_err(|e| format!("Failed to create repository: {}", e))?;
                    
                    // Parse the response body
                    let body_bytes = response.into_body()
                        .collect()
                        .await
                        .map_err(|e| format!("Failed to read response body: {}", e))?
                        .to_bytes();
                    
                    let repo_data: serde_json::Value = serde_json::from_slice(&body_bytes)
                        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
                    
                    if repo_data.get("id").is_some() {
                        archetect_clone.request(CommandRequest::LogInfo(
                            format!("Created repository '{}/{}'", display_owner, repo_name)
                        ));
                        Ok(true)
                    } else {
                        Err("Failed to create repository: unexpected response format".into())
                    }
                },
                Err(e) => Err(format!("Failed to check repository existence: {}", e).into()),
            }
        })
    });

    let archetect_clone2 = archetect.clone();
    engine.register_fn("gh_repo_create", move |repo: &str, visibility: RepoVisibility| -> Result<bool, Box<EvalAltResult>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;
        
        runtime.block_on(async {
            let token = env::var("GITHUB_TOKEN")
                .map_err(|_| "GITHUB_TOKEN environment variable not found. Please set GITHUB_TOKEN to authenticate with GitHub.")?;
            
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|e| format!("Failed to create GitHub client: {}", e))?;
            
            let parts: Vec<&str> = repo.split('/').collect();
            if parts.len() != 2 {
                return Err("Repository must be in the format 'owner/repo'".into());
            }
            
            let owner = parts[0];
            let repo_name = parts[1];
            
            // Check if repo already exists
            match octocrab.repos(owner, repo_name).get().await {
                Ok(_) => {
                    archetect_clone2.request(CommandRequest::LogWarn(
                        format!("Repository '{}/{}' already exists", owner, repo_name)
                    ));
                    Ok(false)
                },
                Err(octocrab::Error::GitHub { source, .. }) if source.message == "Not Found" => {
                    // Repository doesn't exist, try to create it
                    let current_user = octocrab.current()
                        .user()
                        .await
                        .map_err(|e| format!("Failed to get current user: {}", e))?;
                    
                    // Create the repository with specified visibility
                    let visibility_str = match visibility {
                        RepoVisibility::Public => "public",
                        RepoVisibility::Private => "private",
                        RepoVisibility::Internal => "internal",
                    };
                    
                    // Create a JSON body for the request
                    let body = serde_json::json!({
                        "name": repo_name,
                        "private": matches!(visibility, RepoVisibility::Private),
                        "visibility": visibility_str,
                        "auto_init": false
                    });
                    
                    // Determine the API endpoint based on owner
                    let (endpoint, display_owner) = if current_user.login == owner {
                        // Creating under the authenticated user
                        ("/user/repos".to_string(), owner.to_string())
                    } else {
                        // Creating under an organization
                        // First check if the user has access to the organization
                        let org_endpoint = format!("/orgs/{}", owner);
                        match octocrab._get(&org_endpoint).await {
                            Ok(_) => {
                                // User has access to the organization
                                (format!("/orgs/{}/repos", owner), owner.to_string())
                            },
                            Err(e) => {
                                return Err(format!(
                                    "Cannot create repository under '{}'. Either the organization doesn't exist or you don't have permission to create repositories in it. Error: {}", 
                                    owner, e
                                ).into());
                            }
                        }
                    };
                    
                    let response = octocrab
                        ._post(&endpoint, Some(&body))
                        .await
                        .map_err(|e| format!("Failed to create repository: {}", e))?;
                    
                    // Parse the response body
                    let body_bytes = response.into_body()
                        .collect()
                        .await
                        .map_err(|e| format!("Failed to read response body: {}", e))?
                        .to_bytes();
                    
                    let repo_data: serde_json::Value = serde_json::from_slice(&body_bytes)
                        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
                    
                    if repo_data.get("id").is_some() {
                        archetect_clone2.request(CommandRequest::LogInfo(
                            format!("Created {} repository '{}/{}'", visibility_str, display_owner, repo_name)
                        ));
                        Ok(true)
                    } else {
                        Err("Failed to create repository: unexpected response format".into())
                    }
                },
                Err(e) => Err(format!("Failed to check repository existence: {}", e).into()),
            }
        })
    });
}
