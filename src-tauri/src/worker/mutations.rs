use std::sync::Arc;

use anyhow::{Result, anyhow};
use jj_lib::backend::{CopyId, FileId, MergedTreeId, TreeValue};
use jj_lib::merge::Merge;
use jj_lib::merged_tree::{MergedTree, MergedTreeBuilder};
use jj_lib::{
    backend::{BackendError, CommitId},
    commit::Commit,
    object_id::ObjectId as ObjectIdTrait,
    repo::Repo,
    repo_path::RepoPath,
    rewrite::{self},
    store::Store,
};
use pollster::block_on;
use tokio::io::AsyncReadExt as _;

use crate::messages::{
    AbandonRevisions, BackoutRevisions, CheckoutRevision, CopyChanges, CreateRef, CreateRevision,
    CreateRevisionBetween, DeleteRef, DescribeRevision, DuplicateRevisions, GitFetch, GitPush,
    InsertRevision, MoveChanges, MoveHunk, MoveRef, MoveRevisions, MoveSource, MutationResult,
    RenameBranch, TrackBranch, UndoOperation, UntrackBranch,
};

use super::Mutation;
use super::gui_util::WorkspaceSession;

macro_rules! precondition {
    ($($args:tt)*) => {
        return Ok(MutationResult::PreconditionError { message: format!($($args)*) })
    }
}

impl Mutation for AbandonRevisions {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for BackoutRevisions {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        if self.ids.len() != 1 {
            precondition!("Not implemented for >1 rev");
        }

        let mut tx = ws.start_transaction()?;

        let working_copy = ws.get_commit(ws.wc_id())?;
        let reverted = ws.resolve_multiple_changes(self.ids)?;
        let reverted_parents: Result<Vec<_>, BackendError> = reverted[0].parents().collect();

        let old_base_tree = block_on(rewrite::merge_commit_trees(tx.repo(), &reverted_parents?))?;
        let new_base_tree = working_copy.tree()?;
        let old_tree = reverted[0].tree()?;
        let new_tree = block_on(new_base_tree.merge(old_tree, old_base_tree))?;

        tx.repo_mut()
            .rewrite_commit(&working_copy)
            .set_tree_id(new_tree.id())
            .write()?;

        match ws.finish_transaction(tx, format!("back out commit {}", reverted[0].id().hex()))? {
            Some(new_status) => Ok(MutationResult::Updated { new_status }),
            None => Ok(MutationResult::Unchanged),
        }
    }
}

impl Mutation for CheckoutRevision {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for CreateRevision {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for CreateRevisionBetween {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for DescribeRevision {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for DuplicateRevisions {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for InsertRevision {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for MoveRevisions {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for MoveSource {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for MoveChanges {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for CopyChanges {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for TrackBranch {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for UntrackBranch {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for RenameBranch {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}


impl Mutation for CreateRef {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for DeleteRef {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

// does not currently enforce fast-forwards
impl Mutation for MoveRef {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Use CLI-based implementation
        self.execute_cli(ws)
    }
}

impl Mutation for MoveHunk {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        // Resolve the source (from), target, and parent commits
        let from_commit = ws.resolve_single_change(&self.from_id)?;
        let target_commit = ws.resolve_single_commit(&self.to_id)?;

        // Get parent commit - error if merge commit for now
        let from_parents: Result<Vec<Commit>, jj_lib::backend::BackendError> =
            from_commit.parents().collect();
        let from_parents = from_parents?; // Handle potential error
        if from_parents.len() != 1 {
            precondition!("Cannot move hunk from a merge commit yet");
        }
        let parent_commit = &from_parents[0];

        let repo_path = RepoPath::from_internal_string(&self.path.repo_path)?;
        let store = ws.repo().store();

        // Get trees
        let from_tree = from_commit.tree()?;
        let target_tree = target_commit.tree()?;
        let parent_tree = parent_commit.tree()?;

        // --- Check relationship ---
        let is_descendant = ws
            .repo()
            .index()
            .is_ancestor(from_commit.id(), target_commit.id());

        // --- Calculate intermediate trees ---

