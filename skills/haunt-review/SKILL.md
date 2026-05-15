---
name: haunt-review
description: Review code changes and leave haunt annotations. Use when the user asks to review a diff, PR, or branch changes interactively with inline comments.
---

# haunt Review

This skill extends `haunt`. Refer to that skill for CLI commands and general annotation patterns.

## Review workflow

### Starting a new review

1. `haunt clear` to wipe previous comments
2. Read the diff or files under review
3. Identify lines worth commenting on
4. Number each note sequentially with zero-padded prefixes — `01: `, `02: `, … `10: ` — so annotations sort correctly in reading order
5. If you have several notes ready, prefer one `haunt apply` batch over many `haunt add` calls
6. Summarize when done

### Picking up an existing review

1. `haunt list` to see what comments already exist
2. Summarize the current state of the review for the user
3. When the user asks specific questions or wants to dig deeper, read the relevant code/diffs to answer

## Rules

### IMPORTANT: Never fix code without asking

**Do NOT edit code or make changes on your own during a review.** For every comment, present it to the user and ask whether they want you to fix it, skip it, or handle it differently. The review is collaborative — your role is to surface issues and discuss them, not to act unilaterally.

### Moving through comments

After a comment is dealt with — whether the user fixes it, skips it, or dismisses it — **always move to the next comment immediately without asking.** Delete resolved comments with `haunt delete`; leave skipped ones in place. Then `haunt list` to find the next one. Never ask "want to move to the next comment?" — just do it.

### What to comment on

- Keep comments focused: intent, structure, risks, or follow-ups
- Don't comment on every change — highlight what the user wouldn't spot themselves
- Focus on: correctness, subtle bugs, API design, performance implications, missing edge cases
