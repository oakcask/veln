---
role: proposal
update-when: Package-documentation declaration kinds or exported public handler documentation changes.
---

# Publish public handlers in package documentation

## Outcome

Package documentation will publish public handler declarations. Each handler
will have a stable semantic identity, canonical signature, documentation, and
`veln-doc:` declaration resource. The checked standard-library bundle will
therefore expose `net::system()` from the exported `net` module while excluding
its private `prelude_builtin::net_system_*` adapters.

This target changes documentation tooling only. It does not change the
implemented `net::system()` ownership or TCP lifecycle contract.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| The exported `std::net` module is rendered as package documentation | The catalog and MCP declaration resource include `net::system()`, its canonical handler signature, and its effectful examples alongside the surrounding public `net` declarations and `write_all`; neither output includes private host adapters | package-documentation gate and checked standard-library resource bundle covering the handler declaration, examples, neighboring network API, complete-write helper, and private-declaration exclusion |

The current
[package-documentation specification](../specification/package-documentation.md#published-boundary)
owns the declaration kinds that are published today.
