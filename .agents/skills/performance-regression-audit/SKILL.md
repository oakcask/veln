---
name: performance-regression-audit
description: Use when investigating slow tests, performance regressions, or changes to analysis algorithms that process large generated inputs, predicates, static truth checks, repair reasoning, parsing, lowering, typechecking, or other compiler-wide scans. Ensures agents measure the slow path, check fast-path ordering, and verify representative high-cardinality cases before reporting the work complete.
---

# Performance Regression Audit

## Goal

Prevent agents from fixing or extending compiler analysis code while leaving an avoidable slow path in place.

## Workflow

1. Reproduce the slow case directly and record the observed test time from the command output.
2. Measure by pipeline stage when possible, such as parse, lower, analyze, diagnostic rendering, or repair generation.
3. Compare adjacent input sizes or related tests when the case is generated or high-cardinality.
4. Inspect predicate, proof, repair, parser, and typechecker code for repeated whole-input scans, repeated string splitting, repeated normalization, nested pairwise checks, truth-table evaluation, or broad fallback heuristics.
5. Put cheap conclusive checks before broad heuristic searches when they produce the same externally visible result.
6. Route known large shapes to dedicated classifiers before generic algorithms when the codebase already has those classifiers.
7. Prefer parsing or splitting once per level over repeatedly rediscovering top-level structure from the same text.
8. Run the slow target again and at least one neighboring or larger representative case after the change.
9. Run the relevant crate or package test suite when the touched code affects shared analysis behavior.

## Review Checklist

- Does a validated static result bypass slower repair or proof exploration when the final reason/status would be the same?
- Does the code avoid re-tokenizing or re-normalizing the same large predicate inside inner loops?
- Is the fallback path ordered from cheapest and most decisive to broadest and most expensive?
- Did verification include the original slow test and a case that would expose the same growth pattern?
- If a wall-time threshold is considered, is it stable enough for CI? If not, keep the guardrail procedural and document the representative commands in the final report.

## Reporting

Include the original timing, the new timing, the main hot path found, and the tests run. If precise profiling tools are unavailable, say which stage timing or comparative test data supports the conclusion.
