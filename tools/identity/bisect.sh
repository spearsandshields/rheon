#!/usr/bin/env bash
# Finds the commit behind differences of the identity check, as the step of `git bisect run`:
#
#   git bisect start <bad commit> <good commit>
#   git bisect run <this script> <reference.wasm> <entries.json>
#
# <entries.json> is the --record draft of the failing run (run.sh … --record <entries.json> --why …),
# or a part of it, outside the work tree or untracked. Each step builds dist/feel.wasm of the
# checked-out tree (npm run build:wasm, in the work tree git bisect runs this script in) and checks
# it with the entries as the only ledger and no source, against the same reference:
#   - every entry reproduces (this commit already answers each `new`): exit 1, bad;
#   - some entry does not: exit 0, good;
#   - the build fails, or equal.mjs exits 2: exit 125, the commit is skipped.
# A usage error exits 128, which ends the bisect. With differences from several commits, the
# bisect ends at the last of them: cut the file to the entries that commit does not explain and
# bisect again.
#
# The equal.mjs next to this script is used, so a copy of tools/identity/ and scripts/feel-abi.mjs
# outside the work tree (the same layout) bisects commits that predate the script. The reference's
# answers to the entries are cached in IDENTITY_CACHE (default .tools/identity-cache/ of the work
# tree), so a step costs one build and the candidate's answers.
#
#   <this script> <reference.wasm> <entries.json> <candidate.wasm>
#
# checks the given module instead of building one.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ $# -lt 2 ] || [ $# -gt 3 ]; then
  echo "usage: $0 <reference.wasm> <entries.json> [candidate.wasm]" >&2
  exit 128
fi
reference=$1 entries=$2
tree="$(git rev-parse --show-toplevel 2>/dev/null)" || tree=$PWD
if [ $# -eq 3 ]; then
  candidate=$3
else
  if ! (cd "$tree" && npm run build:wasm >&2); then
    echo "bisect: the build failed, skipping $(git rev-parse --short HEAD 2>/dev/null)" >&2
    exit 125
  fi
  candidate=$tree/dist/feel.wasm
fi
node "$here/equal.mjs" "$reference" "$candidate" --ledger "$entries" --cache "${IDENTITY_CACHE:-$tree/.tools/identity-cache}"
case $? in
  0) echo "bisect: bad, every entry reproduces"; exit 1 ;;
  1) echo "bisect: good, not every entry reproduces"; exit 0 ;;
  *) echo "bisect: equal.mjs could not run, skipping" >&2; exit 125 ;;
esac