        // 1. Calculate `hunk_tree`: parent_tree + hunk
        let parent_content = read_file_content(store, &parent_tree, &repo_path)?;
        let hunk_content = apply_hunk(&parent_content, &self.hunk)?;
        let hunk_blob_id = block_on(store.write_file(&repo_path, &mut hunk_content.as_slice()))?;
        // Resolve potential conflicts in source tree path to determine executable status
        let hunk_executable = match from_tree.path_value(&repo_path)?.into_resolved() {
            Ok(Some(tree_value)) => {
                matches!(
                    tree_value,
                    TreeValue::File {
                        executable: true,
                        ..
                    }
                )
            }
            Ok(None) => false, // Not present or resolved to nothing
            Err(conflict) => {
                // Handle conflict: for now, treat as an error.
                // Alternatively, could default to non-executable or try to resolve further.
                return Err(anyhow!(
                    "Conflict determining executable status for path: {}: {:?}",
                    repo_path.as_internal_file_string(),
                    conflict
                ));
            }
        };
        let hunk_tree_id = update_tree_entry(
            store,
            &parent_tree,
            &repo_path,
            hunk_blob_id,
            hunk_executable,
        )?;
        let hunk_tree = ws.repo().store().get_root_tree(&hunk_tree_id)?;

        // 2. Calculate `remainder_tree`: from_tree with hunk reverted.
        //    remainder = from_tree - (hunk_tree - parent_tree)
        //    This is equivalent to the 3-way merge: from_tree.merge(&hunk_tree, &parent_tree)
        let remainder_tree = block_on(from_tree.merge(hunk_tree.clone(), parent_tree.clone()))?;
        let remainder_tree_id = remainder_tree.id();

        // --- Merge and Rewrite ---

        let mut tx = ws.start_transaction()?;

        // Handle source commit (abandon or rewrite)
        let abandon_source = remainder_tree_id == parent_tree.id();
        let mut rewritten_source_id: Option<CommitId> = None; // Store the ID of A'

        if abandon_source {
            if is_descendant {
                // To simplify parent mapping for B later, disallow this for now.
                precondition!(
                    "Moving a hunk from a commit that becomes empty to a descendant is not yet supported."
                );
            }
            tx.repo_mut().record_abandoned_commit(&from_commit);
        } else {
            let rewritten_source = tx
                .repo_mut()
                .rewrite_commit(&from_commit)
                .set_tree_id(remainder_tree_id.clone())
                .write()?;
            rewritten_source_id = Some(rewritten_source.id().clone());
        }

        // Rewrite target commit or its parents
        if is_descendant {
            // Target is descendant: Rewrite B's parents to include A', keep B's tree
            let b_original_parent_ids = target_commit.parent_ids();
            if b_original_parent_ids.len() > 1 {
                precondition!("Moving a hunk to a descendant merge commit is not yet supported.");
            }

            let a_prime_id = rewritten_source_id.ok_or_else(|| {
                anyhow!("Source commit was abandoned, cannot reparent descendant")
            })?;

            // Assuming A was the only parent for simplicity (enforced by check above)
            let new_parent_ids = vec![a_prime_id];

            tx.repo_mut()
                .rewrite_commit(&target_commit)
                .set_parents(new_parent_ids)
                // Keep the original target_tree.id()
                .set_tree_id(target_commit.tree_id().clone())
                .write()?;
        } else {
            // Target is not descendant: Merge hunk change into target tree
            let target_tree_id = target_tree.id();
            let new_target_tree =
                block_on(target_tree.merge(parent_tree.clone(), hunk_tree.clone()))?;
            let new_target_tree_id = new_target_tree.id();

            // Rewrite target commit only if its tree changed
            if new_target_tree_id != target_tree_id {
                tx.repo_mut()
                    .rewrite_commit(&target_commit)
                    .set_tree_id(new_target_tree_id.clone())
                    .write()?;
            }
        }

        // Rebase descendants (handles descendants of A' and B'/original B)
        tx.repo_mut().rebase_descendants()?;

