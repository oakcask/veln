---
role: specification
authority: normative
update-when: The veln explain command listing, diagnostic lookup, or output behavior changes.
---

# Explain Command

`explain` is a read-only diagnostic catalog command. It does not discover,
parse, check, lower, compile, or run source files.

With a known diagnostic ID, it prints the diagnostic title, a short meaning,
and a repair-oriented note. With `--list`, it prints the IDs available in the
implemented catalog. Unknown IDs and an invocation without either `--list` or
a diagnostic ID are command-line errors.

The implemented catalog covers the first diagnostic families used most often
in the typed-hole and predicate repair loop:

- `hole.unfilled`
- `hole.satisfy_type_mismatch`
- `hole.satisfy_candidate_shadow`
- `hole.satisfy_candidate_unused`
- `parse.contract_predicate`
- `parse.satisfy_candidate`
- `parse.satisfy_arrow`
- `parse.satisfy_predicate`

