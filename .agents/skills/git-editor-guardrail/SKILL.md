---
name: git-editor-guardrail
description: Keep git operations noninteractive when commands may open an editor. Use when Codex runs or proposes git commit, amend, rebase, merge, cherry-pick, revert, tag, or continuation commands that could pause for commit-message editing, rebase todo editing, or annotated tag messages, unless the user explicitly requests interactive editor behavior.
---

# Git Editor Guardrail

## Overview

Avoid git commands that can hang waiting for an editor. Prefer flags that make the intended outcome explicit, and set editor environment variables only for command forms that still may invoke an editor.

## Decision Rules

- If the command has a noninteractive flag for the intended behavior, prefer the flag.
- If git may still open the commit-message editor, prefix the command with `GIT_EDITOR=true`.
- If an interactive rebase todo editor may open and the intended todo is already supplied or should be accepted unchanged, prefix with `GIT_SEQUENCE_EDITOR=true`.
- Do not suppress the editor when the user asks to edit text interactively.
- Do not set editor variables globally; apply them only to the specific git command.

## Command Patterns

Use these forms when they match the task:

- `git commit -m "subject"` for subject-only messages, or `git commit -F message-file` for Markdown, multiline, or backtick-containing messages, instead of opening an editor.
- Do not pass Markdown commit messages with backticks through shell-interpreted inline strings, because backticks can be evaluated before git receives the message.
- `git commit --amend --no-edit` when preserving the existing message.
- `GIT_EDITOR=true git commit --amend` only when git may request message editing and the default message should be accepted.
- `GIT_EDITOR=true git rebase --continue` for continuation steps that may reopen a commit message.
- `GIT_EDITOR=true git merge --continue` or `git merge --no-edit <ref>` when a merge commit message may be edited.
- `git revert --no-edit <ref>` when accepting the generated revert message.
- `git tag -a <name> -m "message"` or `git tag -s <name> -m "message"` for annotated or signed tags.
- `GIT_EDITOR=true git cherry-pick --continue` when conflict resolution may lead to commit-message editing.

## Interactive Rebase

Avoid `git rebase -i` unless the task genuinely needs todo editing. When a rebase continuation only needs to proceed after conflicts are resolved, use `GIT_EDITOR=true git rebase --continue`.

For scripted interactive rebases where the todo should be accepted unchanged, use `GIT_SEQUENCE_EDITOR=true git rebase -i <base>`. Do this only when accepting the existing todo is intentional; otherwise prepare a noninteractive rebase strategy or ask before relying on a real editor.
