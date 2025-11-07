# JJ CLI Migration Plan

## Overview
This document analyzes which repository mutations can be migrated from using the `jj_lib` Rust library API to invoking the `jj` CLI tool as a subprocess.

## ✅ COMPLETED MIGRATIONS

The following mutations have been successfully migrated to use the jj CLI:

1. **AbandonRevisions** - `jj abandon <commit_ids>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:14-50`
   - Notes: Fully functional, handles multiple revisions

2. **DescribeRevision** - `jj describe -m <message> <commit_id>`
   - Status: ✅ Migrated  
   - File: `src/worker/mutations_cli.rs:52-93`
   - Notes: Works for description changes. `reset_author` flag not yet supported via CLI (limitation documented)

3. **DuplicateRevisions** - `jj duplicate <commit_ids>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:95-116`
   - Notes: Fully functional, jj handles topological ordering

4. **CheckoutRevision** - `jj edit <commit_id>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:118-149`
   - Notes: Fully functional, checks out revision for editing

5. **CreateRevision** - `jj new <parent_ids>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:151-183`
   - Notes: Fully functional, creates new empty revision with specified parents

6. **UndoOperation** - `jj operation undo`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:185-207`
   - Notes: Undoes most recent operation

7. **TrackBranch** - `jj bookmark track <remote>@<branch>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:209-248`
   - Notes: Fully functional, tracks remote bookmarks

8. **UntrackBranch** - `jj bookmark untrack <bookmark>`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:250-300`
   - Notes: Supports both specific and wildcard patterns

9. **CreateRef** - `jj bookmark create` / `jj tag create`
   - Status: ✅ Migrated
   - File: `src/worker/mutations_cli.rs:302-355`
   - Notes: Supports both bookmarks and tags

10. **DeleteRef** - `jj bookmark delete` / `jj bookmark forget` / `jj tag delete`
    - Status: ✅ Migrated
    - File: `src/worker/mutations_cli.rs:357-408`
    - Notes: Uses `forget` for remote bookmarks, `delete` for local bookmarks and tags

11. **MoveRef** - `jj bookmark set` / `jj tag delete` + `jj tag create`
    - Status: ✅ Migrated
    - File: `src/worker/mutations_cli.rs:410-462`
    - Notes: Tags require delete+recreate since they can't be moved directly

## Infrastructure Added

- **`src/worker/cli_executor.rs`**: New module providing `JjCliExecutor` struct for executing jj CLI commands
  - Handles repository path context
  - Supports `--ignore-working-copy` flag
  - Captures stdout/stderr properly
  - Error handling with context
  
- **`src/worker/mutations_cli.rs`**: New module containing CLI-based mutation implementations
  - Provides `execute_cli()` methods for eligible mutations
  - Maintains immutability checks using library (required)
  - Reloads workspace state after CLI operations

- **`WorkspaceSession::cli_executor()`**: New method to get a CLI executor instance

## Analysis of Remaining Mutations

### ✅ Easy CLI Mappings (High Priority)

These operations have direct CLI equivalents with straightforward argument mappings:

1. **AbandonRevisions** → `jj abandon <revsets>`
   - Direct mapping: `jj abandon <commit_id>`
   - Supports multiple revisions
   - CLI handles descendant rebasing automatically
   
2. **DescribeRevision** → `jj describe -m <message> <revset>`
   - Direct mapping with `--message` flag
   - Supports `--reset-author` if needed (via additional flags)
   
3. **DuplicateRevisions** → `jj duplicate <revsets>`
   - Direct mapping for duplicating commits
   - CLI handles topological ordering
   
4. **CheckoutRevision** → `jj edit <revset>`
   - Direct mapping to edit/checkout a revision
   - Note: jj recommends `jj new` instead but `edit` exists
   
5. **CreateRevision** → `jj new [parent_revsets]`
   - Creates new empty commit
   - Can specify parent(s)
   - Supports `--message` flag
   
6. **GitFetch** → `jj git fetch [--remote <remote>] [--branch <branch>]`
   - Direct mapping with optional filters
   - Supports `--branch` and `--remote` options
   
7. **GitPush** → `jj git push [--remote <remote>] [--bookmark <bookmark>]`
   - Direct mapping with bookmark/remote options
   - Supports `--all` and `--tracked` flags
   
8. **UndoOperation** → `jj operation undo <operation_id>`
   - Direct mapping to undo operations
   - Requires operation ID
   
9. **CreateRef/DeleteRef** → `jj bookmark create/delete/set <name>`
   - Direct mapping for bookmark management
   - `jj bookmark create <name> -r <revset>`
   - `jj bookmark delete <name>`
   
10. **MoveRef** → `jj bookmark set <name> -r <revset>`
    - Sets bookmark to specific revision
    
11. **TrackBranch** → `jj bookmark track <remote>@<branch>`
    - Direct mapping for tracking remote bookmarks
    
12. **UntrackBranch** → `jj bookmark untrack <remote>@<branch>`
    - Direct mapping for untracking
    
13. **RenameBranch** → `jj bookmark rename <old> <new>`
    - Direct mapping for renaming bookmarks

### ⚠️ Moderate CLI Mappings (Medium Priority)

These operations can be mapped to CLI but require multiple commands or careful argument construction:

14. **MoveRevisions** → `jj rebase -r <revset> -d <destination>`
    - Maps to rebase with `--revision` flag
    - Need to handle multiple parents as merge
    
