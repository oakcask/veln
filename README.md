# Veln

Veln is an experimental programming language and toolchain for
AI-agent-assisted programming. It combines explicit public interfaces, typed
effects, contracts, and typed holes with diagnostics and repair tools that
people and coding agents can use together.

The compiler and tools are written in Rust. Programs run on the JVM. The
language and toolchain are evolving; backward compatibility is not guaranteed.

## Try it

Install from the repository root with:

- Rust and Cargo, using the stable toolchain in `rust-toolchain.toml`.
- JDK 25 or newer, with both `javac` and `java` on `PATH`.

```sh
cargo install --locked --path crates/veln-cli
```

Ensure Cargo's installation bin directory is on `PATH`, then verify the
installation with `veln --version`.

Save this as `hello.veln`:

```veln
pub fn main() -> () effects [stdio]
	stdio::println("hello from veln")
end
```

Check and run it:

```sh
veln check hello.veln
veln run main hello.veln
```

The program prints:

```text
hello from veln
```

The `main` function declares its public signature and its `stdio` effect.
Try the [CSV order analyzer](examples/csv_analyzer/README.md) for a Veln HTTP/2
backend with a TypeScript browser UI. More executable examples are in
[examples/specification/](examples/specification/).
The [three-node distributed KVS](examples/distributed_kvs/README.md) demonstrates
Raft replication, binary-schema RPCs over HTTP/2, and process-crash recovery.

## Language and tooling

- **Explicit boundaries:** public function signatures and effect declarations
  describe interfaces; effect handlers scope how operations are handled.
- **Contracts and partial programs:** preconditions, postconditions, and typed
  holes express requirements and give tools context for unfinished code.
- **Diagnostics and repairs:** JSON output, diagnostic explanations, and repair
  previews support automated feedback and source changes.
- **Editor and agent integration:** the CLI includes LSP and MCP servers; a
  VS Code extension lives in [editors/vscode/](editors/vscode/).

See the [language specification](docs/specification/README.md) for current
behavior and limits.

## Commands

After installation, use `veln` from any directory:

```sh
veln --help
```

| Command | Purpose |
| --- | --- |
| `veln check --json hello.veln` | Check source and emit structured diagnostics. |
| `veln fmt hello.veln` | Format source files. |
| `veln run main hello.veln` | Run the selected entry function. |
| `veln test path/to/project` | Run tests. |
| `veln doc path/to/project` | Generate documentation. |
| `veln repair --dry-run --json hello.veln` | Preview repair candidates. |
| `veln explain --list` | List known diagnostics. |
| `veln metrics --json path/to/project` | Report source dependency metrics. |
| `veln package lock` | Write the package lockfile. |
| `veln lsp` | Start the language server on stdio. |
| `veln mcp` | Start the MCP server on stdio. |

Use `veln <command> --help` for arguments and options.

## Documentation

- [Documentation index](docs/README.md): choose a task or subject.
- [Current specification](docs/specification/README.md): implemented language,
  command, runtime, and integration behavior.
- [Proposals](docs/proposals/README.md): planned or incomplete work.
- [Reference](docs/reference/README.md): rationale, source support, and repository
  policies.

For repository changes, start with [AGENTS.md](AGENTS.md). Use
`bash scripts/agent-test` to run the Rust workspace tests with time and memory
limits, or pass Cargo test arguments to select a smaller suite.
