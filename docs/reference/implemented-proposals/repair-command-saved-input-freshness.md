# Repair Command Saved Input Freshness

Status: implemented

This record keeps completion evidence for the implemented saved repair JSON
input and freshness target. Use the specification pages for current behavior.

## Read First

- Current repair candidate behavior:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Current applying-command gates:
  [../../specification/repair-application.md](../../specification/repair-application.md).
- Current repair JSON behavior:
  [../../specification/repair-json.md](../../specification/repair-json.md).
- Executable repair cases:
  `examples/specification/repair/README.md`.

## Outcome

The completed target added saved repair JSON input without making saved files
write authorization.

- `repair` can load saved candidate input from a repair JSON envelope,
  command-level candidate object or array, check JSON envelope, or advisory
  candidate object or array.
- Saved command-level repair ids are accepted for selection while current
  command-local `repair-N` ids are regenerated for output.
- Automatic application of saved input requires matching current safe evidence
  for each non-empty replacement edit.
- Override can apply saved manual-review input only with explicit
  confirmation, and the normal target validation and verification gates still
  run before success.
- Executable cases cover saved preview normalization, saved-input freshness,
  confirmed override, and rollback after verification failure.

## Boundary

This target did not add the remaining repair-loop axes: external verification
commands, broader candidate ranking evidence, partial application of a
candidate's edit set, or broader automatic application authority. Those remain
proposal work only after a narrow target is selected and documented.
