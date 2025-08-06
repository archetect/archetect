# GitHub Integration in Archetect

Archetect now includes built-in GitHub functionality through two Rhai scripting functions that allow you to check if repositories exist and create new ones.

## Prerequisites

- A GitHub personal access token must be available in the `GITHUB_TOKEN` environment variable
- The token must have appropriate permissions:
  - `repo` scope for private repositories
  - `public_repo` scope for public repositories only

## Available Functions

### `gh_repo_exists(repo: string) -> bool`

Checks if a GitHub repository exists.

**Parameters:**
- `repo`: Repository name in the format `owner/repo`

**Returns:**
- `true` if the repository exists
- `false` if the repository does not exist

**Errors:**
- Missing `GITHUB_TOKEN` environment variable
- Invalid repository format (must be `owner/repo`)
- Network or API errors

**Example:**
```rhai
if gh_repo_exists("rust-lang/rust") {
    display("Repository exists!");
} else {
    display("Repository not found.");
}
```

### `gh_repo_create(repo: string) -> bool`

Creates a new GitHub repository with default public visibility.

**Parameters:**
- `repo`: Repository name in the format `owner/repo`

**Returns:**
- `true` if the repository was successfully created
- `false` if the repository already exists

**Errors:**
- Missing `GITHUB_TOKEN` environment variable
- Invalid repository format (must be `owner/repo`)
- Insufficient permissions for organization repositories
- Network or API errors

**Example:**
```rhai
let repo_name = "myuser/my-new-project";

if !gh_repo_exists(repo_name) {
    if gh_repo_create(repo_name) {
        display("Repository created successfully!");
    } else {
        display("Failed to create repository.");
    }
} else {
    display("Repository already exists.");
}
```

### `gh_repo_create(repo: string, visibility: RepoVisibility) -> bool`

Creates a new GitHub repository with specified visibility.

**Parameters:**
- `repo`: Repository name in the format `owner/repo`
- `visibility`: Repository visibility (`Public`, `Private`, or `Internal`)

**Visibility Options:**
- `Public`: Repository is visible to everyone
- `Private`: Repository is only visible to you and collaborators
- `Internal`: Repository is visible to all members of the organization (enterprise feature)

**Returns:**
- `true` if the repository was successfully created
- `false` if the repository already exists

**Example:**
```rhai
// Create a private repository
let repo_name = "myuser/my-private-project";
if gh_repo_create(repo_name, Private) {
    display("Private repository created!");
}

// Prompt user for visibility preference
let visibility_choice = prompt_select("Repository visibility:", ["Public", "Private"]);
let visibility = if visibility_choice == "Private" { Private } else { Public };

if gh_repo_create("myuser/my-project", visibility) {
    display(visibility_choice + " repository created successfully!");
}
```

## Usage in Archetypes

These functions are particularly useful in project generation archetypes where you want to:

1. Check if a repository already exists before creating a project
2. Automatically create a GitHub repository for newly generated projects
3. Integrate with CI/CD workflows

### Example: Dynamic Repository Creation

```rhai
// Get project details from user
let project_name = prompt("Enter project name:");
let github_user = prompt("Enter your GitHub username:");
let repo_name = github_user + "/" + project_name;

// Check and create repository
if !gh_repo_exists(repo_name) {
    display("Repository doesn't exist. Creating...");
    
    if gh_repo_create(repo_name) {
        display("✓ Created repository: https://github.com/" + repo_name);
        
        // Continue with project generation
        directory("./{{ project_name }}");
        // ... rest of archetype logic
    } else {
        display("✗ Failed to create repository");
        abort("Cannot proceed without repository");
    }
} else {
    display("Repository already exists!");
    let proceed = prompt_bool("Continue with project generation?");
    
    if !proceed {
        abort("User cancelled");
    }
}
```

## Current Limitations

1. **Limited Repository Options**: New repositories are created with minimal settings:
   - No customization of description, license, gitignore templates, etc.
   - Repositories are created empty (no auto-initialized README)

2. **Synchronous Operations**: GitHub API calls are made synchronously, which may cause delays in script execution.

3. **Organization Permissions**: Creating repositories in organizations requires appropriate permissions. The token must have the necessary access rights to create repositories in the target organization.

## Security Considerations

- Never hardcode your GitHub token in scripts
- Use environment variables or secure credential storage
- Ensure your token has only the necessary permissions
- Consider using fine-grained personal access tokens for better security

## Troubleshooting

### GITHUB_TOKEN not found
```
Error: GITHUB_TOKEN environment variable not found. Please set GITHUB_TOKEN to authenticate with GitHub.
```
Solution: Set the environment variable:
```bash
export GITHUB_TOKEN="your-personal-access-token"
```

### Invalid repository format
```
Error: Repository must be in the format 'owner/repo'
```
Solution: Ensure the repository name includes both owner and repository name separated by a forward slash.

### Cannot create repository in organization
```
Error: Cannot create repository under 'org-name'. Either the organization doesn't exist or you don't have permission to create repositories in it.
```
Solution: Ensure you have the necessary permissions to create repositories in the target organization. Your GitHub token must have appropriate organization access.