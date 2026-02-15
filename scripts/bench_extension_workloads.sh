#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "hyperfine not installed; install with: cargo install hyperfine" >&2
  exit 1
fi

BENCH_CARGO_PROFILE="${BENCH_CARGO_PROFILE:-perf}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

cargo build --profile "$BENCH_CARGO_PROFILE" --bin pijs_workload

BIN="$TARGET_DIR/$BENCH_CARGO_PROFILE/pijs_workload"
ITERATIONS="${ITERATIONS:-200}"
TOOL_CALLS_CSV="${TOOL_CALLS_CSV:-1,10}"
HYPERFINE_WARMUP="${HYPERFINE_WARMUP:-3}"
HYPERFINE_RUNS="${HYPERFINE_RUNS:-10}"
OUT_DIR="${OUT_DIR:-$TARGET_DIR/perf/$BENCH_CARGO_PROFILE}"
JSONL_OUT="${JSONL_OUT:-$OUT_DIR/pijs_workload_${BENCH_CARGO_PROFILE}.jsonl}"

mkdir -p "$OUT_DIR"
: > "$JSONL_OUT"

IFS=',' read -r -a TOOL_CALLS_SET <<< "$TOOL_CALLS_CSV"
for TOOL_CALLS in "${TOOL_CALLS_SET[@]}"; do
  TOOL_CALLS="${TOOL_CALLS//[[:space:]]/}"
  if [[ -z "$TOOL_CALLS" ]]; then
    continue
  fi

  HYPERFINE_OUT="$OUT_DIR/hyperfine_pijs_workload_${ITERATIONS}x${TOOL_CALLS}_${BENCH_CARGO_PROFILE}.json"
  CMD="$BIN --iterations ${ITERATIONS} --tool-calls ${TOOL_CALLS}"

  hyperfine \
    --warmup "$HYPERFINE_WARMUP" \
    --runs "$HYPERFINE_RUNS" \
    --export-json "$HYPERFINE_OUT" \
    "$CMD"

  PI_BENCH_BUILD_PROFILE="$BENCH_CARGO_PROFILE" \
    "$BIN" --iterations "$ITERATIONS" --tool-calls "$TOOL_CALLS" >> "$JSONL_OUT"
done

echo "Wrote artifacts:"
echo "  - profile=$BENCH_CARGO_PROFILE"
echo "  - $JSONL_OUT"
echo "  - $OUT_DIR/hyperfine_pijs_workload_${ITERATIONS}x*_${BENCH_CARGO_PROFILE}.json"
