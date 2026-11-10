#!/usr/bin/env bash
# On-demand DMN TCK run: the official TCK (github.com/dmn-tck/tck) plus rheon's own tests in
# tck-addendum/ (DMN 1.6 features the TCK does not cover yet), against the dsntk server of this
# repository, with DecisionToolkit's runner. Not part of CI (see docs/FORK.md, "TCK").
#
#   scripts/tck.sh                 # patched official TCK + addendum
#   scripts/tck.sh --unpatched     # without dsntk-tck-patches
#   scripts/tck.sh --addendum-only # only tck-addendum/
#   scripts/tck.sh --refresh       # git pull the cached clones first
#
# Clones live in .tck/ (gitignored). Results: .tck/out/results.csv and a summary on stdout.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
CACHE=$ROOT/.tck
PORT=${TCK_PORT:-22022}
PATCHED=1 OFFICIAL=1 REFRESH=0
for a in "$@"; do case $a in
  --unpatched) PATCHED=0 ;; --addendum-only) OFFICIAL=0 ;; --refresh) REFRESH=1 ;;
  *) echo "unknown option $a" >&2; exit 2 ;; esac; done
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
mkdir -p "$CACHE"

clone() { # url dir
  if [ ! -d "$CACHE/$2/.git" ]; then git clone -q --depth 1 "$1" "$CACHE/$2";
  elif [ $REFRESH = 1 ]; then git -C "$CACHE/$2" pull -q --ff-only; fi
}
[ $OFFICIAL = 1 ] && clone https://github.com/dmn-tck/tck.git tck
[ $OFFICIAL = 1 ] && [ $PATCHED = 1 ] && clone https://github.com/DecisionToolkit/dsntk-tck-patches.git tck-patches
clone https://github.com/DecisionToolkit/dsntk-test-runner.git runner

# the runner's report formatter underflows on long paths ("capacity overflow"); make it saturate
grep -q 'GUTTER - 21 - ' "$CACHE/runner/src/"*.rs 2>/dev/null &&
  sed -i.bak -E 's/GUTTER - 21 - ([a-z_.()]+)/GUTTER.saturating_sub(21).saturating_sub(\1)/g' "$CACHE/runner/src/"*.rs
echo "== building the runner and the server (release)"
(cd "$CACHE/runner" && cargo build -q --release)
cargo build -q --release --features tck -p dsntk

# one test tree: official compliance levels (optionally patched) + the addendum
RUN=$CACHE/run
rm -rf "$RUN" && mkdir -p "$RUN/TestCases"
if [ $OFFICIAL = 1 ]; then
  # dsntk-tck-patches works in the clone itself (git restore, then git apply)
  if [ $PATCHED = 1 ]; then (cd "$CACHE/tck-patches" && bash scripts/apply-patches.sh "$CACHE/tck") >/dev/null
  else git -C "$CACHE/tck" restore . ; fi
  cp -R "$CACHE/tck/TestCases/compliance-level-2" "$CACHE/tck/TestCases/compliance-level-3" "$RUN/TestCases/"
fi
mkdir -p "$RUN/TestCases/compliance-level-3"
cp -R tck-addendum/TestCases/compliance-level-3/rheon-* "$RUN/TestCases/compliance-level-3/" 2>/dev/null || true

mkdir -p "$CACHE/out"
cat > "$CACHE/out/config.yml" <<YML
test_cases_dir_path: $RUN/TestCases
file_search_pattern: compliance-level-
evaluate_url: http://127.0.0.1:$PORT/tck
report_file: $CACHE/out/results.csv
tck_report_file: $CACHE/out/tck_results.csv
stop_on_failure: false
YML

./target/release/dsntk srv -H 127.0.0.1 -P "$PORT" -D "$RUN/TestCases" > "$CACHE/out/server.log" 2>&1 &
SRV=$!
trap 'kill $SRV 2>/dev/null || true' EXIT
for _ in $(seq 1 120); do curl -s -o /dev/null "http://127.0.0.1:$PORT/" && break; sleep 1; done
echo "== running"
(cd "$CACHE/out" && "$CACHE/runner/target/release/dsntk-test-runner" config.yml > runner.log 2>&1) || true
node "$ROOT/scripts/tck-summary.mjs" "$CACHE/out/tck_results.csv"