15. **MoveSource** → `jj rebase -s <revset> -d <destination>`
    - Maps to rebase with `--source` flag (rebases descendants too)
    
16. **InsertRevision** → `jj new --insert-after <after> --insert-before <before>`
    - Maps to `jj new` with insertion flags
    - May need `jj rebase` for complex cases
    
17. **CopyChanges** → `jj restore --from <from_id> --into <to_id> [paths]`
    - Maps to restore command
    - Supports path filtering
    
18. **MoveChanges** → `jj squash --from <from> --into <to> [paths]`
    - Maps to squash command
    - Supports path filtering
    - May need `--keep-emptied` flag handling

### ❌ Difficult/Impossible CLI Mappings (Low Priority / Keep Library)

These operations have no direct CLI equivalents or require complex tree manipulation:

19. **BackoutRevisions** → Partially `jj revert` or custom logic
    - CLI `jj revert` exists but may not match exact semantics
    - May require creating intermediate commits
    
20. **CreateRevisionBetween** → Complex combination of operations
    - No direct CLI equivalent
    - Requires creating commit then rebasing specific commits
    - Better kept as library operation
    
21. **MoveHunk** → No direct CLI equivalent
    - Requires interactive tree/diff manipulation
    - Would need `jj diffedit` interactively or complex scripting
    - Best kept as library operation for programmatic control
    
22. **StoreRef** → Internal operation, no CLI mapping
    - Appears to be for internal state management
    - No CLI equivalent

## Implementation Strategy

### Phase 1: Infrastructure Setup
1. Create a CLI executor utility module (`src-tauri/src/worker/cli_executor.rs`)
   - Execute jj commands with proper error handling
   - Parse JSON output where available
   - Handle working directory context
   - Capture stdout/stderr
   - Support `--ignore-working-copy` flag when needed

2. Add configuration to choose between library vs CLI mode
   - Feature flag or runtime configuration
   - Allow gradual migration

### Phase 2: Migrate Simple Operations (Priority 1-13)
Start with operations that have 1:1 CLI mappings:
- AbandonRevisions
- DescribeRevision  
- DuplicateRevisions
- CheckoutRevision
- CreateRevision
- GitFetch/GitPush
- UndoOperation
- Bookmark operations (Create/Delete/Move/Track/Untrack/Rename)

### Phase 3: Migrate Complex Operations (Priority 14-18)
Handle operations requiring multiple commands or complex arguments:
- MoveRevisions
- MoveSource
- InsertRevision
- CopyChanges
- MoveChanges

### Phase 4: Keep Library Operations (Priority 19-22)
Keep these as library operations:
- BackoutRevisions
- CreateRevisionBetween
- MoveHunk
- StoreRef

## Benefits of CLI Migration

1. **Decoupling**: Less tight binding to `jj_lib` internals
2. **Stability**: CLI is a stable interface, library API may change
3. **Debugging**: Easier to see what commands are being run
4. **Testing**: Can test against different jj versions
5. **Simplicity**: Let jj handle complex rebasing logic

## Risks & Considerations

1. **Performance**: Subprocess overhead vs in-process library calls
2. **Error Handling**: Need to parse CLI error messages
3. **Transaction Control**: CLI commands are atomic, may need multiple commands
4. **State Synchronization**: Need to refresh repository state after CLI operations
5. **Immutability Checks**: CLI has its own immutability checks, may differ from library
6. **Progress/Cancellation**: Harder to get progress updates or cancel operations

## Testing Strategy

1. Create parallel tests for CLI vs library implementations
2. Verify identical outcomes for both approaches
3. Test error cases and edge conditions
4. Performance benchmarking

## CLI Command Templates

```rust
// Abandon
jj abandon {commit_id}

// Describe
jj describe -m "{message}" {commit_id}

// Duplicate
jj duplicate {commit_ids...}

// Create new
jj new {parent_ids...} -m "{message}"

// Rebase
jj rebase -r {commit_id} -d {destination}
jj rebase -s {source} -d {destination}

// Git operations
jj git fetch --remote {remote} --branch {branch}
jj git push --remote {remote} --bookmark {bookmark}

// Bookmarks
jj bookmark create {name} -r {revset}
jj bookmark delete {name}
jj bookmark set {name} -r {revset}
jj bookmark track {remote}@{branch}
jj bookmark untrack {remote}@{branch}

// Restore/Squash
jj restore --from {from} --into {to} {paths...}
jj squash --from {from} --into {to} {paths...}

// Undo
jj operation undo {operation_id}
```

## Recommended Implementation Order

1. ✅ GitFetch, GitPush (external operations, less risk)
2. ✅ Bookmark operations (simple, side-effect free)
3. ✅ AbandonRevisions (commonly used, simple)
4. ✅ DescribeRevision (metadata only, simple)
5. ✅ CreateRevision (simple creation)
6. ✅ DuplicateRevisions (well-defined operation)
7. ⚠️ MoveRevisions, MoveSource (rebase operations)
8. ⚠️ CopyChanges (restore operation)
9. ⚠️ MoveChanges (squash operation)
10. ⚠️ CheckoutRevision (working copy manipulation)
11. ❌ Keep remaining operations as library calls

## Notes

- All CLI operations should use `--ignore-working-copy` when appropriate to avoid unintended snapshots
- Need to handle repository path with `-R` flag
- Consider using `--no-pager` and other output control flags
- May want to use `--color=never` for easier parsing
- Check for `jj` binary availability at startup
