package literal_paths

workflow_trigger := object.get(input, "on", object.get(input, "true", null))

deny contains msg if {
  not is_object(data.workflow_repository_files)
  msg := "generate the tracked-file inventory with workflow-scripts/workflow-path-inventory.mjs and pass it with --data so literal path filters can be checked against the checkout"
}

# GitHub path patterns use *, ?, + and character classes. A leading ! negates
# a pattern; an escaped character is literal and must still name a file.
literal_path(pattern) := path if {
  normalized := trim_prefix(pattern, "!")
  not regex.match(`(^|[^\\])(\\\\)*[*?+\[]`, normalized)
  path := regex.replace(normalized, `\\(.)`, "$1")
}

deny contains msg if {
  files := data.workflow_repository_files
  some event_name, event in workflow_trigger
  some filter in {"paths", "paths-ignore"}
  some pattern in object.get(event, filter, [])
  path := literal_path(pattern)
  not object.get(files, path, false)
  msg := sprintf("on.%s.%s entry %q does not name an existing tracked file; update or remove this filter so it cannot silently miss repository changes", [event_name, filter, pattern])
}
