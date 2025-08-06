// Example Rhai script demonstrating GitHub functionality
// Requires GITHUB_TOKEN environment variable to be set

// Example 1: Check if a repository exists
let repo_name = "rust-lang/rust";
display("Checking if repository '" + repo_name + "' exists...");

let exists = gh_repo_exists(repo_name);
if exists {
    display("✓ Repository exists!");
} else {
    display("✗ Repository does not exist.");
}

display("");

// Example 2: Create a new repository with default visibility (public)
// Note: This will create a repository under your GitHub account
/*
let new_repo = prompt("Enter repository name to create (format: username/repo):");

if new_repo != "" {
    display("Attempting to create repository '" + new_repo + "'...");
    
    let created = gh_repo_create(new_repo);
    if created {
        display("✓ Repository created successfully!");
    } else {
        display("✗ Repository was not created (may already exist).");
    }
}
*/

// Example 2b: Create a repository with specific visibility
// Available visibility options: Public, Private, Internal
/*
let new_repo = prompt("Enter repository name to create (format: username/repo):");

if new_repo != "" {
    display("Attempting to create private repository '" + new_repo + "'...");
    
    let created = gh_repo_create(new_repo, Private);
    if created {
        display("✓ Private repository created successfully!");
    } else {
        display("✗ Repository was not created (may already exist).");
    }
}
*/

// Example 3: Dynamic repository creation based on project name
let prefix = prompt("Enter project prefix:");
let suffix = prompt("Enter project suffix:");
let github_user = prompt("Enter your GitHub username:");

if prefix != "" && suffix != "" && github_user != "" {
    let project_name = prefix + "-" + suffix;
    let repo_full_name = github_user + "/" + project_name;
    
    display("Checking GitHub repository: " + repo_full_name);
    
    if !gh_repo_exists(repo_full_name) {
        let create_repo = prompt_bool("Repository doesn't exist. Create it?");
        
        if create_repo {
            // Example of prompting for visibility
            let visibility_choice = prompt_select("Repository visibility:", ["Public", "Private"]);
            let visibility = if visibility_choice == "Private" { Private } else { Public };
            
            let created = gh_repo_create(repo_full_name, visibility);
            if created {
                display("✓ " + visibility_choice + " repository created: https://github.com/" + repo_full_name);
            } else {
                display("✗ Failed to create repository.");
            }
        }
    } else {
        display("Repository already exists: https://github.com/" + repo_full_name);
    }
}

display("");
display("GitHub operations completed.");