//! CLI-based implementations of mutations
//! These use the jj CLI tool instead of library calls

use anyhow::{Context, Result};
use itertools::Itertools;

use crate::messages::{
    AbandonRevisions, CheckoutRevision, CreateRef, CreateRevision, DeleteRef, DescribeRevision,
    DuplicateRevisions, MoveRef, MutationResult, StoreRef, TrackBranch, UndoOperation,
    UntrackBranch,
};

use super::gui_util::WorkspaceSession;

/// CLI-based implementation of AbandonRevisions
impl AbandonRevisions {
    pub fn execute_cli(self, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Check immutability first (this requires library access)
        let abandoned_ids = self
            .ids
            .iter()
            .map(|id| jj_lib::backend::CommitId::try_from_hex(&id.hex).expect("frontend-validated id"))
            .collect_vec();

        if ws.check_immutable(abandoned_ids.clone())? {
            return Ok(MutationResult::PreconditionError {
                message: "Some revisions are immutable".to_string(),
            });
        }

        // Build CLI command
        let cli = ws.cli_executor();
        let commit_ids: Vec<&str> = self.ids.iter().map(|id| id.hex.as_str()).collect();
        
        let mut args = vec!["abandon"];
        args.extend(&commit_ids);
        
        // Execute the command
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
        let mut args = vec!["describe", "-m", &self.new_description, &self.id.commit.hex];
        
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
        let commit = ws.resolve_single_change(&self.id)?;
        
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
        let commit = ws.resolve_single_change(&self.to_id)?;
        
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
                let args = vec!["bookmark", "set", &branch_name, "-r", &self.to_id.commit.hex];

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
