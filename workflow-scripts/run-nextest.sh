set -euo pipefail

suite="${1:-}"
if [ "$#" -gt 0 ]; then
  shift
fi

coverage="${1:-}"
if [ "$#" -gt 0 ]; then
  shift
fi

case "$suite" in
  workspace)
    scope=(--workspace --exclude 'veln-repo-*')
    ;;
  toolchain)
    scope=(-p veln-cli --test toolchain_harness)
    ;;
  repository)
    scope=(-p 'veln-repo-*')
    ;;
  *)
    printf 'Unknown Rust test suite: %s. Select workspace, toolchain, or repository so Cargo builds the intended test targets.\n' "$suite" >&2
    exit 64
    ;;
esac

case "$coverage" in
  true)
    command=(cargo llvm-cov --no-report nextest)
    ;;
  false)
    command=(cargo nextest run)
    ;;
  *)
    printf 'Unknown Rust coverage mode: %s. Select true to produce main-branch coverage or false for an uninstrumented test run.\n' "$coverage" >&2
    exit 64
    ;;
esac

exec bash scripts/ci-run "${command[@]}" --locked "${scope[@]}" "$@"
