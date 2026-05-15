# haunt CLI

Standalone command-line tool for managing [haunt.nvim](https://github.com/TheNoeTrevino/haunt.nvim) comments without a running Neovim instance. Reads and writes the same JSON storage files as the plugin, so changes made in either direction are immediately visible to the other.

## Install

```sh
cargo install --path .
```

## Usage

Run `haunt` from anywhere inside a git repository. The CLI detects the repo root and branch automatically, then reads/writes comment files in `<repo-root>/.haunt/<session>/`.

Most commands require a session name, either via `--session <name>` or the `HAUNT_SESSION` environment variable:

```sh
export HAUNT_SESSION=review
```

### List comments

```sh
haunt list
```

```
src/main.rs:42 TODO: refactor this [32abf904]
lua/haunt/store.lua:10 [91ab9607]
```

Filter to a specific file with `--file`:

```sh
haunt list --file src/main.rs
```

### Add a comment

```sh
haunt add --file src/main.rs --line 42 "TODO: refactor this"
# prints the new comment ID
32abf9048595248f
```

The file path is relative to the repo root. Absolute paths are also accepted and stored with `absolute: true`.

### Batch-add comments

Pipe a JSON array into `haunt apply` to add multiple comments at once:

```sh
echo '[{"file":"src/main.rs","line":10,"note":"fix this"},{"file":"src/lib.rs","line":5,"note":"check this"}]' | haunt apply
```

Each entry needs `file`, `line`, and `note`. Prints one ID per line in input order.

### Show a comment

```sh
haunt show --id 32ab
```

```
id:   32abf9048595248f
file: src/main.rs
line: 42
note: TODO: refactor this
```

IDs can be shortened to any unambiguous prefix.

### Edit a comment

```sh
haunt edit --id 32ab --note "FIXME: urgent"
haunt edit --id 32ab --line 50
haunt edit --id 32ab --note "FIXME: urgent" --line 50
```

At least one of `--note` or `--line` is required. Note cannot be empty.

### Delete a comment

```sh
haunt delete --id 32ab
```

When the last comment is deleted, the JSON file itself is removed (matching the plugin's behavior).

### Batch-edit comments

Pipe a JSON array into `haunt batch-edit` to update multiple comments at once:

```sh
echo '[{"id":"32ab","note":"updated note"},{"id":"91ab","line":20}]' | haunt batch-edit
```

Each entry needs `id` and at least one of `note` or `line`.

### List sessions

```sh
haunt sessions
```

Lists all session directories under `.haunt/`. This command does not require `--session`.

### Clear comments

```sh
# clear all comments for the current project/branch
haunt clear

# clear only comments in a specific file
haunt clear --file src/main.rs
```

## Global options

| Flag | Description |
|------|-------------|
| `--session <NAME>` | Named session (or set `HAUNT_SESSION` env var) |
| `--no-per-branch` | Use a single storage file per project instead of per-branch |

## Agent skills

The `skills/` directory contains agent skills that teach an AI agent how to use the haunt CLI effectively:

- **`skills/haunt/`** — General-purpose skill for annotating code with haunt. Covers all commands, session management, batch operations, and keeping annotations in sync after edits. Useful for guided code explanations, code tours, and any annotation workflow.
- **`skills/haunt-review/`** — Extends the base skill with a review-specific workflow: leave inline review comments on diffs or PRs, walk through findings collaboratively, and advance through comments without prompting.

## Storage compatibility

The CLI produces the exact same storage format as haunt.nvim v2:

- Storage files live in `<repo-root>/.haunt/<session>/`. In linked worktrees, root is determined via `git rev-parse --git-common-dir`, so worktrees share the same storage as the main checkout
- Filename is `sha256(project_id + "|" + branch)[0:12].json`
- `project_id` is the repo's root commit hash (`git rev-list --max-parents=0 HEAD`)
- File paths are stored relative to the repo root; files outside the repo are stored as absolute with `"absolute": true`
- Writes are atomic (write to temp file, then rename) to avoid corrupting files that Neovim might read concurrently
