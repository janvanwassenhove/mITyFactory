//! Git operations for project management.
//!
//! Provides Git functionality for initializing repos, committing, and publishing
//! generated applications to remote repositories.

use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// Git repository information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepo {
    pub path: PathBuf,
    pub initialized: bool,
    pub has_remote: bool,
    pub remote_url: Option<String>,
    pub current_branch: Option<String>,
    pub has_uncommitted_changes: bool,
    pub commit_count: usize,
}

/// Git remote configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
}

/// Git commit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub author: Option<String>,
    pub timestamp: Option<String>,
}

/// Git status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub ahead: usize,
    pub behind: usize,
}

/// Git operations manager.
#[derive(Debug)]
pub struct GitOps {
    repo_path: PathBuf,
}

impl GitOps {
    /// Create a new Git operations manager for a repository.
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
        }
    }

    /// Check if Git is available on the system.
    pub fn is_git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if the repository is initialized.
    pub fn is_initialized(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    /// Initialize a Git repository.
    pub fn init(&self) -> CoreResult<()> {
        if self.is_initialized() {
            debug!("Repository already initialized");
            return Ok(());
        }

        info!("Initializing Git repository at {}", self.repo_path.display());

        let output = Command::new("git")
            .args(["init"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to run git init: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git init failed: {}", stderr)));
        }

        Ok(())
    }

    /// Get repository status.
    pub fn get_status(&self) -> CoreResult<GitStatus> {
        if !self.is_initialized() {
            return Err(CoreError::GitError("Repository not initialized".to_string()));
        }

        // Get current branch
        let branch = self.get_current_branch().ok();

        // Get porcelain status
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to get status: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git status failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut staged_files = Vec::new();
        let mut unstaged_files = Vec::new();
        let mut untracked_files = Vec::new();

        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }
            let x = &line[0..1];
            let y = &line[1..2];
            let file = &line[3..];

            if x != " " && x != "?" {
                staged_files.push(file.to_string());
            }
            if y != " " && y != "?" {
                unstaged_files.push(file.to_string());
            }
            if x == "?" && y == "?" {
                untracked_files.push(file.to_string());
            }
        }

        // Get ahead/behind info
        let (ahead, behind) = self.get_ahead_behind().unwrap_or((0, 0));

        Ok(GitStatus {
            branch,
            staged_files,
            unstaged_files,
            untracked_files,
            ahead,
            behind,
        })
    }

    /// Get current branch name.
    pub fn get_current_branch(&self) -> CoreResult<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to get branch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git branch failed: {}", stderr)));
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            return Err(CoreError::GitError("No branch found".to_string()));
        }

        Ok(branch)
    }

    /// Get ahead/behind commit counts.
    fn get_ahead_behind(&self) -> CoreResult<(usize, usize)> {
        let output = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to get ahead/behind: {}", e)))?;

        if !output.status.success() {
            // No upstream configured
            return Ok((0, 0));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split('\t').collect();
        if parts.len() == 2 {
            let ahead = parts[0].parse().unwrap_or(0);
            let behind = parts[1].parse().unwrap_or(0);
            return Ok((ahead, behind));
        }

        Ok((0, 0))
    }

    /// Add files to staging.
    pub fn add(&self, paths: &[&str]) -> CoreResult<()> {
        if !self.is_initialized() {
            return Err(CoreError::GitError("Repository not initialized".to_string()));
        }

        let mut args = vec!["add"];
        args.extend(paths);

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to add files: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git add failed: {}", stderr)));
        }

        Ok(())
    }

    /// Add all files to staging.
    pub fn add_all(&self) -> CoreResult<()> {
        self.add(&["."])
    }

    /// Commit staged changes.
    pub fn commit(&self, message: &str) -> CoreResult<GitCommit> {
        if !self.is_initialized() {
            return Err(CoreError::GitError("Repository not initialized".to_string()));
        }

        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("nothing to commit") {
                return Err(CoreError::GitError("Nothing to commit".to_string()));
            }
            return Err(CoreError::GitError(format!("git commit failed: {}", stderr)));
        }

        // Get commit hash
        let hash_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to get commit hash: {}", e)))?;

        let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

        Ok(GitCommit {
            hash,
            message: message.to_string(),
            author: None,
            timestamp: None,
        })
    }

    /// Get remote URL.
    pub fn get_remote(&self, name: &str) -> CoreResult<String> {
        let output = Command::new("git")
            .args(["remote", "get-url", name])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to get remote: {}", e)))?;

        if !output.status.success() {
            return Err(CoreError::GitError(format!("No remote '{}' found", name)));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(url)
    }

    /// List all remotes.
    pub fn list_remotes(&self) -> CoreResult<Vec<GitRemote>> {
        let output = Command::new("git")
            .args(["remote", "-v"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to list remotes: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git remote failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut remotes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                
                // Only add each remote once (git remote -v shows fetch and push)
                if seen.insert(name.clone()) {
                    remotes.push(GitRemote { name, url });
                }
            }
        }

        Ok(remotes)
    }

    /// Add a remote.
    pub fn add_remote(&self, name: &str, url: &str) -> CoreResult<()> {
        if !self.is_initialized() {
            return Err(CoreError::GitError("Repository not initialized".to_string()));
        }

        // Check if remote already exists
        if self.get_remote(name).is_ok() {
            // Update existing remote
            let output = Command::new("git")
                .args(["remote", "set-url", name, url])
                .current_dir(&self.repo_path)
                .output()
                .map_err(|e| CoreError::GitError(format!("Failed to update remote: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CoreError::GitError(format!("git remote set-url failed: {}", stderr)));
            }
        } else {
            // Add new remote
            let output = Command::new("git")
                .args(["remote", "add", name, url])
                .current_dir(&self.repo_path)
                .output()
                .map_err(|e| CoreError::GitError(format!("Failed to add remote: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CoreError::GitError(format!("git remote add failed: {}", stderr)));
            }
        }

        info!("Added remote '{}' -> {}", name, url);
        Ok(())
    }

    /// Remove a remote.
    pub fn remove_remote(&self, name: &str) -> CoreResult<()> {
        let output = Command::new("git")
            .args(["remote", "remove", name])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to remove remote: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git remote remove failed: {}", stderr)));
        }

        Ok(())
    }

    /// Push to remote.
    pub fn push(&self, remote: &str, branch: &str, force: bool) -> CoreResult<()> {
        if !self.is_initialized() {
            return Err(CoreError::GitError("Repository not initialized".to_string()));
        }

        let mut args = vec!["push"];
        if force {
            args.push("--force");
        }
        args.push("--set-upstream");
        args.push(remote);
        args.push(branch);

        info!("Pushing to {} {}", remote, branch);

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to push: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git push failed: {}", stderr)));
        }

        Ok(())
    }

    /// Pull from remote.
    pub fn pull(&self, remote: &str, branch: &str) -> CoreResult<()> {
        let output = Command::new("git")
            .args(["pull", remote, branch])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| CoreError::GitError(format!("Failed to pull: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError(format!("git pull failed: {}", stderr)));
        }

        Ok(())
    }

    /// Get repository information.
    pub fn get_repo_info(&self) -> CoreResult<GitRepo> {
        let initialized = self.is_initialized();
        
        if !initialized {
            return Ok(GitRepo {
                path: self.repo_path.clone(),
                initialized: false,
                has_remote: false,
                remote_url: None,
                current_branch: None,
                has_uncommitted_changes: false,
                commit_count: 0,
            });
        }

        let remotes = self.list_remotes().unwrap_or_default();
        let has_remote = !remotes.is_empty();
        let remote_url = remotes.first().map(|r| r.url.clone());
        
        let current_branch = self.get_current_branch().ok();
        
        let status = self.get_status().unwrap_or_else(|_| GitStatus {
            branch: None,
            staged_files: vec![],
            unstaged_files: vec![],
            untracked_files: vec![],
            ahead: 0,
            behind: 0,
        });
        
        let has_uncommitted_changes = !status.staged_files.is_empty() 
            || !status.unstaged_files.is_empty() 
            || !status.untracked_files.is_empty();

        // Get commit count
        let count_output = Command::new("git")
            .args(["rev-list", "--count", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .ok();

        let commit_count = count_output
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .parse::<usize>()
                        .ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        Ok(GitRepo {
            path: self.repo_path.clone(),
            initialized,
            has_remote,
            remote_url,
            current_branch,
            has_uncommitted_changes,
            commit_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_available() {
        // This will fail if git is not installed, which is expected
        let available = GitOps::is_git_available();
        println!("Git available: {}", available);
    }

    #[test]
    fn test_init_repo() {
        if !GitOps::is_git_available() {
            println!("Git not available, skipping test");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOps::new(temp_dir.path());

        assert!(!git_ops.is_initialized());
        git_ops.init().unwrap();
        assert!(git_ops.is_initialized());
    }

    #[test]
    fn test_repo_info() {
        if !GitOps::is_git_available() {
            println!("Git not available, skipping test");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOps::new(temp_dir.path());

        let info = git_ops.get_repo_info().unwrap();
        assert!(!info.initialized);
        assert!(!info.has_remote);
    }
}
