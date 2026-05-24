package main

workflow_trigger := object.get(input, "on", object.get(input, "true", null))

workflow_name_parts(name) := parts if {
  regex.match(`^[^/]+ / [^/]+$`, name)
  parts := split(name, " / ")
}

workflow_file_segment(part) := lower(replace(part, " ", "-"))

default workflow_file_name := ""

workflow_file_name := data.conftest.file.name if {
  data.conftest.file.name
}

deny contains msg if {
  name := object.get(input, "name", "")
  name != ""
  not workflow_name_parts(name)
  msg := "workflow name must follow {subsystem} / {task name}"
}

deny contains msg if {
  name := object.get(input, "name", "")
  parts := workflow_name_parts(name)
  expected := sprintf("%s--%s.yaml", [
    workflow_file_segment(parts[0]),
    workflow_file_segment(parts[1]),
  ])
  workflow_file_name != ""
  workflow_file_name != expected
  msg := sprintf("workflow file name must be .github/workflows/%s for name %q", [expected, name])
}

deny contains msg if {
  not object.get(input, "name", "")
  msg := "workflow must define a non-empty top-level name"
}

deny contains msg if {
  not workflow_trigger
  msg := "workflow must define a top-level on trigger"
}

deny contains msg if {
  not input.permissions
  msg := "workflow should define top-level permissions"
}

deny contains msg if {
  not input.jobs
  msg := "workflow must define at least one job"
}

deny contains msg if {
  some job_name
  job := input.jobs[job_name]
  not job["runs-on"]
  not job.uses
  msg := sprintf("job %q must define runs-on unless it uses a reusable workflow", [job_name])
}

deny contains msg if {
  some job_name
  job := input.jobs[job_name]
  not job.uses
  count(object.get(job, "steps", [])) == 0
  msg := sprintf("job %q must define at least one step unless it uses a reusable workflow", [job_name])
}

deny contains msg if {
  push := object.get(workflow_trigger, "push", null)
  pull_request := object.get(workflow_trigger, "pull_request", null)
  push != null
  pull_request != null
  not object.get(push, "paths", null)
  msg := "workflow with both push and pull_request triggers must define push.paths"
}

deny contains msg if {
  push := object.get(workflow_trigger, "push", null)
  pull_request := object.get(workflow_trigger, "pull_request", null)
  push != null
  pull_request != null
  not object.get(pull_request, "paths", null)
  msg := "workflow with both push and pull_request triggers must define pull_request.paths"
}

deny contains msg if {
  push := object.get(workflow_trigger, "push", null)
  pull_request := object.get(workflow_trigger, "pull_request", null)
  push_paths := object.get(push, "paths", null)
  pull_request_paths := object.get(pull_request, "paths", null)
  push_paths != null
  pull_request_paths != null
  push_paths != pull_request_paths
  msg := "workflow push.paths and pull_request.paths must match"
}
