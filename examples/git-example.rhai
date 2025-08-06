// Example Rhai script demonstrating Git functionality

// Example 1: Initialize a Git repository in the current directory
display("Initializing Git repository...");
git_init();  // Uses default "main" branch
display("✓ Git repository initialized with 'main' branch");

// Example 1b: Initialize with custom default branch
// Uncomment to use a different default branch:
// git_init("master");     // Traditional master branch
// git_init("develop");    // Use develop as default
// git_init("trunk");      // Use trunk (SVN-style naming)

display("");

// Example 2: Add files and commit
display("Creating some example files...");
render("templates/README.md", "README.md");
render("templates/main.rs", "src/main.rs");

display("Adding files to Git...");
git_add_all();  // Add all files
// Or add specific patterns:
// git_add("*.rs");  // Add all Rust files
// git_add("README.md");  // Add specific file

display("Creating initial commit...");
git_commit("Initial commit: Project scaffolding");
display("✓ Files committed");

display("");

// Example 3: Working with branches
display("Creating development branch...");
git_branch("development");
git_checkout("development");
display("✓ Switched to development branch");

// Make some changes
render("templates/lib.rs", "src/lib.rs");
git_add("src/lib.rs");
git_commit("Add library module");

// Switch back to main
git_checkout("main");
display("✓ Switched back to main branch");

display("");

// Example 4: Dynamic Git operations based on user input
let project_name = prompt("Enter project name:");
let github_user = prompt("Enter your GitHub username:");
let create_repo = prompt_bool("Initialize Git and create GitHub repository?");

if create_repo {
    // Optionally prompt for default branch name
    let default_branch = prompt_select("Default branch name:", ["main", "master", "develop"]);
    
    // Initialize Git with chosen branch name
    git_init(default_branch);
    
    // Add all project files
    git_add_all();
    
    // Create initial commit
    let commit_message = "Initial commit: " + project_name + " project";
    git_commit(commit_message);
    
    // Set up GitHub remote
    let repo_name = github_user + "/" + project_name;
    let repo_url = "https://github.com/" + repo_name + ".git";
    
    // Check if GitHub repo exists, create if not
    if !gh_repo_exists(repo_name) {
        display("Creating GitHub repository...");
        if gh_repo_create(repo_name) {
            display("✓ GitHub repository created");
            
            // Add remote and push
            git_remote_add("origin", repo_url);
            git_push("origin", default_branch);
            display("✓ Code pushed to GitHub");
        } else {
            display("✗ Failed to create GitHub repository");
        }
    } else {
        display("GitHub repository already exists");
        git_remote_add("origin", repo_url);
    }
}

display("");
display("Git operations completed!");

// Example 5: Working with different paths
// You can also specify paths for Git operations:
//
// let subproject = Path("subproject");
// git_init(subproject);
// git_add(subproject, "*.txt");
// git_commit(subproject, "Add text files");
//
// Or with string paths:
// git_init("docs");
// git_add("docs", "*.md");
// git_commit("docs", "Add documentation");