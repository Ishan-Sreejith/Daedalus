#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

PROGRAM_FILE="${1:-examples/test_jit_loop.fr}"
ITERATIONS="${2:-30}"
WARMUP="${3:-5}"

if [ ! -f "$PROGRAM_FILE" ]; then
  echo "Program not found: $PROGRAM_FILE"
  echo "Usage: ./benchmark_backends.sh <program.fr> [iterations] [warmup]"
  exit 1
fi

if ! [[ "$ITERATIONS" =~ ^[0-9]+$ ]] || [ "$ITERATIONS" -lt 1 ]; then
  echo "Iterations must be a positive integer."
  exit 1
fi

if ! [[ "$WARMUP" =~ ^[0-9]+$ ]]; then
  echo "Warmup must be a non-negative integer."
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for statistics output."
  exit 1
fi

echo "Building release binaries..."
cargo build --release >/dev/null
cargo build --release --manifest-path vm/Cargo.toml >/dev/null

JIT_CMD=(./target/release/fforge "$PROGRAM_FILE")
VM_CMD=(./target/release/forge --vm "$PROGRAM_FILE")
DIRECT_CMD=(./target/release/forge --rust "$PROGRAM_FILE")

sanitize_output() {
  sed '/^\[DEBUG\]/d;/^→/d;/^✓/d;/^Finished/d;/^$/d'
}

benchmark_cmd() {
  local name="$1"
  shift
  local cmd=("$@")
  local raw_file
  raw_file="$(mktemp)"

  for ((i = 0; i < WARMUP; i++)); do
    "${cmd[@]}" >/dev/null 2>&1
  done

  for ((i = 0; i < ITERATIONS; i++)); do
    local t0 t1
    t0="$(python3 -c 'import time; print(time.perf_counter_ns())')"
    "${cmd[@]}" >/dev/null 2>&1
    t1="$(python3 -c 'import time; print(time.perf_counter_ns())')"
    echo "$((t1 - t0))" >>"$raw_file"
  done

  python3 - "$name" "$raw_file" <<'PY'
import statistics
import sys

name = sys.argv[1]
path = sys.argv[2]
vals = [int(x.strip()) / 1_000_000 for x in open(path) if x.strip()]
vals.sort()

mean = statistics.fmean(vals)
median = statistics.median(vals)
stdev = statistics.pstdev(vals) if len(vals) > 1 else 0.0
p95_idx = min(len(vals) - 1, max(0, int(0.95 * len(vals)) - 1))
p95 = vals[p95_idx]
best = vals[0]
worst = vals[-1]

print(f"{name:8} mean={mean:8.3f}ms  median={median:8.3f}ms  p95={p95:8.3f}ms  best={best:8.3f}ms  worst={worst:8.3f}ms  stdev={stdev:8.3f}ms")
PY

  rm -f "$raw_file"
}

echo
echo "Verifying backend output parity for: $PROGRAM_FILE"
tmp_jit="$(mktemp)"
tmp_vm="$(mktemp)"
tmp_direct="$(mktemp)"

if ! "${JIT_CMD[@]}" >"$tmp_jit" 2>&1; then
  echo "Backend failed: JIT"
  cat "$tmp_jit"
  rm -f "$tmp_jit" "$tmp_vm" "$tmp_direct"
  exit 1
fi
if ! "${VM_CMD[@]}" >"$tmp_vm" 2>&1; then
  echo "Backend failed: VM"
  cat "$tmp_vm"
  rm -f "$tmp_jit" "$tmp_vm" "$tmp_direct"
  exit 1
fi
if ! "${DIRECT_CMD[@]}" >"$tmp_direct" 2>&1; then
  echo "Backend failed: Direct"
  cat "$tmp_direct"
  rm -f "$tmp_jit" "$tmp_vm" "$tmp_direct"
  exit 1
fi

jit_out="$(sanitize_output <"$tmp_jit")"
vm_out="$(sanitize_output <"$tmp_vm")"
direct_out="$(sanitize_output <"$tmp_direct")"
rm -f "$tmp_jit" "$tmp_vm" "$tmp_direct"

if [ "$jit_out" != "$vm_out" ]; then
  echo "Mismatch: JIT and VM outputs differ."
  diff -u <(echo "$jit_out") <(echo "$vm_out") || true
  exit 1
fi

if [ "$jit_out" != "$direct_out" ]; then
  echo "Mismatch: JIT and Direct outputs differ."
  diff -u <(echo "$jit_out") <(echo "$direct_out") || true
  exit 1
fi

echo "Output parity: OK"
echo
echo "Benchmarking ($ITERATIONS measured + $WARMUP warmup runs):"
benchmark_cmd "JIT" "${JIT_CMD[@]}"
benchmark_cmd "VM" "${VM_CMD[@]}"
benchmark_cmd "Direct" "${DIRECT_CMD[@]}"
