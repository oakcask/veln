---
name: github-actions-design
description: Use when adding, changing, or reviewing GitHub Actions workflows or repository-local actions, especially when deciding whether behavior belongs in workflow YAML, workflow-scripts, a local action, or a reusable workflow. Do not use solely to inspect runs, logs, or artifacts.
---

# GitHub Actions Design

## Goal

Keep workflow YAML focused on orchestration while placing behavior behind
boundaries that can be tested, reused, and reviewed without executing a full
workflow.

## Placement

Choose the narrowest boundary that owns the behavior:

- Keep a direct command or short pipeline in a workflow `run` step.
- Put parsing, calculation, shell control flow, retries, cleanup, and other
  multi-step behavior in `workflow-scripts/`. Export the decision-making parts
  where practical and cover meaningful success and failure paths with tests.
- Use a repository-local action under `actions/{action-name}` when a related
  sequence of action calls, inputs, outputs, and runner setup forms one reusable
  step. Do not use a local action merely to hide complex untested shell code.
- Use a reusable workflow when callers need to share a job or job graph,
  including its runner, permissions boundary, matrix, or other workflow-level
  orchestration.

Keep event triggers, permissions, concurrency, job dependencies, and matrices
visible in workflow YAML because they define the GitHub Actions execution
boundary.

## Workflow

1. Inspect nearby workflows, `workflow-scripts/`, local actions, and workflow
   policy before choosing a new boundary. Reuse the repository's established
   naming and invocation patterns.
2. Separate GitHub orchestration from repository behavior. Treat branches and
   loops in `run` steps as evidence that the behavior belongs in a tested
   repository script; the workflow Conftest policy enforces this boundary.
3. Keep script interfaces explicit through arguments, environment variables,
   files, and GitHub outputs. Avoid coupling testable logic directly to ambient
   runner state when dependencies can be passed in.
4. Keep path-filtered workflows synchronized with the scripts and exact local
   action manifests they consume. Follow the existing workflow policy instead
   of weakening it with broad wildcard exceptions.
5. When output is intended to guide a maintainer or agent, use
   `$ci-message-policy` so the action and its reason are clear.
6. Add tests at the extracted behavior boundary. Cover the fallback and cleanup
   paths when workflow availability depends on them, not only the successful
   case.

## Verification

- Run the focused tests for changed files under `workflow-scripts/`.
- Run `conftest verify -p conftest/workflow` when workflow policy changes.
- Run `conftest test -o github -p conftest/workflow` against changed workflows,
  or against `.github/workflows` when the policy itself changes.
- Run `actionlint` when available.
- Confirm path filters include each new repository script or exact local action
  manifest used by a filtered workflow.
