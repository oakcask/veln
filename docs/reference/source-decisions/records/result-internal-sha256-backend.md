# Internal SHA-256 Backend

Status: implemented

## Decision

Veln uses RustCrypto `sha2` for internal SHA-256 computation. The selected
dependency and exact resolved versions are authoritative in `Cargo.toml` and
`Cargo.lock`.

This decision covers source-tree checksums in package lockfiles and JVM class
cache keys. It does not define a public cryptography API or select a provider
for a future public API.

## Compatibility Evidence

The framing bytes, path and class ordering, length fields, cache version
prefix, and lowercase checksum representation remain unchanged. Golden tests
are the authority for digest compatibility:

- `crates/veln-project/src/tests.rs` fixes a representative source-tree
  checksum.
- `crates/veln-cli/src/java.rs` fixes the JVM cache key and the standard empty
  input and `abc` digests.
- `crates/veln-analysis/src/lib.rs` and `crates/veln-lsp/src/lib.rs` separately
  fix the diagnostic JSON and LSP wire behavior that moved in the same change.

The benchmark table below is selection evidence. It is not a performance
contract and does not create a CI threshold.

## Threat Model

The inputs are non-secret source paths, source bytes, class paths, and class
bytes. The digests detect accidental or adversarial modification of cached or
materialized content. The implementation does not process a key, a Message
Authentication Code (MAC) secret, a credential, or private-key material.

Collision resistance and byte-for-byte interoperability matter. Secret-data
side-channel properties and provider-backed key isolation are outside this
decision because no secret enters these hashes.

## Alternatives

| Backend | Result | Reason |
| --- | --- | --- |
| Two local implementations | Rejected | They duplicate cryptographic code, review effort, tests, and architecture-specific optimization work. |
| RustCrypto `sha2` | Selected | It is pure Rust, portable, `no_std` capable, and narrowly scoped. It matches every golden digest and outperforms the local implementation on the measured host. |
| System OpenSSL EVP | Rejected | Performance is comparable to `sha2`, but the binding adds C FFI, platform discovery, build scripts, and a runtime `libcrypto` dependency. OpenSSL is not one common operating-system API. |
| Vendored OpenSSL | Rejected | It removes the system-library assumption but adds a large native source build and longer, more complex cross-platform builds. |
| Operating-system APIs | Rejected | Windows, Apple, and other targets expose different APIs and availability policies. Adapters would add target-specific unsafe or FFI code for no measured benefit here. |

