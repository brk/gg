//! CLI-based implementations of mutations
//! These use the jj CLI tool instead of library calls

use anyhow::{Context, Result};

use crate::messages::{
    AbandonRevisions, CheckoutRevision, CreateRef, CreateRevision, DeleteRef, DescribeRevision,
    DuplicateRevisions, GitFetch, MoveRef, MutationResult, StoreRef, TrackBranch,
    UndoOperation, UntrackBranch,
};

use super::gui_util::WorkspaceSession;

/// CLI-based implementation of AbandonRevisions
impl AbandonRevisions {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let revset_expr = ws.parse_revset_str(&self.revset)?;
        if ws.check_immutable_revset(revset_expr)? {
            return Ok(MutationResult::PreconditionError {
                message: format!("Cannot abandon immutable revision"),
            });
        }

        let args = vec!["abandon", &self.revset];
        
        let cli = ws.cli_executor();
        cli.execute(&args)
            .context("Failed to abandon revisions via CLI")?;
        
        // Reload the workspace to reflect changes
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of DescribeRevision
impl DescribeRevision {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Check immutability first
        let commit = ws.resolve_single_change(&self.id)?;
        
        if ws.check_immutable(vec![commit.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: format!("Revision {} is immutable", self.id.change.prefix),
            });
        }

        // Check if there's actually a change
        if self.new_description == commit.description() && !self.reset_author {
            return Ok(MutationResult::Unchanged);
        }

        let cli = ws.cli_executor();
        let args = vec!["describe", "-m", &self.new_description, &self.id.commit.hex];
        
        // Note: reset_author would need additional handling as CLI doesn't have direct flag
        // This is a limitation of the CLI approach
        if self.reset_author {
            // This would require a separate metaedit command or similar
            // For now, we'll document this as a limitation
        }
        
