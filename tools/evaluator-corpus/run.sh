#!/bin/bash
# FEEL-level differential run: every corpus through feel-eval built with `intel` and with `dashu`,
# then compare.py (Display/Debug mismatches, panics, time) and oracle_feel.py (who is right).
#
# usage: tools/evaluator-corpus/run.sh [corpus ...]   (default: corpus gen extra dmn)
# env:   DFP_NUMBER_SYS=<path to the patched dfp-number-sys crate> (default: tools/dfp-number-sys, vendored in this branch)
# Output: runs/<timestamp>/<corpus>-{intel,dashu}.jsonl, <corpus>-diff.txt, <corpus>-oracle.txt
set -euo pipefail
cd "$(dirname "$0")"
DFP=${DFP_NUMBER_SYS:-$(cd .. && pwd)/dfp-number-sys}
CARGO_TARGET_DIR=target/intel cargo build --release --no-default-features --features intel --config "patch.crates-io.dfp-number-sys.path=\"$DFP\""
CARGO_TARGET_DIR=target/dashu cargo build --release --no-default-features --features dashu
OUT=runs/$(date +%Y%m%d-%H%M%S)
mkdir -p "$OUT"
for C in ${@:-corpus gen extra dmn}; do
  for B in intel dashu; do
    /usr/bin/time -p target/$B/release/feel-eval < corpora/$C.jsonl > $OUT/$C-$B.jsonl 2> $OUT/$C-$B.err
  done
  python3 -I compare.py $OUT/$C-intel.jsonl $OUT/$C-dashu.jsonl > $OUT/$C-diff.txt
  python3 -I oracle_feel.py $OUT/$C-intel.jsonl $OUT/$C-dashu.jsonl > $OUT/$C-oracle.txt 2>&1 || true
  echo "== $C ($(wc -l < corpora/$C.jsonl) expressions; intel $(grep real $OUT/$C-intel.err | cut -d' ' -f2)s, dashu $(grep real $OUT/$C-dashu.err | cut -d' ' -f2)s)"
  head -1 $OUT/$C-diff.txt; grep '^panics' $OUT/$C-diff.txt; tail -1 $OUT/$C-oracle.txt
done
echo "output in $(pwd)/$OUT"
