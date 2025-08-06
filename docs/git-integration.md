# Git Integration in Archetect

Archetect provides comprehensive Git functionality through a set of Rhai scripting functions that allow you to initialize repositories, add files, commit changes, manage branches, and push to remote repositories.

## Overview

All Git functions are prefixed with `git_` and support multiple overloads for flexibility. Functions that operate on a repository path can accept:
- No path (operates on current directory)
- A string path
- A Path object

## Available Functions

### Repository Management

#### `git_init([path], [branch_name])`

Initializes a new Git repository. By default uses "main" as the default branch, but you can specify a custom branch name.

**Overloads:**
```rhai
git_init()                      // Initialize in current directory with "main" branch
git_init("path")                // Initialize at string path with "main" branch
git_init(Path("path"))          // Initialize at Path object with "main" branch
git_init("branch-name")         // Initialize in current directory with custom branch
git_init("path", "branch-name") // Initialize at string path with custom branch
git_init(Path("path"), "branch-name") // Initialize at Path object with custom branch
```

**Example:**
```rhai
// Initialize repository with default "main" branch
git_init();

// Initialize with custom branch name
git_init("master");        // Use traditional "master" branch
git_init("develop");        // Use "develop" as default branch
git_init("trunk");          // Use "trunk" as default branch

// Initialize in subdirectory with custom branch
git_init("my-project", "main");

// Use organization's standard branch name
let default_branch = prompt("Default branch name (main/master/develop):");
git_init(default_branch);
```

**Note:** The default branch is set to "main" to follow modern Git conventions, but you can override it to match your organization's standards.

### Staging Files

#### `git_add([path], pattern)`

Adds files matching a pattern to the Git staging area.

**Overloads:**
```rhai
git_add("*.rs")                    // Add files matching pattern in current directory
git_add("src", "*.rs")             // Add files matching pattern in specific path
git_add(Path("src"), "*.rs")       // Add files matching pattern in Path object
```

**Example:**
```rhai
// Add all Rust files
git_add("*.rs");

// Add all files in src directory
git_add("src", "*");

// Add specific file
git_add("README.md");
```

#### `git_add_all([path])`

Adds all files to the Git staging area.

**Overloads:**
```rhai
git_add_all()              // Add all files in current directory
git_add_all("path")        // Add all files at string path
git_add_all(Path("path"))  // Add all files at Path object
```

**Example:**
```rhai
// Stage all changes
git_add_all();

// Stage all changes in subdirectory
git_add_all("docs");
```

### Committing Changes

#### `git_commit([path], message)`

Creates a commit with the staged changes.

**Overloads:**
```rhai
git_commit("message")              // Commit in current directory
git_commit("path", "message")      // Commit at string path
git_commit(Path("path"), "message") // Commit at Path object
```

**Example:**
```rhai
// Simple commit
git_commit("Initial commit");

// Commit with dynamic message
let version = prompt("Version number:");
git_commit("Release version " + version);
```

**Note:** Git signature is determined by:
1. `GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` environment variables
2. `GIT_COMMITTER_NAME` / `GIT_COMMITTER_EMAIL` environment variables
3. Defaults to "Archetect" / "archetect@example.com"

### Branch Management

#### `git_branch([path], branch_name)`

Creates a new branch at the current HEAD.

**Overloads:**
```rhai
git_branch("branch-name")              // Create branch in current directory
git_branch("path", "branch-name")      // Create branch at string path
git_branch(Path("path"), "branch-name") // Create branch at Path object
```

**Example:**
```rhai
// Create feature branch
git_branch("feature/new-feature");

// Create version branch
let version = prompt("Version:");
git_branch("release/v" + version);
```

#### `git_checkout([path], branch_name)`

Switches to an existing branch.

**Overloads:**
```rhai
git_checkout("branch-name")              // Checkout branch in current directory
git_checkout("path", "branch-name")      // Checkout branch at string path
git_checkout(Path("path"), "branch-name") // Checkout branch at Path object
```

**Example:**
```rhai
// Switch to development branch
git_checkout("development");

// Create and switch to new branch
git_branch("feature/auth");
git_checkout("feature/auth");
```

### Remote Repository Operations

#### `git_remote_add([path], name, url)`

Adds a remote repository.

