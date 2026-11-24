package literal_paths

filtered_workflow(event, filter, paths) := {
  "on": {event: {filter: paths}},
}

test_requires_inventory if {
  violations := deny with input as {"on": {"pull_request": {}}} with data.workflow_repository_files as null
  count(violations) == 1
}

test_accepts_existing_literal_and_negated_paths if {
  every filter in {"paths", "paths-ignore"} {
    violations := deny with input as filtered_workflow("push", filter, ["src/main.rs", "!src/main.rs"])
      with data.workflow_repository_files as {"src/main.rs": true}
    count(violations) == 0
  }
}

test_rejects_missing_literal_paths_for_each_event_and_filter if {
  every event in {"push", "pull_request", "pull_request_target"} {
    every filter in {"paths", "paths-ignore"} {
      violations := deny with input as filtered_workflow(event, filter, ["missing.rs", "!removed.rs"])
        with data.workflow_repository_files as {}
      count(violations) == 2
      violations[sprintf("on.%s.%s entry %q does not name an existing tracked file; update or remove this filter so it cannot silently miss repository changes", [event, filter, "missing.rs"])]
    }
  }
}

test_skips_glob_patterns if {
  violations := deny with input as filtered_workflow("pull_request", "paths", [
    "src/*", "src/**", "file.rs?", "file.rs+", "file[0-9].rs", "!src/**",
  ]) with data.workflow_repository_files as {}
  count(violations) == 0
}

test_unescapes_literal_glob_characters if {
  violations := deny with input as filtered_workflow("pull_request", "paths", [
    `file\*.rs`, `file\?.rs`, `file\+.rs`, `file\[1].rs`, `\!file.rs`,
  ]) with data.workflow_repository_files as {
    "file*.rs": true, "file?.rs": true, "file+.rs": true,
    "file[1].rs": true, "!file.rs": true,
  }
  count(violations) == 0
}

test_rejects_missing_escaped_literal if {
  violations := deny with input as filtered_workflow("push", "paths", [`file\*.rs`])
    with data.workflow_repository_files as {}
  count(violations) == 1
}

test_glob_after_escaped_backslash_is_not_literal if {
  not literal_path(`src\\*.rs`)
}

test_handles_yaml_boolean_on_key if {
  violations := deny with input as {"true": {"pull_request": {"paths": ["missing.rs"]}}}
    with data.workflow_repository_files as {}
  count(violations) == 1
}

test_accepts_workflows_without_path_filters if {
  every trigger in {"push", ["push", "pull_request"], {"workflow_call": {}}, {"push": {"branches": ["main"]}}} {
    violations := deny with input as {"on": trigger} with data.workflow_repository_files as {}
    count(violations) == 0
  }
}