        match ws.finish_transaction(
            tx,
            format!(
                "move hunk in file {} from {} to {}",
                self.path.repo_path,
                ws.format_commit_id(from_commit.id()).hex,
                ws.format_commit_id(target_commit.id()).hex
            ),
        )? {
            Some(new_status) => Ok(MutationResult::Updated { new_status }),
            None => Ok(MutationResult::Unchanged),
        }
    }
}

// --- Helper Functions ---

// Reads content of a file from a tree, returns empty Vec if not found or not a file.
fn read_file_content(store: &Arc<Store>, tree: &MergedTree, path: &RepoPath) -> Result<Vec<u8>> {
    match tree.path_value(path)?.into_resolved() {
        Ok(Some(TreeValue::File { id, .. })) => {
            let mut reader = block_on(store.read_file(path, &id))?;
            let mut content = Vec::new();
            block_on(reader.read_to_end(&mut content))?;
            Ok(content)
        }
        Ok(Some(_)) => Ok(Vec::new()), // Found, but not a file (Tree, Symlink, Conflict, GitSubmodule)
        Ok(None) => Ok(Vec::new()),    // Not found
        Err(conflict) => {
            // Handle conflict when reading file content
            Err(anyhow!(
                "Conflict reading file content for path: {}: {:?}",
                path.as_internal_file_string(),
                conflict
            ))
        }
    }
}

// Applies a hunk (diff format) to content.
fn apply_hunk(content: &[u8], hunk: &crate::messages::ChangeHunk) -> Result<Vec<u8>> {
    let base_text = String::from_utf8_lossy(content);
    let base_lines: Vec<&str> = base_text.lines().collect();
    // Check if the original content ended with a newline
    let ends_with_newline = content.ends_with(b"\n");

    let mut result_lines: Vec<String> = Vec::new();
    let mut base_line_idx: usize;
    let mut hunk_lines = hunk.lines.lines.iter().peekable();

    // TODO: Handle potential errors like hunk not matching content.

    // 1. Add lines from base content before the hunk starts.
    //    hunk.location.from_file.start is 1-based index of the *first context/removed line*.
    let hunk_start_line_0_based = hunk.location.from_file.start.saturating_sub(1);
    result_lines.extend(
        base_lines[..hunk_start_line_0_based]
            .iter()
            .map(|s| s.to_string()),
    );
    base_line_idx = hunk_start_line_0_based;

    // 2. Process the hunk lines.
    while let Some(hunk_line) = hunk_lines.next() {
        if hunk_line.starts_with(' ') || hunk_line.starts_with('-') {
            // Context line (' ') or removed line ('-'): consume from base.
            // Check if base content matches context/removed line, ignoring trailing whitespace.
            let hunk_content_part = &hunk_line[1..];
            if base_line_idx < base_lines.len()
                && base_lines[base_line_idx].trim_end() == hunk_content_part.trim_end()
            {
                // Only add context lines to the result
                if hunk_line.starts_with(' ') {
                    // Add the original base line, preserving its exact whitespace
                    result_lines.push(base_lines[base_line_idx].to_string());
                }
                base_line_idx += 1;
            } else {
                // Hunk doesn't match base content
                return Err(anyhow!(
                    "Hunk mismatch: context/removed line does not match base content at index {}. Expected (trimmed): '{}', Found (trimmed): '{}'",
                    base_line_idx,
                    hunk_content_part.trim_end(), // Show trimmed in error for clarity
                    base_lines
                        .get(base_line_idx)
                        .map_or("<EOF>", |l| l.trim_end())
                ));
            }
        } else if hunk_line.starts_with('+') {
            // Added line ('+'): add to result, removing potential trailing newline.
            let added_content = hunk_line[1..].trim_end_matches('\n');
            result_lines.push(added_content.to_string());
        } else {
            // Malformed hunk line
            return Err(anyhow!("Malformed hunk line: {}", hunk_line));
        }
    }

    // 3. Add remaining lines from base content after the hunk.
    result_lines.extend(base_lines[base_line_idx..].iter().map(|s| s.to_string()));

    // --- Reconstruction with trailing newline handling ---
    let mut result_bytes = Vec::new();
    let num_lines = result_lines.len();
    for (i, line) in result_lines.iter().enumerate() {
        result_bytes.extend_from_slice(line.as_bytes());
        // Add newline after every line *except* the last one
        if i < num_lines - 1 {
            result_bytes.push(b'\n');
        }
    }

    // Add trailing newline only if original had one and result is not empty
    if ends_with_newline && !result_bytes.is_empty() {
        // Ensure we don't add a *double* newline if the last line already ended with one
        // (unlikely with lines() iterator, but safer to check)
        if !result_bytes.ends_with(b"\n") {
            result_bytes.push(b'\n');
        }
    }

    Ok(result_bytes)
}

// Helper function to update an entry in a tree with a new blob.
fn update_tree_entry(
    store: &Arc<jj_lib::store::Store>,
    original_tree: &MergedTree,
    path: &RepoPath,
    new_blob: FileId,
    executable: bool,
) -> Result<MergedTreeId, anyhow::Error> {
    let mut builder = MergedTreeBuilder::new(original_tree.id().clone());
    builder.set_or_remove(
        path.to_owned(),
        Merge::normal(TreeValue::File {
            id: new_blob,
            executable,
            copy_id: CopyId::placeholder(),
        }),
    );
    let new_tree_id = builder.write_tree(store)?;
    Ok(new_tree_id)
}

impl Mutation for GitPush {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        self.execute_cli(ws)
    }
}

impl Mutation for GitFetch {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        self.execute_cli(ws)
    }
}
impl Mutation for UndoOperation {
    fn execute(self: Box<Self>, ws: &mut WorkspaceSession) -> Result<MutationResult> {
        self.execute_cli(ws)
    }
}
