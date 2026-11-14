#!/usr/bin/env bash
# Identity check of full responses: every committed request source goes through the reference
# module (the previous release, see fetch-baseline.sh) and the candidate, and every answer must be
# identical except the differences registered in registered/ (see README.md).
#
#   tools/identity/run.sh <reference.wasm> <candidate.wasm> [source…] [equal.mjs option…]
#
# Further sources (request files or directories) and options such as --record are passed on to
# equal.mjs. Exit status as equal.mjs: 0 identical or registered, 1 a failure, 2 a usage error.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
if [ $# -lt 2 ]; then
  echo "usage: $0 <reference.wasm> <candidate.wasm> [source…] [equal.mjs option…]" >&2
  exit 2
fi
reference=$1 candidate=$2
shift 2
sources=("$root/wasm/feel-wasm/tests/corpus/no-trap.json")
node "$here/equal.mjs" "$reference" "$candidate" --parse-too "${sources[@]}" --ledger "$here/registered" "$@"
