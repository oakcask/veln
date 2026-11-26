* Follow the standard-library test helper naming and placement policy in
  `docs/reference/toolchain-test-harness.md`: use `.test.veln` for companions,
  `_test.veln` for shared internal helpers, and `_testing.veln` for
  library-provided test support. Production modules must not import internal
  test helpers.
