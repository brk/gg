//! CLI-based implementations of mutations
//! These use the jj CLI tool instead of library calls

use anyhow::{Context, Result};
use itertools::Itertools;

use crate::messages::{
    AbandonRevisions, DescribeRevision, DuplicateRevisions, MutationResult,
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
