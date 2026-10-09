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

## Related work

### Semantic documentation items

- **Identity and locator:** Rust's
  [`rustdoc_json_types` source at revision
  `282215592`](https://github.com/rust-lang/rust/blob/282215592/src/rustdoc-json-types/lib.rs),
  exposed in the `1.101.0-nightly` interface snapshot as
  [`rustdoc_json_types::Item`](https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/struct.Item.html)
  and
  [`rustdoc_json_types::Function`](https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/struct.Function.html).
- **Relevant finding:** rustdoc JSON represents a documented item with an ID,
  name, visibility, documentation, and a kind-specific payload. Function items
  carry structured signature and generic information. The crate result also
  maps item IDs to fully qualified paths, while its own documentation warns
  that an item can have several paths and that the selected path is an
  implementation detail.
- **Applicability and limits:** This is direct implementation evidence that a
  documentation catalog can preserve declaration kind, visibility,
  documentation, semantic path, and signature separately. Rust has no Veln
  algebraic-effect handler declaration, and rustdoc's nightly JSON interface
  does not define a stable cross-snapshot ID or a canonical Veln signature.
- **Veln consequence:** a public handler becomes its own declaration kind. Its
  Veln identity uses the existing package-documentation identity inputs:
  declaration kind, fully qualified semantic name, and canonical signature.
  It does not use source order, byte offsets, or a function-shaped surrogate.
  The proposal rejects rendering a handler only as module prose or pretending
  that `net::system()` is a function, because either choice loses the
  declaration kind or signature needed for lookup and change identity.

### Declaration resources

- **Identity and locator:** [Model Context Protocol, *Resources*
  specification](https://modelcontextprotocol.io/specification/2025-06-18/server/resources).
- **Relevant finding:** MCP resources are individually identified by URIs;
  clients discover them through `resources/list` and retrieve their contents
  through `resources/read`. The protocol permits custom URI schemes that
  conform to RFC 3986.
- **Applicability and limits:** MCP defines transport behavior and resource
  identity, not how a language documentation generator chooses declaration
  IDs, canonicalizes signatures, or filters visibility. Its examples are
  file-oriented rather than language-declaration-oriented.
- **Veln consequence:** the public handler participates in the existing
  declaration catalog and receives an individually readable `veln-doc:` URI.
  The proposal rejects catalog-only handler text with no declaration resource,
  because MCP clients could discover the catalog entry but could not follow
  the same declaration-read route used by other public declarations.

### Checked examples and the public boundary

- **Identity and locator:** [The rustdoc book, *Documentation
  tests*](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html).
- **Relevant finding:** rustdoc extracts documentation examples and executes
  them as tests. It can compile hidden setup while omitting that setup from the
  rendered example, and doctests link against only public crate items.
- **Applicability and limits:** Rust doctests have different syntax and runtime
  behavior from Veln effectful examples. This source supports keeping rendered
  examples aligned with executable checks and public API, but it does not say
  that every checked example must be published or identify the examples needed
  to explain a handler.
- **Veln consequence:** the handler resource includes its visible, already
  gated effectful examples and surrounding public `net` declarations needed to
  use them. Private `prelude_builtin::net_system_*` adapters remain excluded,
  as do hidden setup details. The proposal rejects publishing private adapters
  merely because an example or handler implementation depends on them, and it
  rejects a signature-only handler entry with no usage example.

## Unsupported questions and access limits

The focused review covered official rustdoc implementation documentation, the
rustdoc doctest guide, the versioned MCP Resources specification, and Veln's
current package-documentation and source-less lookup specifications. These
sources were publicly accessible. No inspected external source defines an
algebraic-effect handler documentation kind equivalent to Veln's, the exact
canonical text of its signature, or the declaration-ID digest transcript.
They also do not establish that `net::system()`, neighboring public network
declarations, `write_all`, and the selected effectful examples are a complete
teaching set. Those questions remain Veln-specific and must be settled by the
canonicalization contract plus the checked package-documentation fixture; the
comparative sources do not by themselves verify the acceptance case.
