# JJ CLI Migration Summary

## What Was Done

Successfully migrated **18 repository mutations** from jj_lib library calls to jj CLI subprocess invocations:

### Phase 1 - Simple Operations (13 operations)
1. **AbandonRevisions** - Abandon commits and rebase descendants
2. **DescribeRevision** - Update commit descriptions  
3. **DuplicateRevisions** - Duplicate commits with new change IDs
4. **CheckoutRevision** - Edit/checkout a revision in the working copy
5. **CreateRevision** - Create new empty revisions with specified parents
6. **UndoOperation** - Undo the most recent operation
7. **TrackBranch** - Track remote bookmarks
8. **UntrackBranch** - Untrack remote bookmarks
9. **CreateRef** - Create bookmarks or tags
10. **DeleteRef** - Delete bookmarks or tags
11. **MoveRef** - Move bookmarks or tags to different commits
12. **GitFetch** - Fetch from git remotes
13. **GitPush** - Push to git remotes

### Phase 2 - Complex Operations (5 operations)
14. **MoveRevision** - Rebase a single revision to new parents
15. **MoveSource** - Rebase a revision and its descendants
16. **CopyChanges** - Restore file changes from one revision to another
17. **MoveChanges** - Squash file changes from one revision into another
18. **CreateRevisionBetween** - Insert a new revision between two existing ones

### New Infrastructure

#### `src/worker/cli_executor.rs`
A new utility module for executing jj CLI commands:
- `JjCliExecutor` struct that wraps subprocess execution
- Automatic repository path handling via `-R` flag
- Support for `--ignore-working-copy` to avoid working copy snapshots
- Consistent error handling and output capture
- Color output disabled for easier parsing

#### `src/worker/mutations_cli.rs`
CLI-based implementations of mutations:
- Provides `execute_cli()` methods as alternatives to library implementations
- Maintains immutability checks (requires library access)
- Automatically reloads workspace state after CLI operations
- Proper error context propagation

#### Integration Points
- Added `WorkspaceSession::cli_executor()` method for easy access
- Modified `impl Mutation` blocks to call `execute_cli()` methods
- Maintains existing API surface - no changes to callers

## How It Works

### Execution Flow
1. Frontend calls mutation (e.g., `AbandonRevisions`)
2. Mutation checks immutability using library (required)
3. CLI executor builds command arguments
4. jj subprocess executes with `--ignore-working-copy` flag
5. Workspace state reloaded from disk (`load_at_head()`)
6. Status returned to frontend

### Example: AbandonRevisions
```rust
// Before (library):
for id in &abandoned_ids {
    let commit = tx.repo().store().get_commit(id)?;
    tx.repo_mut().record_abandoned_commit(&commit);
}
tx.repo_mut().rebase_descendants()?;
ws.finish_transaction(tx, description)?;

// After (CLI):
let args = vec!["abandon", "abc123", "def456"];
cli.execute(&args, true)?;  // true = --ignore-working-copy
ws.load_at_head()?;
```

## Benefits Achieved

1. **Decoupling**: Less dependency on jj_lib internal APIs
2. **Stability**: CLI interface is more stable than library internals
3. **Simplicity**: Let jj handle complex rebasing logic
4. **Maintainability**: Easier to understand what operations are being performed
5. **Debugging**: Can see exact jj commands being executed

## Limitations & Trade-offs

### Performance
- Subprocess overhead vs in-process library calls
- Need to reload repository state after each operation
- Multiple CLI calls cannot share a transaction

### Feature Gaps
- `DescribeRevision::reset_author` has no CLI equivalent (would need metaedit or separate handling)
- Complex operations requiring tree manipulation still need library
- Immutability checks still require library access for now

### Transaction Semantics
- CLI operations are atomic but separate
- Cannot batch multiple mutations in one transaction
- Each operation triggers a new jj operation in the op-log

## Remaining Work

### ⚠️ Phase 1 Remaining (1 operation)
- RenameBranch → `jj bookmark rename` (should be easy to add)

### ⚠️ Phase 2 Remaining (1 operation)  
- InsertRevision → `jj new --insert-after/before` (similar to CreateRevisionBetween, should be straightforward)

### ❌ Keep as Library (3 operations)
- BackoutRevisions - Complex semantics, may need custom logic
- MoveHunk - Requires interactive diff editing
- StoreRef - Internal state management

## Testing Recommendations

1. **Integration Tests**: Verify CLI operations produce identical results to library versions
2. **Error Handling**: Test error cases and ensure proper error messages
3. **Performance**: Benchmark CLI vs library for common operations
4. **Compatibility**: Test against different jj versions

## Future Enhancements

1. **Batch Operations**: Investigate if multiple operations can be scripted
2. **JSON Output**: Use jj's JSON output modes where available for easier parsing
3. **Progress Reporting**: Hook into jj's progress output for long operations
4. **Configuration**: Add runtime toggle between library and CLI modes
5. **Caching**: Cache jj binary location and version info

## Files Modified

- `src/worker/mod.rs` - Added cli_executor and mutations_cli modules
- `src/worker/cli_executor.rs` - New file (104 lines)
- `src/worker/mutations_cli.rs` - New file (~800 lines)
- `src/worker/gui_util.rs` - Added cli_executor() method
- `src/worker/mutations.rs` - Updated 18 mutation implementations (~540 lines removed)
- `CLI_MIGRATION_PLAN.md` - Updated with completed migrations
- `CLI_MIGRATION_SUMMARY.md` - This file

## Verification

Build status: ✅ Success
```
cargo build --manifest-path src-tauri/Cargo.toml
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.31s
```

Warnings: 16 warnings (unused functions and variables, not errors)
