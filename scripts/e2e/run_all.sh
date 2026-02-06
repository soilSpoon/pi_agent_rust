#!/usr/bin/env bash
# scripts/e2e/run_all.sh — Run all E2E test targets with structured logging + artifacts.
#
# Usage:
#   ./scripts/e2e/run_all.sh                  # run all suites
#   ./scripts/e2e/run_all.sh --suite e2e_tui  # run specific suite
#   ./scripts/e2e/run_all.sh --list           # list available suites
#
# Environment:
#   E2E_ARTIFACT_DIR   Override artifact output directory (default: tests/e2e_results/<timestamp>)
#   E2E_PARALLELISM    Cargo test threads (default: 1 for determinism)
#   RUST_LOG           Log level for test harness (default: info)
#   VCR_MODE           Override VCR mode for all suites (default: unset, per-test decision)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ─── Configuration ────────────────────────────────────────────────────────────

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="${E2E_ARTIFACT_DIR:-$PROJECT_ROOT/tests/e2e_results/$TIMESTAMP}"
PARALLELISM="${E2E_PARALLELISM:-1}"
LOG_LEVEL="${RUST_LOG:-info}"

# All known E2E test targets (discovered from tests/e2e_*.rs).
ALL_SUITES=(
    e2e_agent_loop
    e2e_cli
    e2e_cross_provider_parity
    e2e_extension_registration
    e2e_library_integration
    e2e_live
    e2e_live_harness
    e2e_message_session_control
    e2e_provider_streaming
    e2e_rpc
    e2e_session_persistence
    e2e_tools
    e2e_ts_extension_loading
    e2e_tui
)

# ─── CLI Parsing ──────────────────────────────────────────────────────────────

SELECTED_SUITES=()
LIST_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --suite)
            shift
            SELECTED_SUITES+=("$1")
            shift
            ;;
        --list)
            LIST_ONLY=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--suite NAME]... [--list] [--help]"
            echo ""
            echo "Options:"
            echo "  --suite NAME   Run only the specified suite (repeatable)"
            echo "  --list         List available suites and exit"
            echo "  --help         Show this help"
            echo ""
            echo "Environment:"
            echo "  E2E_ARTIFACT_DIR   Artifact output directory"
            echo "  E2E_PARALLELISM    Cargo test threads (default: 1)"
            echo "  RUST_LOG           Log level (default: info)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

if $LIST_ONLY; then
    echo "Available E2E suites:"
    for suite in "${ALL_SUITES[@]}"; do
        if [[ -f "tests/${suite}.rs" ]]; then
            echo "  $suite"
        else
            echo "  $suite (missing)"
        fi
    done
    exit 0
fi

