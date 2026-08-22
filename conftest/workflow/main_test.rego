package main

workflow_with(paths, uses) := {
  "name": "test / local action",
  "on": {
    "pull_request": {"paths": paths},
  },
  "permissions": {"contents": "read"},
  "jobs": {
    "test": {
      "runs-on": "ubuntu-latest",
      "steps": [{"uses": uses}],
    },
  },
}

workflow_with_run(run) := {
  "name": "test / inline shell",
  "on": {"pull_request": {}},
  "permissions": {"contents": "read"},
  "jobs": {
    "test": {
      "runs-on": "ubuntu-latest",
      "steps": [{
        "name": "Prepare reports",
        "run": run,
      }],
    },
  },
}

test_accepts_exact_local_action_manifest_in_filtered_trigger if {
  violations := deny with input as workflow_with(
    [".github/workflows/test--local-action.yaml", "actions/example/action.yaml"],
    "./actions/example",
  )
  count(violations) == 0
}

test_rejects_missing_local_action_manifest_in_filtered_trigger if {
  violations := deny with input as workflow_with(
    [".github/workflows/test--local-action.yaml"],
    "./actions/example",
  )
  violations["add \"actions/example/action.yaml\" to on.pull_request.paths because this workflow uses local action \"./actions/example\""]
}

test_rejects_wildcard_local_action_path if {
  violations := deny with input as workflow_with(
    [".github/workflows/test--local-action.yaml", "actions/**"],
    "./actions/example",
  )
  violations["replace wildcard path \"actions/**\" in on.pull_request.paths with each exact local action.yaml path; wildcard custom-action trigger paths are not allowed"]
}

test_requires_each_local_action_manifest if {
  workflow := {
    "name": "test / local actions",
    "on": {
      "pull_request": {
        "paths": [
          ".github/workflows/test--local-actions.yaml",
          "actions/first/action.yaml",
        ],
      },
    },
    "permissions": {"contents": "read"},
    "jobs": {
      "test": {
        "runs-on": "ubuntu-latest",
        "steps": [
          {"uses": "./actions/first"},
          {"uses": "./actions/second"},
        ],
      },
    },
  }
  violations := deny with input as workflow
  violations["add \"actions/second/action.yaml\" to on.pull_request.paths because this workflow uses local action \"./actions/second\""]
}

test_ignores_external_actions if {
  violations := deny with input as workflow_with(
    [".github/workflows/test--local-action.yaml"],
    "actions/checkout@v4",
  )
  count(violations) == 0
}

test_accepts_local_action_without_path_filter if {
  unfiltered := {
    "name": "test / local action",
    "on": {"pull_request": {}},
    "permissions": {"contents": "read"},
    "jobs": {
      "test": {
        "runs-on": "ubuntu-latest",
        "steps": [{"uses": "./actions/example"}],
      },
    },
  }
  violations := deny with input as unfiltered
  count(violations) == 0
}

test_accepts_multiline_commands_without_shell_control_flow if {
  violations := deny with input as workflow_with_run(`set -euo pipefail
cargo run --locked -p veln-repo-metrics -- \
  --format json \
  crates tools`)
  count(violations) == 0
}

test_rejects_inline_shell_branch if {
  violations := deny with input as workflow_with_run(`if gh run list; then
  echo "history available"
fi`)
  violations["move shell control flow from step \"Prepare reports\" in job \"test\" into a tested repository script so branch and loop behavior is covered by tests"]
}

test_rejects_inline_shell_loop if {
  violations := deny with input as workflow_with_run(`for shard in 1 2 3 4; do
  echo "${shard}"
done`)
  violations["move shell control flow from step \"Prepare reports\" in job \"test\" into a tested repository script so branch and loop behavior is covered by tests"]
}
