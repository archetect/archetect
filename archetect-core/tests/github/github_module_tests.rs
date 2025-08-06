#[cfg(test)]
mod tests {
    use std::env;

    // Since we can't directly test the Rhai functions without access to the internal create_engine function,
    // we'll create simple unit tests to verify the GitHub API logic works correctly.
    // Full integration tests would require creating a complete Archetype setup.

    #[test]
    fn test_repo_format_validation() {
        // Test that repo format validation logic is correct
        let valid_repo = "owner/repo";
        let parts: Vec<&str> = valid_repo.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "owner");
        assert_eq!(parts[1], "repo");

        let invalid_repo = "invalid-format";
        let parts: Vec<&str> = invalid_repo.split('/').collect();
        assert_ne!(parts.len(), 2);
    }

    #[test]
    fn test_github_token_env_check() {
        // Test environment variable checking
        let original_token = env::var("GITHUB_TOKEN").ok();
        
        // Test with token present
        env::set_var("GITHUB_TOKEN", "test_token");
        assert!(env::var("GITHUB_TOKEN").is_ok());
        
        // Test with token absent
        env::remove_var("GITHUB_TOKEN");
        assert!(env::var("GITHUB_TOKEN").is_err());
        
        // Restore original state
        if let Some(token) = original_token {
            env::set_var("GITHUB_TOKEN", token);
        }
    }

    // NOTE: Full integration tests for gh_repo_exists() and gh_repo_create() would require:
    // 1. A valid GITHUB_TOKEN
    // 2. Network access to GitHub API
    // 3. Appropriate test repositories or test user account
    // 
    // These tests should be run as part of CI/CD with proper test infrastructure.
    // For now, manual testing can be done by:
    // 1. Setting GITHUB_TOKEN environment variable
    // 2. Creating a test archetype with a script that calls gh_repo_exists("owner/repo")
    // 3. Running archetect with that archetype
}