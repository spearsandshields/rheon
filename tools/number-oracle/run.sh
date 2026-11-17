#!/usr/bin/env bash
# The number oracle: every number result of a feel.wasm judged against its truth (see README.md).
#
#   tools/number-oracle/run.sh [quick|full] [module.wasm]     default: quick, dist/feel.wasm
#
# PYTHON names the interpreter (default python3); it needs the C decimal module and mpmath 1.3.0
# (requirements.txt). ORACLE_SEED changes the seed of the random parts (default 1). One Python
# process writes the requests, one node process answers them, one Python process judges.
# Exit status: 0 every case right or exempt, 1 a wrong answer, a stale exemption or an edge source
# that drifted, 2 usage or environment. The committed identity source
# tools/identity/requests/numbers.json must be the output of cases.py --edges.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
size=${1:-quick}
module=${2:-$root/dist/feel.wasm}
seed=${ORACLE_SEED:-1}
python=${PYTHON:-python3}
# ORACLE_JOBS: the shards of a run, each answered and judged by its own processes (default: 1 for
# quick; for full the processors, at most 8). The cases and the verdicts do not depend on it.
processors=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)
default_jobs=1
if [ "$size" = full ]; then
  default_jobs=$processors
  [ "$default_jobs" -gt 8 ] && default_jobs=8
  [ "$default_jobs" -lt 1 ] && default_jobs=1
fi
jobs=${ORACLE_JOBS:-$default_jobs}
if [ $# -gt 2 ] || { [ "$size" != quick ] && [ "$size" != full ]; } || ! [[ "$seed" =~ ^[0-9]+$ ]] || ! [[ "$jobs" =~ ^[1-9][0-9]*$ ]]; then
  echo "usage: $0 [quick|full] [module.wasm]  (ORACLE_SEED: digits; ORACLE_JOBS: a whole number of at least 1)" >&2
  exit 2
fi
if [ ! -f "$module" ]; then
  echo "number oracle: no module at $module" >&2
  exit 2
fi
if ! "$python" -I -B -c 'import _decimal, mpmath, sys; sys.exit(mpmath.__version__ != "1.3.0")' 2>/dev/null; then
  echo "number oracle: $python -I needs the C decimal module and mpmath 1.3.0 (tools/number-oracle/requirements.txt)" >&2
  exit 2
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$python" -I -B "$here/oracle.py" --self-test
# The edge vectors are also an identity source (tools/identity/requests/numbers.json); the two must not drift.
if ! "$python" -I -B "$here/cases.py" --edges | cmp -s - "$root/tools/identity/requests/numbers.json"; then
  echo "number oracle: tools/identity/requests/numbers.json is not the output of cases.py --edges; regenerate it:" >&2
  echo "  $python -I -B tools/number-oracle/cases.py --edges > tools/identity/requests/numbers.json" >&2
  exit 1
fi
"$python" -I -B "$here/cases.py" "$size" --seed "$seed" > "$work/requests.jsonl"
[ -n "${ORACLE_TIMES:-}" ] && echo "number oracle: cases written after $SECONDS s" >&2
if [ "$jobs" -le 1 ]; then
  set +e
  node --no-warnings "$root/scripts/feel-answers.mjs" "$module" "$work/requests.jsonl" - \
    | "$python" -I -B "$here/judge.py" "$size" --seed "$seed" - "$here/known-wrong.json"
  statuses=("${PIPESTATUS[@]}")
  set -e
  if [ "${statuses[0]}" -ne 0 ]; then
    echo "number oracle: the module did not answer every request (node exit ${statuses[0]})" >&2
    exit 2
  fi
  exit "${statuses[1]}"
fi
# In shards: request number k goes to shard k modulo $jobs. First every shard is answered, each by
# its own node process, then every shard is judged, each by its own Python process, and the
# parts are merged into the report of the whole run. (Answering into a pipe that the judge
# reads makes the two wait for each other: node writes to a pipe synchronously.)
parts=()
pids=()
for ((i = 0; i < jobs; i++)); do
  awk -v n="$jobs" -v i="$i" '(NR - 1) % n == i' "$work/requests.jsonl" > "$work/requests.$i.jsonl"
  node --no-warnings "$root/scripts/feel-answers.mjs" "$module" "$work/requests.$i.jsonl" "$work/answers.$i.jsonl" 2> "$work/answers.$i.err" &
  pids+=($!)
done
failed=0
for pid in "${pids[@]}"; do
  wait "$pid" || failed=1
done
if [ "$failed" -ne 0 ]; then
  cat "$work"/answers.*.err >&2
  echo "number oracle: the module did not answer every request" >&2
  exit 2
fi
[ -n "${ORACLE_TIMES:-}" ] && echo "number oracle: answered after $SECONDS s" >&2
pids=()
for ((i = 0; i < jobs; i++)); do
  parts+=("$work/part.$i.json")
  "$python" -I -B "$here/judge.py" --shard "$i/$jobs" --part "$work/part.$i.json" "$size" --seed "$seed" "$work/answers.$i.jsonl" "$here/known-wrong.json" &
  pids+=($!)
done
for pid in "${pids[@]}"; do
  wait "$pid" || failed=1
done
if [ "$failed" -ne 0 ]; then
  echo "number oracle: a shard was not judged" >&2
  exit 2
fi
cat "$work"/answers.*.err >&2
"$python" -I -B "$here/judge.py" --merge "$size" --seed "$seed" "$here/known-wrong.json" "${parts[@]}"
