---
name: haunt
description: Use the haunt CLI to annotate code with comments pinned to file:line. Supports guided explanations, code tours, reviews, and any workflow where an agent needs to leave visible annotations for the user in neovim.
---

# haunt

haunt is a CLI for placing annotations (comments pinned to file:line) that render in the user's neovim. Use it to explain code, guide through a codebase, review changes, or any workflow where you need to point the user at specific lines.

## Prerequisites

The `HAUNT_SESSION` environment variable MUST be set before using any haunt command (except `haunt sessions`). This identifies which session to operate on. If it is not set, haunt will error. Do not pass `--session` explicitly — rely on the env var.

## Commands

```bash
haunt list [--file FILE]
haunt add --file FILE --line LINE "note text"
haunt show --id ID
haunt edit --id ID [--note "new text"] [--line LINE]
haunt delete --id ID
haunt clear [--file FILE]
haunt sessions
echo '[{"file":"src/main.rs","line":10,"note":"fix this"}]' | haunt apply
echo '[{"id":"abc","line":50}]' | haunt batch-edit
```

### Global flags

- `--session NAME` — named session (stored in `.haunt/<name>/`). Falls back to `HAUNT_SESSION` env var.
- `--no-per-branch` — disable per-branch storage, use project-wide comments

### Notes

- `batch-edit` reads a JSON array from stdin; each item requires `id` and optionally `line` and/or `note`
- `add` is best for one note; `apply` is best when you have several notes ready
- `add` requires `--file`, `--line`, and the note text as a positional argument
- `apply` reads a JSON array from stdin; each item requires `file`, `line`, and `note`
- `--id` accepts a unique prefix — you don't need the full ID
- `list` output format: `file:line note [short-id]`
- `list` and `clear` accept optional `--file` to scope to a single file
- Use `\n` in note text for multi-line annotations
- Each note's text must start with a zero-padded sequential number, e.g. `01: <text>`, `02: <text>`, `10: <text>` — this controls sort order in the annotation list

## Starting a session

Always begin a new session by clearing old comments so the view is clean:

1. `haunt clear` to wipe previous comments
2. Read the code/diffs relevant to the task
3. Identify lines worth annotating
4. If you have several notes ready, prefer one `haunt apply` batch over many `haunt add` calls
5. Summarize when done

If the user asks to **continue** an existing session, skip the clear — run `haunt list` to see what's already there and pick up from that state.

## Keeping annotations in sync after code changes

**Every time a file is edited, immediately run `haunt list --file <changed-file>` to check if comment line numbers have drifted.** Fix any that no longer point to the right line — use `haunt edit --id <id> --line <new-line>` for a single fix, or pipe JSON to `haunt batch-edit` when multiple comments shifted. Do this before moving on.

## Example workflows

### Guided code explanation

The user asks you to explain a module or feature:

1. Clear the session
2. Read the relevant files
3. Place annotations on the key lines — entry points, important state transitions, non-obvious logic
4. Summarize the flow to the user and offer to walk through annotations one by one

### Code tour

The user asks for a tour of an unfamiliar area:

1. Clear the session
2. Identify the files involved and their relationships
3. Annotate in reading order — start at the entry point, follow the call chain
4. Walk the user through each annotation sequentially

### Code review

For reviewing diffs, PRs, or branches — use the `haunt-review` skill, which adds review-specific rules (don't fix code unilaterally, advance through comments without asking, what to focus on).

## Common errors

- **"no comment matching id prefix '...'"** — use `haunt list` to find valid IDs
- **"ambiguous id prefix '...' matches N comments"** — provide a longer prefix
- **"error: invalid JSON input: ..."** — `apply` stdin must be a JSON array of `{"file","line","note"}` objects
- **"error: not in a git repository"** — run from inside a git repo
- **"error: no session specified — set HAUNT_SESSION or pass --session <name>"** — set `HAUNT_SESSION` or pass `--session`