The [rust-openssl build documentation](https://docs.rs/openssl/latest/openssl/#building)
describes the distinct system discovery and vendored build paths.

## Dependency Review

The published `sha2` package is maintained in the active
[RustCrypto hashes repository](https://github.com/RustCrypto/hashes). It is
licensed under MIT or Apache-2.0 and declares an MSRV of Rust 1.85. The package
has no build script. Its library is `no_std`, and its source performs no
network access, process execution, telemetry, or filesystem writes.

Veln disables default features. The resolved normal dependency surface is
`digest`, `block-buffer`, `crypto-common`, `hybrid-array`, `typenum`, `cfg-if`,
and `cpufeatures`; `libc` is target-specific through CPU feature detection.
The implementation contains architecture-specific unsafe intrinsics selected
behind runtime or target feature detection. Veln calls only the safe `Digest`
and `Sha256` interface.

The [RustSec package record](https://rustsec.org/packages/sha2.html) lists one
historical AVX2 digest-correctness advisory. The affected release is older
than the selected release, and the advisory identifies a patched range that
includes the selected release. A lockfile vulnerability scan found no
advisory affecting the resolved workspace dependencies.

## Benchmark Method

The measurement used a temporary, untracked release harness on
x86_64-unknown-linux-gnu under WSL2, on an AMD Ryzen 9 7900X. The compiler was
rustc 1.97.1. The OpenSSL comparison used the `openssl` crate 0.10.81 with
system OpenSSL 3.6.3. The RustCrypto comparison used `sha2` 0.11.0 with default
features disabled.

The harness generated all input before measurement and performed no file I/O
inside the timed region. Each operation included hasher initialization,
updates, finalization, and digest retrieval. Each backend warmed up on one
thread and then produced nine samples per workload. The table reports the
median, sample range, throughput, and median time divided by the local
implementation median. Inputs were identical across backends, and the harness
asserted digest equality before timing.

The one-shot inputs contained 1 KiB, 16 KiB, 128 KiB, and 1 MiB. The JVM
workload contained 12 framed classes of approximately 24 KiB each. The small
source tree contained 18 framed files of approximately 4 KiB each. The large
source tree contained 240 framed files of approximately 12 KiB each.

The adoption gate required investigation if `sha2` was both more than 10%
slower than the local implementation and more than 1 ms slower on a
representative workload. No measured case approached that gate.

| Workload | Backend | Median ns/op | Range ns/op | MiB/s | Current ratio |
| --- | --- | ---: | ---: | ---: | ---: |
| One-shot 1 KiB | Local | 2,891.8 | 2,884.8–3,169.1 | 337.7 | 1.000 |
| One-shot 1 KiB | `sha2` | 468.0 | 461.6–486.1 | 2,086.7 | 0.162 |
| One-shot 1 KiB | OpenSSL EVP | 640.1 | 637.9–644.2 | 1,525.7 | 0.221 |
| One-shot 16 KiB | Local | 47,106.3 | 46,998.7–47,968.5 | 331.7 | 1.000 |
| One-shot 16 KiB | `sha2` | 6,662.6 | 6,649.0–6,667.3 | 2,345.2 | 0.141 |
| One-shot 16 KiB | OpenSSL EVP | 6,923.9 | 6,828.6–7,027.0 | 2,256.7 | 0.147 |
| One-shot 128 KiB | Local | 378,316.3 | 373,900.9–379,878.1 | 330.4 | 1.000 |
| One-shot 128 KiB | `sha2` | 53,825.8 | 53,530.6–54,595.1 | 2,322.3 | 0.142 |
| One-shot 128 KiB | OpenSSL EVP | 53,325.9 | 53,152.6–53,482.5 | 2,344.1 | 0.141 |
| One-shot 1 MiB | Local | 2,757,307.7 | 2,750,512.7–2,793,040.1 | 362.7 | 1.000 |
| One-shot 1 MiB | `sha2` | 398,283.7 | 392,700.0–463,778.3 | 2,510.8 | 0.144 |
| One-shot 1 MiB | OpenSSL EVP | 392,659.1 | 391,481.1–397,748.8 | 2,546.7 | 0.142 |
| JVM cache, 12 classes | Local | 845,801.9 | 842,177.0–861,016.5 | 333.2 | 1.000 |
| JVM cache, 12 classes | `sha2` | 121,983.5 | 120,151.2–123,078.6 | 2,310.7 | 0.144 |
| JVM cache, 12 classes | OpenSSL EVP | 122,465.9 | 120,142.5–125,410.9 | 2,301.6 | 0.145 |
| Source tree, 18 files | Local | 215,100.7 | 213,358.7–219,144.0 | 331.3 | 1.000 |
| Source tree, 18 files | `sha2` | 30,671.0 | 30,566.8–31,221.8 | 2,323.6 | 0.143 |
| Source tree, 18 files | OpenSSL EVP | 31,127.2 | 30,929.6–31,353.4 | 2,289.5 | 0.145 |
| Source tree, 240 files | Local | 8,474,633.4 | 8,342,157.7–8,591,361.7 | 333.5 | 1.000 |
| Source tree, 240 files | `sha2` | 1,219,080.3 | 1,206,447.0–1,234,740.5 | 2,318.5 | 0.144 |
| Source tree, 240 files | OpenSSL EVP | 1,213,183.2 | 1,210,212.7–1,227,334.2 | 2,329.7 | 0.143 |

The initial combined release harness build took 4.77 seconds and produced a
549,432-byte binary. Its ELF dependencies included `libssl.so.3` and
`libcrypto.so.3`. These values describe the selection host and are not build
or binary-size guarantees.

## Reconsideration Conditions

Reconsider this decision if Veln requires FIPS-validated operation, an
operating-system key store, HMAC, encryption, secret-key operations, or a
provider policy controlled outside the process. Treat any public cryptography
API as a separate provider-design decision. Do not infer that `sha2` is the
provider for that API from these internal checksums.
