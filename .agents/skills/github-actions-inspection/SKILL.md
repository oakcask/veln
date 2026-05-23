---
name: github-actions-inspection
description: Use when inspecting GitHub Actions workflow runs, jobs, logs, artifacts, or check failures with GitHub CLI, especially when searching run output or avoiding repeated approval prompts from shell pipelines.
---

# GitHub Actions Inspection

## Goal

Inspect GitHub Actions runs efficiently while preserving approval rules and avoiding repeated prompts caused by piping network output through local tools.

## Workflow

1. Identify the run, job, artifact, or check failure to inspect.
2. Prefer direct `gh run` or `gh api` commands with built-in flags, fields, and JSON filtering when that gives enough information.
3. When logs are needed, first try direct GitHub CLI output that reduces volume, such as `--log-failed`, `--json`, `--jq`, or `--template`.
4. When log or artifact output needs repeated searching, fetch or save the GitHub data once into a disposable cache file without shell redirection or escalation for a piped command, then run local searches against that file.
5. Search cached files with `rg` in a separate local command.
6. Report only sanitized, task-relevant excerpts or summaries. Do not quote secrets, local paths, hostnames, usernames, or irrelevant log content.

## Command Rules

- Do not combine network-producing `gh run` or `gh api` commands with `rg`, `grep`, `sed`, `awk`, `jq`, `tee`, or similar local processing in a single shell pipeline.
- Avoid command substitutions that embed `gh run` or `gh api` output into another command.
- Avoid shell redirection on network-producing `gh run` or `gh api` commands. Redirection can prevent approval-prefix matching and force approval for the whole command line.
- Do not request escalation for a command that pipes `gh run` or `gh api` output into a local helper. In this environment, each side of a pipeline can be evaluated separately, which can produce a persistent approval suggestion for the local helper instead of the stable GitHub CLI prefix.
- If a command needs approval, request it for the stable GitHub CLI prefix such as `["gh", "run"]` or `["gh", "api"]`, not for the local search step.
- Never suggest a persistent approval prefix for `node .agents/skills/github-actions-inspection/scripts/cache-stdin.mjs ...`; the output path is task-specific and should not become a reusable approval rule.
- Use `node .agents/skills/github-actions-inspection/scripts/cache-stdin.mjs OUTPUT_PATH` only when the command can run without escalation. If it would trigger approval, cancel that shape and use direct `gh` output, `--log-failed`, `--json` plus `--jq`, or a one-off approval with no persistent prefix rule.
- Use cache paths under ignored working directories such as `.cache/` or `tmp/`, and keep cache file names generic.
- Treat cached GitHub output as sensitive working data: do not copy it into docs, commits, PR descriptions, comments, or final answers unless explicitly requested and sanitized.

## Examples

Prefer this shape:

```sh
gh run view RUN_ID --job JOB_ID --log-failed
gh run view RUN_ID --json jobs --jq '.jobs[] | select(.conclusion == "failure") | {name, databaseId, conclusion}'
```

Search an existing cache in a separate local command:

```sh
rg -i "failure|error|timeout" .cache/github/job-log.txt
```

Avoid this shape:

```sh
gh run view RUN_ID --job JOB_ID --log > .cache/github/job-log.txt
gh run view RUN_ID --log | rg -i "failure|error|timeout"
gh run view RUN_ID --job JOB_ID --log | node .agents/skills/github-actions-inspection/scripts/cache-stdin.mjs .cache/github/job-log.txt
```
