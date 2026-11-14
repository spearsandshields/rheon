#!/usr/bin/env bash
# The number oracle: every number result of a feel.wasm judged against its truth (see README.md).
#
#   tools/number-oracle/run.sh [quick|full] [module.wasm]     default: quick, dist/feel.wasm
#
# PYTHON names the interpreter (default python3); it needs the C decimal module and mpmath 1.3.0
# (requirements.txt). ORACLE_SEED changes the seed of the random parts (default 1). One Python
# process writes the requests, one node process answers them, one Python process judges.
# Exit status: 0 every case right or exempt, 1 a wrong answer or a stale exemption, 2 usage or
# environment.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
size=${1:-quick}
module=${2:-$root/dist/feel.wasm}
seed=${ORACLE_SEED:-1}
python=${PYTHON:-python3}
if [ $# -gt 2 ] || { [ "$size" != quick ] && [ "$size" != full ]; } || ! [[ "$seed" =~ ^[0-9]+$ ]]; then
  echo "usage: $0 [quick|full] [module.wasm]  (ORACLE_SEED: digits)" >&2
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
"$python" -I -B "$here/cases.py" "$size" --seed "$seed" > "$work/requests.jsonl"
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