if [[ ${#SELECTED_SUITES[@]} -eq 0 ]]; then
    SELECTED_SUITES=("${ALL_SUITES[@]}")
fi

# ─── Environment Capture ─────────────────────────────────────────────────────

mkdir -p "$ARTIFACT_DIR"

capture_env() {
    local env_file="$ARTIFACT_DIR/environment.json"
    local rustc_version cargo_version os_info git_sha git_branch
    rustc_version="$(rustc --version 2>/dev/null || echo 'unknown')"
    cargo_version="$(cargo --version 2>/dev/null || echo 'unknown')"
    os_info="$(uname -srm 2>/dev/null || echo 'unknown')"
    git_sha="$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
    git_branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"

    cat > "$env_file" <<ENVJSON
{
  "timestamp": "$TIMESTAMP",
  "rustc": "$rustc_version",
  "cargo": "$cargo_version",
  "os": "$os_info",
  "git_sha": "$git_sha",
  "git_branch": "$git_branch",
  "parallelism": $PARALLELISM,
  "log_level": "$LOG_LEVEL",
  "artifact_dir": "$ARTIFACT_DIR",
  "vcr_mode": "${VCR_MODE:-unset}"
}
ENVJSON
    echo "[env] Captured environment to $env_file"
}

# ─── Build First ──────────────────────────────────────────────────────────────

build_tests() {
    echo "[build] Compiling selected test targets..."
    local build_log="$ARTIFACT_DIR/build.log"
    local build_ok=true
    for suite in "${SELECTED_SUITES[@]}"; do
        if [[ ! -f "tests/${suite}.rs" ]]; then
            continue
        fi
        echo "[build]   $suite"
        if ! cargo test --test "$suite" --no-run 2>>"$build_log"; then
            echo "[build]   $suite FAILED" >&2
            build_ok=false
        fi
    done
    if $build_ok; then
        echo "[build] OK"
        return 0
    else
        echo "[build] Some targets failed — see $build_log" >&2
        return 1
    fi
}

# ─── Run a Single Suite ──────────────────────────────────────────────────────

run_suite() {
    local suite="$1"
    local suite_dir="$ARTIFACT_DIR/$suite"
    local log_file="$suite_dir/output.log"
    local result_file="$suite_dir/result.json"
    local start_epoch exit_code duration_ms

    mkdir -p "$suite_dir"

    echo "[suite] Running: $suite"

    start_epoch=$(date +%s%N 2>/dev/null || date +%s)

    # Set per-suite environment for test harness logging.
    export TEST_LOG_JSONL_PATH="$suite_dir/test-log.jsonl"
    export TEST_ARTIFACT_INDEX_PATH="$suite_dir/artifact-index.jsonl"
    export RUST_LOG="$LOG_LEVEL"

    set +e
    cargo test \
        --test "$suite" \
        -- \
        --test-threads="$PARALLELISM" \
        2>&1 | tee "$log_file"
    exit_code=${PIPESTATUS[0]}
    set -e

    local end_epoch
    end_epoch=$(date +%s%N 2>/dev/null || date +%s)

    # Compute duration (nanosecond precision if available, else seconds).
    if [[ ${#start_epoch} -gt 12 ]]; then
        duration_ms=$(( (end_epoch - start_epoch) / 1000000 ))
    else
        duration_ms=$(( (end_epoch - start_epoch) * 1000 ))
    fi

    # Parse test counts from cargo test output.
    local passed failed ignored total
    passed=$(grep -oP '\d+ passed' "$log_file" | tail -1 | grep -oP '\d+' || echo "0")
    failed=$(grep -oP '\d+ failed' "$log_file" | tail -1 | grep -oP '\d+' || echo "0")
    ignored=$(grep -oP '\d+ ignored' "$log_file" | tail -1 | grep -oP '\d+' || echo "0")
    total=$((passed + failed + ignored))

    cat > "$result_file" <<RESULTJSON
{
  "suite": "$suite",
  "exit_code": $exit_code,
  "duration_ms": $duration_ms,
  "passed": $passed,
  "failed": $failed,
  "ignored": $ignored,
  "total": $total,
  "log_file": "$log_file",
  "timestamp": "$TIMESTAMP"
}
RESULTJSON

    if [[ $exit_code -eq 0 ]]; then
        echo "[suite] $suite: PASS ($passed passed, $ignored ignored, ${duration_ms}ms)"
    else
        echo "[suite] $suite: FAIL (exit $exit_code, $failed failed, $passed passed, ${duration_ms}ms)"
        # Emit failure triage hints.
        echo "[triage] Logs: $log_file"
        echo "[triage] Artifacts: $suite_dir/"
        if [[ -f "$suite_dir/test-log.jsonl" ]]; then
            echo "[triage] JSONL log: $suite_dir/test-log.jsonl"
        fi
    fi

    return $exit_code
}

# ─── Summary Manifest ────────────────────────────────────────────────────────

write_summary() {
    local summary_file="$ARTIFACT_DIR/summary.json"
    local total_suites=${#SELECTED_SUITES[@]}
    local passed_suites=0
    local failed_suites=0
    local failed_names=()

    echo "[summary] Writing manifest to $summary_file"

    # Read results from each suite.
    local results_array="["
    local first=true
    for suite in "${SELECTED_SUITES[@]}"; do
        local result_file="$ARTIFACT_DIR/$suite/result.json"
        if [[ -f "$result_file" ]]; then
            local exit_code
            exit_code=$(python3 -c "import json; print(json.load(open('$result_file'))['exit_code'])" 2>/dev/null || echo "1")
            if [[ "$exit_code" -eq 0 ]]; then
                ((passed_suites++)) || true
            else
                ((failed_suites++)) || true
                failed_names+=("$suite")
            fi
            if ! $first; then results_array+=","; fi
            results_array+="$(cat "$result_file")"
            first=false
        else
            ((failed_suites++)) || true
            failed_names+=("$suite")
            if ! $first; then results_array+=","; fi
            results_array+="{\"suite\":\"$suite\",\"exit_code\":1,\"error\":\"no result file\"}"
            first=false
        fi
    done
    results_array+="]"

    # Redact secrets from logs.
    redact_secrets

    cat > "$summary_file" <<SUMMARYJSON
{
  "timestamp": "$TIMESTAMP",
  "artifact_dir": "$ARTIFACT_DIR",
  "total_suites": $total_suites,
  "passed_suites": $passed_suites,
  "failed_suites": $failed_suites,
  "failed_names": $(printf '%s\n' "${failed_names[@]:-}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))' 2>/dev/null || echo '[]'),
  "suites": $results_array
}
SUMMARYJSON

    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo " E2E Summary: $passed_suites/$total_suites passed"
    if [[ $failed_suites -gt 0 ]]; then
        echo " Failed: ${failed_names[*]}"
    fi
    echo " Artifacts: $ARTIFACT_DIR"
    echo "═══════════════════════════════════════════════════════════════"
}

# ─── Secret Redaction ─────────────────────────────────────────────────────────

redact_secrets() {
    # Redact common API key patterns from all log files.
    local patterns=(
        's/sk-[a-zA-Z0-9_-]{20,}/sk-REDACTED/g'
        's/key-[a-zA-Z0-9_-]{20,}/key-REDACTED/g'
        's/ANTHROPIC_API_KEY=[^ ]*/ANTHROPIC_API_KEY=REDACTED/g'
        's/OPENAI_API_KEY=[^ ]*/OPENAI_API_KEY=REDACTED/g'
        's/GOOGLE_API_KEY=[^ ]*/GOOGLE_API_KEY=REDACTED/g'
        's/AZURE_OPENAI_API_KEY=[^ ]*/AZURE_OPENAI_API_KEY=REDACTED/g'
    )

    local sed_args=()
    for pattern in "${patterns[@]}"; do
        sed_args+=(-e "$pattern")
    done

    # Find all log/jsonl files and redact in-place.
    find "$ARTIFACT_DIR" -type f \( -name "*.log" -o -name "*.jsonl" \) -print0 | \
        xargs -0 -r sed -i "${sed_args[@]}" 2>/dev/null || true
}

# ─── Main ─────────────────────────────────────────────────────────────────────

main() {
    echo "═══════════════════════════════════════════════════════════════"
    echo " Pi Agent Rust — E2E Test Runner"
    echo " Timestamp: $TIMESTAMP"
    echo " Artifact dir: $ARTIFACT_DIR"
    echo " Suites: ${#SELECTED_SUITES[@]}"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""

    capture_env

    if ! build_tests; then
        echo "[fatal] Build failed, aborting E2E run." >&2
        exit 1
    fi

    local overall_exit=0
    for suite in "${SELECTED_SUITES[@]}"; do
        if [[ ! -f "tests/${suite}.rs" ]]; then
            echo "[skip] $suite: test file not found"
            continue
        fi
        if ! run_suite "$suite"; then
            overall_exit=1
        fi
    done

    write_summary

    exit $overall_exit
}

main "$@"
