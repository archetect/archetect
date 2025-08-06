#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;
    use git2::Repository;

    #[test]
    fn test_git_operations_flow() {
        // Create a temporary directory
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        
        // Initialize a repository
        let repo = Repository::init(repo_path).expect("Failed to init repository");
        
        // Set default branch to main (mimicking what our git_init does)
        repo.set_head("refs/heads/main").expect("Failed to set HEAD to main");
        
        // Create a test file
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "Hello, Git!").expect("Failed to write test file");
        
        // Add the file
        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("test.txt")).expect("Failed to add file");
        index.write().expect("Failed to write index");
        
        // Create a signature
        let sig = git2::Signature::now("Test User", "test@example.com")
            .expect("Failed to create signature");
        
        // Get the tree
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        
        // Create the first commit
        let commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        ).expect("Failed to create commit");
        
        // Verify the commit was created
        let commit = repo.find_commit(commit_id).expect("Failed to find commit");
        assert_eq!(commit.message(), Some("Initial commit"));
        
        // Verify we're on the main branch
        let head = repo.head().expect("Failed to get HEAD");
        assert_eq!(head.shorthand(), Some("main"));
        
        // Create a branch
        repo.branch("test-branch", &commit, false).expect("Failed to create branch");
        
        // Verify branch exists
        let branch = repo.find_branch("test-branch", git2::BranchType::Local)
            .expect("Failed to find branch");
        assert!(branch.name().unwrap() == Some("test-branch"));
    }

    #[test]
    fn test_git_signature_from_env() {
        use std::env;
        
        // Save original values
        let orig_name = env::var("GIT_AUTHOR_NAME").ok();
        let orig_email = env::var("GIT_AUTHOR_EMAIL").ok();
        
        // Set test values
        env::set_var("GIT_AUTHOR_NAME", "Test Author");
        env::set_var("GIT_AUTHOR_EMAIL", "author@test.com");
        
        // Test that we can get the values
        assert_eq!(env::var("GIT_AUTHOR_NAME").unwrap(), "Test Author");
        assert_eq!(env::var("GIT_AUTHOR_EMAIL").unwrap(), "author@test.com");
        
        // Restore original values
        match orig_name {
            Some(name) => env::set_var("GIT_AUTHOR_NAME", name),
            None => env::remove_var("GIT_AUTHOR_NAME"),
        }
        match orig_email {
            Some(email) => env::set_var("GIT_AUTHOR_EMAIL", email),
            None => env::remove_var("GIT_AUTHOR_EMAIL"),
        }
    }
}