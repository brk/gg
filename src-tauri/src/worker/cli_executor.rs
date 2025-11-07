//! Utilities for executing jj CLI commands as an alternative to library calls

use anyhow::{Context, Result, anyhow};
use std::process::{Command, Stdio};

/// Execute a jj CLI command and return the output
pub struct JjCliExecutor {
    repo_path: String,
}

impl JjCliExecutor {
    pub fn new(repo_path: String) -> Self {
        Self { repo_path }
    }

    /// Execute a jj command with the given arguments
    ///
    /// # Arguments
    /// * `args` - Command arguments (e.g., &["abandon", "abc123"])
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        println!("RUNNING jj {}", args.join(" "));
        let mut cmd = Command::new("jj");

        // With the -R flag one must specify repo-relative paths with
        // `root:"relative/path"`; it's simpler to just run jj from the root.
        cmd.current_dir(&self.repo_path);

        // Disable color output for easier parsing
        cmd.arg("--color=never");
        
        // Add the actual command arguments
        cmd.args(args);
        
        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output()
            .context("Failed to execute jj command")?;
        
        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("Failed to parse jj output as UTF-8")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("jj command failed: {}", stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_executor_creation() {
        let executor = JjCliExecutor::new("/tmp/test".to_string());
        assert_eq!(executor.repo_path, "/tmp/test");
    }
}
