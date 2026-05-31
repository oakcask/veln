# Veln

Veln is an experimental programming language and toolchain based on
<https://oakcask.github.io/docs/202605-programming-language-for-agents/>.

The current prototype explores a language for AI-agent-assisted programming:
small syntax, explicit public boundaries, typed holes, contracts, coarse
effects, and structured diagnostics are treated as one tool surface.

## Commands

Use the prototype CLI as `veln`:

```sh
veln --version
veln check samples/demo/hello.veln
veln fmt samples/demo/hello.veln
veln run main samples/demo/hello.veln
veln test samples/demo
```

The CLI shape is:

```text
veln check [--json] [path ...]
veln fmt [path ...]
veln run <entry> [path ...]
veln test [--json] [target ...]
```

## Example

```veln
use stdio

pub fn main() -> () effects [stdio]
  stdio::println("hello from veln")
  stdio::eprintln("stderr from veln")
  ()
end
```

```sh
veln check samples/demo/hello.veln
```

```text
ok
```

```sh
veln run main samples/demo/hello.veln
```

```text
stderr from veln
hello from veln
```

## Agent-Oriented Features

Public functions carry explicit signatures and effect declarations:

```veln
pub fn main() -> () effects [stdio]
  stdio::println("hello from veln")
  ()
end
```

Typed holes keep partial programs checkable and report repair context:

```veln
pub fn main() -> Result<(), AppError> effects []
  _todo satisfy candidate => candidate == Ok(())
end
```

```sh
veln check samples/demo/hole_error.veln
```

```text
samples/demo/hole_error.veln:2:3: hint[hole.unfilled]: hole requires a `Result<(), AppError>` value
  note: samples/demo/hole_error.veln:1:1: Return type declared here.
  note: samples/demo/hole_error.veln:2:9: Satisfy predicate contributes a repair constraint.
```

The same diagnostics can be emitted as JSON for tools and coding agents:

```sh
veln check --json samples/demo/hole_error.veln
```
