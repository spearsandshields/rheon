#!/bin/bash
# Publishes the rheon-* crates that crates.io does not have at the workspace version yet, in
# dependency order, sleeping until the time named by a 429 (new-crate rate limit) between
# attempts. Safe to re-run: crates already published are skipped. See docs/FORK.md, "Publishing".
cd "$(dirname "$0")/.." || exit 1
VERSION=$(grep -m1 "^version = " Cargo.toml | sed "s/.*\"\(.*\)\"/\1/")
export CARGO_BUILD_JOBS=4
ORDER="rheon-feel-number rheon-feel-regex rheon-feel-temporal rheon-feel rheon-feel-parser rheon-feel-evaluator rheon-model rheon-model-evaluator"
for attempt in 1 2 3 4 5 6 7 8; do
  echo "=== attempt $attempt $(date -u)"
  args=""
  for c in $ORDER; do
    v=$(curl -s -A rheon-publish https://crates.io/api/v1/crates/$c | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d["crate"]["max_version"] if "crate" in d else "")')
    [ "$v" = "$VERSION" ] || args="$args -p $c"
  done
  if [ -z "$args" ]; then echo "=== all published $(date -u)"; exit 0; fi
  echo "remaining:$args"
  out=$(cargo publish $args 2>&1); rc=$?
  echo "$out" | grep -E 'Uploaded|Published|error|Caused|try again|not been published|^\s+rheon-'
  if [ $rc -eq 0 ]; then echo "=== all published $(date -u)"; exit 0; fi
  when=$(echo "$out" | grep -oE 'try again after [^,]+, [0-9]{2} [A-Za-z]{3} [0-9]{4} [0-9:]{8} GMT' | head -1 | sed 's/try again after //')
  if [ -z "$when" ]; then echo "=== no retry time in output; stopping"; exit 1; fi
  wait=$(python3 -c "import sys,datetime,email.utils; t=email.utils.parsedate_to_datetime(sys.argv[1]); print(max(5,int((t-datetime.datetime.now(datetime.timezone.utc)).total_seconds())+10))" "$when")
  echo "=== rate-limited until $when; sleeping ${wait}s"
  sleep "$wait"
done
echo "=== gave up"; exit 1