**Overloads:**
```rhai
git_remote_add("name", "url")              // Add remote in current directory
git_remote_add("path", "name", "url")      // Add remote at string path
git_remote_add(Path("path"), "name", "url") // Add remote at Path object
```

**Example:**
```rhai
// Add GitHub origin
git_remote_add("origin", "https://github.com/user/repo.git");

// Add multiple remotes
git_remote_add("upstream", "https://github.com/original/repo.git");
git_remote_add("backup", "git@backup-server:user/repo.git");
```

#### `git_push([path], [remote], [branch])`

Pushes commits to a remote repository.

**Overloads:**
```rhai
git_push()                          // Push to origin/main
git_push("remote", "branch")        // Push to specific remote/branch
git_push("path", "remote", "branch") // Push from specific path
git_push(Path("path"), "remote", "branch") // Push from Path object
```

**Example:**
```rhai
// Push to default origin/main
git_push();

// Push to specific branch
git_push("origin", "development");

// Push feature branch
let feature = prompt("Feature name:");
git_push("origin", "feature/" + feature);
```

**Authentication:** The `git_push` function uses the system `git` executable, which means it leverages your existing Git configuration and credential helpers for authentication. This supports:
- SSH keys
- GitHub personal access tokens
- Credential managers
- Git credential helpers

## Complete Workflow Example

```rhai
// Get project details
let project_name = prompt("Project name:");
let description = prompt("Project description:");
let github_user = prompt("GitHub username:");

// Create project structure
directory(project_name);
render("README.md", "{{ project_name }}/README.md");
render("Cargo.toml", "{{ project_name }}/Cargo.toml");
render("src/main.rs", "{{ project_name }}/src/main.rs");

// Initialize Git
let project_path = Path(project_name);
git_init(project_path);

// Create .gitignore
render("gitignore", project_name + "/.gitignore");

// Initial commit
git_add_all(project_path);
git_commit(project_path, "Initial commit: " + description);

// Create development branch
git_branch(project_path, "development");

// Set up GitHub repository
let repo_name = github_user + "/" + project_name;
if !gh_repo_exists(repo_name) {
    if gh_repo_create(repo_name) {
        display("Created GitHub repository: " + repo_name);
        
        // Add remote and push
        let repo_url = "https://github.com/" + repo_name + ".git";
        git_remote_add(project_path, "origin", repo_url);
        git_push(project_path, "origin", "main");
        git_push(project_path, "origin", "development");
        
        display("✓ Project pushed to GitHub!");
    }
}

display("Project setup complete!");
```

## Error Handling

All Git functions will return errors in the following cases:

1. **Repository not initialized**: Attempting operations on a non-Git directory
2. **Invalid paths**: Path manipulation attempts outside the destination directory
3. **Git operation failures**: Conflicts, authentication issues, etc.
4. **Missing dependencies**: Git executable not found (for push operations)

Example error handling:
```rhai
// The script will abort with an error message if Git operations fail
// You can check for repository existence before operations:
if !file_exists(".git") {
    git_init();
}

// Ensure files exist before committing
if file_exists("README.md") {
    git_add("README.md");
    git_commit("Add README");
}
```

## Best Practices

1. **Always initialize first**: Call `git_init()` before other Git operations
2. **Stage before committing**: Use `git_add()` or `git_add_all()` before `git_commit()`
3. **Check branch existence**: Create branches before checking them out
4. **Use meaningful commit messages**: Include context in your commit messages
5. **Handle authentication**: Ensure Git credentials are configured for push operations

## Integration with GitHub Functions

The Git functions work seamlessly with the GitHub functions:

```rhai
// Complete GitHub integration example
let repo = prompt("Enter repo (owner/name):");

// Create GitHub repository if needed
if !gh_repo_exists(repo) {
    gh_repo_create(repo);
}

// Initialize local repository
git_init();
git_add_all();
git_commit("Initial commit");

// Connect to GitHub
let url = "https://github.com/" + repo + ".git";
git_remote_add("origin", url);
git_push("origin", "main");
```

## Limitations

1. **No merge operations**: Currently doesn't support merging branches
2. **No pull operations**: Fetching from remotes is not yet implemented
3. **Basic push only**: No support for force push or push options
4. **No tag support**: Creating and pushing tags is not implemented
5. **No stash operations**: Git stash functionality is not available

These limitations may be addressed in future versions of Archetect.