        cli.execute(&args)
            .context("Failed to describe revision via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of DuplicateRevisions  
impl DuplicateRevisions {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        // Build the command
        let commit_ids: Vec<&str> = self.ids.iter().map(|id| id.commit.hex.as_str()).collect();
        let mut args = vec!["duplicate"];
        args.extend(&commit_ids);
        
        cli.execute(&args)
            .context("Failed to duplicate revisions via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of CheckoutRevision
impl CheckoutRevision {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let commit = ws.resolve_single_change(&self.id)?;
        
        if ws.check_immutable(vec![commit.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: "Revision is immutable".to_string(),
            });
        }

        if commit.id() == ws.wc_id() {
            return Ok(MutationResult::Unchanged);
        }

        let cli = ws.cli_executor();
        let args = vec!["edit", &self.id.commit.hex];
        
        cli.execute(&args)
            .context("Failed to checkout revision via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            let new_selection = ws.format_header(&commit, Some(false))?;
            Ok(MutationResult::UpdatedSelection {
                new_status: ws.format_status(),
                new_selection,
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of CreateRevision
impl CreateRevision {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        // Build parent revision arguments
        let parent_ids: Vec<String> = self.parent_ids.iter().map(|id| id.commit.hex.clone()).collect();
        let mut args = vec!["new"];
        
        // Add parent arguments
        for parent_id in &parent_ids {
            args.push(parent_id.as_str());
        }
        
        cli.execute(&args)
            .context("Failed to create new revision via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            // Get the new working copy commit
            let new_commit = ws.get_commit(ws.wc_id())?;
            let new_selection = ws.format_header(&new_commit, Some(false))?;
            Ok(MutationResult::UpdatedSelection {
                new_status: ws.format_status(),
                new_selection,
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of UndoOperation
impl UndoOperation {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        // Undo the most recent operation (@)
        let args = vec!["operation", "undo"];
        
        cli.execute(&args)
            .context("Failed to undo operation via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            let working_copy = ws.get_commit(ws.wc_id())?;
            let new_selection = ws.format_header(&working_copy, None)?;
            Ok(MutationResult::UpdatedSelection {
                new_status: ws.format_status(),
                new_selection,
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of TrackBranch
impl TrackBranch {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        match self.r#ref {
            StoreRef::Tag { tag_name } => {
                return Ok(MutationResult::PreconditionError {
                    message: format!("{} is a tag and cannot be tracked", tag_name),
                });
            }
            StoreRef::LocalBookmark { branch_name, .. } => {
                return Ok(MutationResult::PreconditionError {
                    message: format!("{} is a local bookmark and cannot be tracked", branch_name),
                });
            }
            StoreRef::RemoteBookmark {
                branch_name,
                remote_name,
                ..
            } => {
                let bookmark_ref = format!("{}@{}", branch_name, remote_name);
                let args = vec!["bookmark", "track", &bookmark_ref];
                
                cli.execute(&args)
                    .context("Failed to track branch via CLI")?;
                
                let changed = ws.load_at_head()?;
                
                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
        }
    }
}

/// CLI-based implementation of UntrackBranch
impl UntrackBranch {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        match self.r#ref {
            StoreRef::Tag { tag_name } => {
                return Ok(MutationResult::PreconditionError {
                    message: format!("{} is a tag and cannot be untracked", tag_name),
                });
            }
            StoreRef::LocalBookmark { branch_name, .. } => {
                // Untrack all remotes for this local bookmark
                // Use glob pattern to match all remotes
                let bookmark_pattern = format!("{}@*", branch_name);
                let args = vec!["bookmark", "untrack", &bookmark_pattern];
                
                cli.execute(&args)
                    .context("Failed to untrack branch via CLI")?;

                let changed = ws.load_at_head()?;

                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
            StoreRef::RemoteBookmark {
                branch_name,
                remote_name,
                ..
            } => {
                let bookmark_ref = format!("{}@{}", branch_name, remote_name);
                let args = vec!["bookmark", "untrack", &bookmark_ref];

                cli.execute(&args)
                    .context("Failed to untrack branch via CLI")?;
                
                let changed = ws.load_at_head()?;
                
                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
        }
    }
}

/// CLI-based implementation of CreateRef
impl CreateRef {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        let _commit = ws.resolve_single_change(&self.id)?;
        
        match self.r#ref {
            StoreRef::RemoteBookmark {
                branch_name,
                remote_name,
                ..
            } => {
                return Ok(MutationResult::PreconditionError {
                    message: format!(
                        "{}@{} is a remote bookmark and cannot be created",
                        branch_name, remote_name
                    ),
                });
            }
            StoreRef::LocalBookmark { branch_name, .. } => {
                let args = vec!["bookmark", "create", &branch_name, "-r", &self.id.commit.hex];

                cli.execute(&args)
                    .context("Failed to create bookmark via CLI")?;

                let changed = ws.load_at_head()?;

                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
            StoreRef::Tag { tag_name } => {
                let args = vec!["tag", "create", &tag_name, "-r", &self.id.commit.hex];

                cli.execute(&args)
                    .context("Failed to create tag via CLI")?;
                
                let changed = ws.load_at_head()?;
                
                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
        }
    }
}

/// CLI-based implementation of DeleteRef
impl DeleteRef {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        match self.r#ref {
            StoreRef::RemoteBookmark {
                branch_name,
                remote_name,
                ..
            } => {
                let bookmark_ref = format!("{}@{}", branch_name, remote_name);
                let args = vec!["bookmark", "forget", &bookmark_ref];

                cli.execute(&args)
                    .context("Failed to delete remote bookmark via CLI")?;

                let changed = ws.load_at_head()?;

                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
            StoreRef::LocalBookmark { branch_name, .. } => {
                let args = vec!["bookmark", "delete", &branch_name];

                cli.execute(&args)
                    .context("Failed to delete bookmark via CLI")?;

                let changed = ws.load_at_head()?;

                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
            StoreRef::Tag { tag_name } => {
                let args = vec!["tag", "delete", &tag_name];

                cli.execute(&args)
                    .context("Failed to delete tag via CLI")?;
                
                let changed = ws.load_at_head()?;
                
                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
        }
    }
}

/// CLI-based implementation of MoveRef
impl MoveRef {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        let _commit = ws.resolve_single_change(&self.to_id)?;
        
        match self.r#ref {
            StoreRef::RemoteBookmark {
                branch_name,
                remote_name,
                ..
            } => {
                return Ok(MutationResult::PreconditionError {
                    message: format!(
                        "{}@{} is a remote bookmark and cannot be moved",
                        branch_name, remote_name
                    ),
                });
            }
            StoreRef::LocalBookmark { branch_name, .. } => {
                let args = vec!["bookmark", "set", &branch_name, "-r", &self.to_id.commit.hex, "--allow-backwards"];

                cli.execute(&args)
                    .context("Failed to move bookmark via CLI")?;

                let changed = ws.load_at_head()?;

                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
            StoreRef::Tag { tag_name } => {
                // Tags need to be deleted and recreated
                let args_delete = vec!["tag", "delete", &tag_name];
                cli.execute(&args_delete)
                    .context("Failed to delete tag via CLI")?;

                let args_create = vec!["tag", "create", &tag_name, "-r", &self.to_id.commit.hex];
                cli.execute(&args_create)
                    .context("Failed to recreate tag via CLI")?;
                
                let changed = ws.load_at_head()?;
                
                if changed {
                    Ok(MutationResult::Updated {
                        new_status: ws.format_status(),
                    })
                } else {
                    Ok(MutationResult::Unchanged)
                }
            }
        }
    }
}

/// CLI-based implementation of GitFetch
impl GitFetch {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        // Collect all arguments as owned strings to avoid lifetime issues
        let mut args: Vec<String> = vec!["git".to_string(), "fetch".to_string()];
        
        match self {
            GitFetch::AllBookmarks { remote_name } => {
                // Fetch all branches from a specific remote
                args.push("--remote".to_string());
                args.push(remote_name);
            }
            GitFetch::AllRemotes { branch_ref } => {
                // Fetch a specific branch from all remotes
                let branch_name = branch_ref.as_branch()?.to_string();
                args.push("--branch".to_string());
                args.push(branch_name);
            }
            GitFetch::RemoteBookmark {
                remote_name,
                branch_ref,
            } => {
                // Fetch a specific branch from a specific remote
                let branch_name = branch_ref.as_branch()?.to_string();
                args.push("--remote".to_string());
                args.push(remote_name);
                args.push("--branch".to_string());
                args.push(branch_name);
            }
        }
        
        // Convert to &str for execute
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to fetch from git remote via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/*
impl GitPush {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let cli = ws.cli_executor();
        
        // Collect all arguments as owned strings to avoid lifetime issues
        let mut args: Vec<String> = vec!["git".to_string(), "push".to_string()];
        
        match self {
            GitPush::AllBookmarks { remote_name } => {
                // Push all tracked bookmarks to a specific remote
                args.push("--remote".to_string());
                args.push(remote_name);
                args.push("--tracked".to_string());
            }
            GitPush::AllRemotes { branch_ref } => {
                // Push a specific branch to all remotes
                let branch_name = branch_ref.as_branch()?.to_string();
                args.push("--bookmark".to_string());
                args.push(branch_name);
            }
            GitPush::RemoteBookmark {
                remote_name,
                branch_ref,
            } => {
                // Push a specific branch to a specific remote
                let branch_name = branch_ref.as_branch()?.to_string();
                args.push("--remote".to_string());
                args.push(remote_name);
                args.push("--bookmark".to_string());
                args.push(branch_name);
            }
        }
        
        // Convert to &str for execute
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to push to git remote via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}
*/

/// CLI-based implementation of MoveRevisions  
/// Maps to: jj rebase -r <revset> -d <destination>
impl crate::messages::MoveRevisions {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let revset_expr = ws.parse_revset_str(&self.revset)?;
        if ws.check_immutable_revset(revset_expr)? {
            return Ok(MutationResult::PreconditionError {
                message: format!("Cannot move immutable revision"),
            });
        }

        let cli = ws.cli_executor();
        
        // Build arguments: jj rebase -r <revset> -d <parent1> -d <parent2> ...
        let mut args: Vec<String> = vec!["rebase".to_string(), "-r".to_string(), self.revset];
        
        for parent_id in &self.parent_ids {
            args.push("-d".to_string());
            args.push(parent_id.commit.hex.clone());
        }
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to rebase revision via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of MoveSource
/// Maps to: jj rebase -s <source> -d <destination>
impl crate::messages::MoveSource {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let target = ws.resolve_single_change(&self.id)?;
        
        if ws.check_immutable(vec![target.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: format!("Revision {} is immutable", self.id.change.prefix),
            });
        }

        let cli = ws.cli_executor();
        
        // Build arguments: jj rebase -s <source> -d <parent1> -d <parent2> ...
        // Note: parent_ids are CommitId (not RevId) for MoveSource
        let mut args: Vec<String> = vec!["rebase".to_string(), "-s".to_string(), self.id.commit.hex.clone()];
        
        for parent_id in &self.parent_ids {
            args.push("-d".to_string());
            args.push(parent_id.hex.clone());  // CommitId.hex, not .commit.hex
        }
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to rebase source via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of CopyChanges
/// Maps to: jj restore --from <from> --into <to> [paths]
impl crate::messages::CopyChanges {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let to_commit = ws.resolve_single_change(&self.to_id)?;
        
        if ws.check_immutable(vec![to_commit.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: "Revisions are immutable".to_string(),
            });
        }

        let cli = ws.cli_executor();
        
        // Build arguments: jj restore --from <from> --into <to> [paths]
        // Note: from_id is CommitId, to_id is RevId
        let mut args: Vec<String> = vec![
            "restore".to_string(),
            "--from".to_string(),
            self.from_id.hex.clone(),  // CommitId.hex
            "--into".to_string(),
            self.to_id.commit.hex.clone(),  // RevId.commit.hex
        ];
        
        // Add paths if specified
        for path in &self.paths {
            args.push(path.repo_path.clone());
        }
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to restore changes via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of MoveChanges
/// Maps to: jj squash --from <from> --into <to> [paths]
impl crate::messages::MoveChanges {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Note: to_id is CommitId for MoveChanges
        let to_id_hex = &self.to_id.hex;
        let to_commit = ws.resolve_single_commit(&self.to_id)?;
        
        if ws.check_immutable(vec![to_commit.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: "Revisions are immutable".to_string(),
            });
        }

        let cli = ws.cli_executor();
        // Build arguments: jj squash --from <from> --into <to> [paths]
        // Note: from_id is RevId, to_id is CommitId
        let mut args: Vec<String> = vec![
            "squash".to_string(),
            "--from".to_string(),
            self.from_id.commit.hex.clone(),  // RevId.commit.hex
            "--into".to_string(),
            to_id_hex.clone(),  // CommitId.hex
        ];
        
        // Add paths if specified
        for path in &self.paths {
            args.push(path.repo_path.clone());
        }

        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        cli.execute(&args_str)
            .context("Failed to squash changes via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of CreateRevisionBetween
/// Maps to: jj new --insert-after A --insert-before B
impl crate::messages::CreateRevisionBetween {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Check immutability of the 'before' revision
        let before_commit = ws.resolve_single_change(&self.before_id)?;
        
        if ws.check_immutable(vec![before_commit.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: "'Before' revision is immutable".to_string(),
            });
        }

        let cli = ws.cli_executor();
        
        // Note: after_id is CommitId, before_id is RevId
        let args: Vec<String> = vec![
            "new".to_string(),
            "--insert-after".to_string(),
            self.after_id.hex.clone(),  // CommitId.hex
            "--insert-before".to_string(),
            self.before_id.commit.hex.clone(),  // RevId.commit.hex
        ];
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to create revision between via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            // Get the new working copy commit (the newly created revision)
            let new_commit = ws.get_commit(ws.wc_id())?;
            let new_selection = ws.format_header(&new_commit, Some(false))?;
            Ok(MutationResult::UpdatedSelection {
                new_status: ws.format_status(),
                new_selection,
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// CLI-based implementation of RenameBranch
/// Maps to: jj bookmark rename <old> <new>
impl crate::messages::RenameBranch {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let old_name = self.r#ref.as_branch()?;
        
        let cli = ws.cli_executor();
        
        // Build arguments: jj bookmark rename <old> <new>
        let args: Vec<String> = vec![
            "bookmark".to_string(),
            "rename".to_string(),
            old_name.to_string(),
            self.new_name.clone(),
        ];
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cli.execute(&args_str)
            .context("Failed to rename bookmark via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}

/// jj rebase -r <target> -A <after> -B <before>
impl crate::messages::InsertRevisions {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        let before = ws.resolve_single_change(&self.before_id)?;
        if ws.check_immutable(vec![before.id().clone()])? {
            return Ok(MutationResult::PreconditionError {
                message: "Cannot insert before immutable revision".to_string(),
            });
        }

        let revset_expr = ws.parse_revset_str(&self.revset)?;
        if ws.check_immutable_revset(revset_expr)? {
            return Ok(MutationResult::PreconditionError {
                message: format!("Cannot move immutable revision"),
            });
        }

        let args: Vec<String> = vec![
            "rebase".to_string(),
            "-r".to_string(),
            self.revset,
            "--insert-after".to_string(),
            self.after_id.commit.hex.clone(),
            "--insert-before".to_string(),
            self.before_id.commit.hex.clone(),
        ];
        
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let cli = ws.cli_executor();
        cli.execute(&args_str)
            .context("Failed to insert revision via CLI")?;
        
        let changed = ws.load_at_head()?;
        
        if changed {
            Ok(MutationResult::Updated {
                new_status: ws.format_status(),
            })
        } else {
            Ok(MutationResult::Unchanged)
        }
    }
}
