#!/bin/bash
# Op-by-op differential fuzz, Intel BID128 vs dashu, judged by the mpmath/decimal oracle.
#
# usage: tools/difftest/run.sh [iters_per_op] [only_op,...]
# env:   DFP_NUMBER_SYS=<path to the patched dfp-number-sys crate> (default: fetched by tools/fetch-dfp-number-sys.sh)
# Output: runs/<timestamp>/{run.txt,mismatches.tsv,cohort.tsv,oracle.txt,time.txt}
set -euo pipefail
cd "$(dirname "$0")"
DFP=${DFP_NUMBER_SYS:-$(../fetch-dfp-number-sys.sh)}
CFG=(--config "patch.crates-io.dfp-number-sys.path=\"$DFP\"")
cargo build --release "${CFG[@]}"
OUT=runs/$(date +%Y%m%d-%H%M%S)
mkdir -p "$OUT" && cd "$OUT"
echo "difftest ${1:-200000} iterations per op, seed base ${DIFFTEST_SEED:-default}, output in $(pwd)"
/usr/bin/time -p ../../target/release/difftest "${1:-200000}" ${2:+"$2"} > run.txt 2> time.txt
python3 -I ../../judge4.py mismatches.tsv 2 > oracle.txt
grep -E '^TOTAL|^PANIC' run.txt || true
grep -E '^(real|user)' time.txt
echo "cohort-only (Debug) differences: $(wc -l < cohort.tsv)"
echo "== oracle verdicts (count, op, verdict)"
sed -n '1,/^$/p' oracle.txt
