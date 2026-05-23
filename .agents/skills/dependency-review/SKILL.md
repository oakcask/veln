---
name: dependency-review
description: Review proposed third-party dependencies before adoption. Use when Codex is asked to add, choose, compare, upgrade, or approve a dependency, especially runtime dependencies, packages with install scripts or native code, broad permissions, unclear maintenance status, or security-sensitive behavior.
---

# Dependency Review

## Goal

Choose dependencies that are current, actively maintained, and low risk enough for the project. Prefer avoiding a dependency when the benefit is small or the risk cannot be understood and reduced.

## Workflow

1. Identify why the dependency is needed and whether the standard library, existing dependencies, or a small local implementation would be simpler.
2. Check freshness and maintenance health: recent releases, commit activity, issue and pull request responsiveness, maintainer continuity, package ownership, license clarity, and compatibility with the target runtime.
3. Inspect the published package and repository source, not only the README.
4. Review install-time and runtime behavior for security risk: lifecycle scripts, native code, generated or minified source, network calls, filesystem writes, process execution, environment variable access, credential handling, telemetry, and broad permissions.
5. Inspect the transitive dependency surface and known vulnerability reports using the repository's package manager and lockfile tooling when available.
6. Compare maintained alternatives when the first candidate is stale, overpowered for the need, or hard to audit.
7. Decide whether to adopt, avoid, or mitigate. Prefer pinning, narrowing import surface, sandboxing, configuration hardening, or replacing the package when risks are manageable.
8. Record the adoption reason, reviewed risks, and mitigations in the change description.

## Red Flags

- No meaningful release or repository activity for a long period while the ecosystem has moved on.
- Unclear ownership, abandoned issue tracker, or unreviewed maintainer transfer.
- Obfuscated, generated, or bundled code that cannot be traced to source.
- Install scripts, binary downloads, native addons, or postinstall behavior that are not necessary for the package's purpose.
- Unexpected access to credentials, home directories, shell execution, network services, or writable project files.
- Large transitive dependency trees for a narrow feature.
- Security reports without a timely fix, workaround, or clear maintainer response.

## Output

When reporting the review, include:

- Recommendation: adopt, adopt with mitigations, or avoid.
- Reason the dependency is needed.
- Maintenance and freshness summary.
- Source and package security findings.
- Transitive dependency and vulnerability findings.
- Alternatives considered when relevant.
- Required mitigations or follow-up checks.
