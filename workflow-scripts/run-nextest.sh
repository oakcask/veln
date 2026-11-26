set -euo pipefail

suite="${1:-}"
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

exec bash scripts/ci-run cargo llvm-cov --no-report nextest --locked "${scope[@]}" "$@"
