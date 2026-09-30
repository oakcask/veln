* Child tasks inherit their execution's effect handlers, and independent
  executions must not share fake state. Leaving an injected scope restores
  the previous handler even when the body fails.
* Check ambient effect configuration with
  `bash scripts/agent-test -p veln-backend-jvm effect_boundary_policy` from the
  repository root. Environment access for `process::env` and diagnostic output
  remains separate from effect substitution.
