# Identity of full responses against the previous release

The released module is the frozen oracle of today's behaviour. Every request of the committed
sources goes through it and through a candidate build, and every full response (result, type,
warnings, diagnostics, errors, traps) must be identical, except the differences registered in
`registered/`. A change that alters an answer therefore names it, with its basis and both answers,
in the commit that makes it.

```sh
bash tools/identity/fetch-baseline.sh               # .tools/baseline/feel.wasm, the release in baseline.json
npm run build:wasm                                  # dist/feel.wasm, the candidate
bash tools/identity/run.sh .tools/baseline/feel.wasm dist/feel.wasm [source…] [option…]
```

| File | What it is |
|---|---|
| `run.sh <reference.wasm> <candidate.wasm> [source…] [option…]` | Runs `equal.mjs` with every committed source and ledger and the cache in `.tools/identity-cache/`; further sources and options are passed on |
| `equal.mjs` | The comparison (Node 21 or later, no dependencies); its header lists the options |
| `test.mjs` | The tool's own checks (`npm run test:identity`), on `IDENTITY_REFERENCE` (default `.tools/baseline/feel.wasm`) and `IDENTITY_CANDIDATE` (default `dist/feel.wasm`); a missing module fails them |
| `baseline.json` | `tag` and `sha256` of the previous release's `feel.wasm` |
| `fetch-baseline.sh` | Downloads that asset with the GitHub CLI into `.tools/baseline/` and verifies the sum; fails closed. A file already there with the right sum is kept |
| `requests/*.json` | Curated request sets. `temporal.json`: fractional seconds of 1 to 9 digits and beyond in every literal form, exact constructor paths and carries, the range edges and year forms, daylight saving gaps and overlaps, zones and offsets. They hold no expected answers: the previous release is the oracle |
| `registered/<change>.json` | The ledger: one file per change that alters answers, written in the commit that makes it |

CI runs the first and the last command after `npm run build:wasm` (`.github/workflows/ci.yml`, job
`wasm`), with `GH_TOKEN` set to the workflow's token for the download, and then `npm run
test:identity`.

The committed request sources are `requests/` and `wasm/feel-wasm/tests/corpus/no-trap.json`. A
source is a JSON file holding one request or an array of requests, exactly as a host sends them, or
a directory of such files. Each request is sent byte for byte as it is written in its file, and so
is the `request` of a ledger entry. `run.sh` also sends every `evaluate` request as a `parse`
request, the same request with `op` set to `"parse"`. Requests that call `now()` or `today()` are
left out, because their answers depend on the clock.

JSON is read losslessly: a number keeps its source text in requests, responses and ledger
entries, so a number beyond the range or precision of a double (`1.5e-6176`, a 1,000-digit
integer) is sent, keyed and compared exactly as written. This needs Node 21 or later
(`JSON.rawJSON` and the source text of a parsed number); an older Node exits with status 2.

## Comparison rules

1. `elapsedMicros` and `engineCommit` are removed from every response, and `version` from `info`
   responses. A trap of the instance is the response `{"trap": true}`, and the next request runs on
   a fresh instance. The reference answers each source file (with the parse twins of its requests)
   and each ledger file on a fresh instance of its own, so that its answers to a file depend on
   the module and the file alone; a request that several files list takes the answer of the first.
   The candidate answers every request once, on one instance.
2. Every request is compared. A difference that is not registered fails the run.
3. Every ledger entry is run, whether or not a source lists its request. The reference must answer
   its `old` and the candidate its `new`; an entry that does not reproduce fails the run, so the
   ledger cannot go stale.
4. A request is registered at most once across all ledger files. Its key is its JSON with sorted
   object keys.
5. A response that is not JSON fails the run.
6. Exit status: 0 when every response is identical or registered, 1 on any failure, 2 on a usage or
   input error (including a malformed ledger).

## The cache of reference answers

The reference is a fixed module, so its answers to a file do not change until the file does.
`--cache <dir>` keeps them, one file per source file or ledger file, and `run.sh` passes
`--cache .tools/identity-cache` (git-ignored, under `.tools/`). The candidate is never cached.

- A cache file is `<dir>/<sha256 of the reference module>/<16 hex digits of the sha256 of the
  unit's absolute path>-<file name>.jsonl`: a header line, one line per request of the unit (the
  sha256 of the request text sent, and the answer as the canonical text of the response without
  the fields of rule 1, or `"trap": true`), and a trailer line with the sha256 of every byte before
  it.
- The cached answers are used only when the trailer's digest is right and the header equals the
  run's: the format, the reference's sha256, the sha256 of the unit file's bytes, the kind of unit
  (requests or ledger), `--parse-too`, the sha256 of `equal.mjs` and `scripts/feel-abi.mjs`, and
  Node's version, architecture and `execArgv`; the request hashes must also be the unit's requests
  in order. Anything else is a miss: the unit is evaluated and its file written again (to a
  temporary file, then renamed). An existing file that does not match is reported as
  `cache: <file> rejected (<reason>), evaluating`.
- A trap depends on the host as well as on the module (stack and memory limits), so every cached
  trap is evaluated again, on a fresh instance each. If it answers, the file is rejected and the
  unit evaluated in full. A ledger entry with `"old": {"trap": true}` is therefore proven on every
  run.
- A unit with a response that is not JSON is not cached; the run fails as without a cache.
- There is at most one file per unit path, and a new reference directory removes the directories
  of other references, so the cache holds no more than one file for each file a run has named
  with the current reference. It holds no engine state, and deleting it is always safe.
- The summary line ends with `reference: <n> answers from the cache, <m> evaluated`.
- `--verify-cache` evaluates the reference in full, compares every answer of a matching cache file
  with the live one, reports each difference as `CACHE MISMATCH <request>` (exit status 1) and
  writes the files again. Run it over every request set before a release.
- CI starts from a fresh checkout and restores no cache, so its run is uncached.

## Ledger entries

A ledger file is a JSON array of entries:

```jsonc
{
  "expression": "string(time(\"10:00:00.0157\"))",  // or "request": { the whole request }; "expression" stands for {"op": "evaluate", "expression": …}
  "why": "DMN 1.6 §10.3.2.3.4: …",                  // the clause or the documented rule the new answer follows
  "old": "10:00:00.015699999",                      // what the reference answers
  "new": "10:00:00.0157"                            // what the candidate answers
}
```

`old` and `new` take one of these forms, and `old` must differ from `new`:

- A string, in both: the result text, that is `result.display` without its JSON quotes when
  `result.type` is `string`. Both responses must succeed, and they must agree on `ok`,
  `result.type`, `warnings` and `diagnostics`.
- An object: the whole response, without the fields of rule 1.
- `{"trap": true}`: the instance traps.

`why` is timeless and self-contained: it states the basis, not a plan or a tracker. Draft a file
with `run.sh … --record <out.json> --why "<basis>"`, which writes the run's unregistered
differences as entries in the whole-response form; then check every `new` against the spec before
committing it.

## When a release is cut

The commit after the release tag moves `baseline.json` to the new release (`tag` and the `sha256`
of its `feel.wasm` asset) and deletes `registered/*.json`: the new release answers every request
as registered. Git history keeps the old ledger, and `docs/FORK.md` keeps the count of registered
differences per change.
