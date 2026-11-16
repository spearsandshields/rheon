#!/usr/bin/env bash
# Identity check of full responses: every committed request source goes through the reference
# module (the previous release, see fetch-baseline.sh) and the candidate, and every answer must be
# identical except the differences registered in registered/ (see README.md).
#
#   tools/identity/run.sh <reference.wasm> <candidate.wasm> [--release] [--large] [source…] [equal.mjs option…]
#
# The requests of release/requests/ cost the reference seconds each. They are left out of every
# source unless --release is given, which adds them and the ledger in release/registered/.
# --large adds the request sets kept outside the repository: the files or directories that
# IDENTITY_LARGE_SETS lists, separated by colons, without the requests that large-left-out.json
# names (see README.md). Further sources (request files or directories) and options such as
# --record are passed on to equal.mjs. The reference's answers are cached in .tools/identity-cache/
# (git-ignored; deleting it is always safe), and --verify-cache checks them against a full
# evaluation. Exit status as equal.mjs: 0 identical or registered, 1 a failure, 2 a usage error.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
usage() {
  echo "usage: $0 <reference.wasm> <candidate.wasm> [--release] [--large] [source…] [equal.mjs option…]" >&2
  exit 2
}
[ $# -ge 2 ] || usage
reference=$1 candidate=$2
shift 2
release=0 large=0 rest=()
for arg in "$@"; do
  case $arg in
    --release) release=1 ;;
    --large) large=1 ;;
    *) rest+=("$arg") ;;
  esac
done
sources=("$here/requests" "$root/wasm/feel-wasm/tests/corpus/no-trap.json")
ledgers=(--ledger "$here/registered")
if [ $release = 1 ]; then
  sources+=("$here/release/requests")
  ledgers+=(--ledger "$here/release/registered")
else
  ledgers+=(--exclude "$here/release/requests")
fi
if [ $large = 1 ]; then
  if [ -z "${IDENTITY_LARGE_SETS:-}" ]; then
    echo "$0: --large needs IDENTITY_LARGE_SETS, the large request sets separated by colons" >&2
    exit 2
  fi
  IFS=: read -r -a sets <<< "$IDENTITY_LARGE_SETS"
  sources+=("${sets[@]}")
  ledgers+=(--leave-out "$here/large-left-out.json")
fi
node "$here/equal.mjs" "$reference" "$candidate" --parse-too "${sources[@]}" "${ledgers[@]}" --cache "$root/.tools/identity-cache" ${rest[@]+"${rest[@]}"}
