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
| `run.sh <reference.wasm> <candidate.wasm> [source…] [option…]` | Runs `equal.mjs` with every committed source and ledger; further sources and options are passed on |
| `equal.mjs` | The comparison (Node 21 or later, no dependencies); its header lists the options |
| `baseline.json` | `tag` and `sha256` of the previous release's `feel.wasm` |
| `fetch-baseline.sh` | Downloads that asset with the GitHub CLI into `.tools/baseline/` and verifies the sum; fails closed. A file already there with the right sum is kept |
| `requests/*.json` | Curated request sets. `temporal.json`: fractional seconds of 1 to 9 digits and beyond in every literal form, exact constructor paths and carries, the range edges and year forms, daylight saving gaps and overlaps, zones and offsets. They hold no expected answers: the previous release is the oracle |
| `registered/<change>.json` | The ledger: one file per change that alters answers, written in the commit that makes it |

CI runs the first and the last command after `npm run build:wasm` (`.github/workflows/ci.yml`, job
`wasm`), with `GH_TOKEN` set to the workflow's token for the download.

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
   a fresh instance.
2. Every request is compared. A difference that is not registered fails the run.
3. Every ledger entry is run, whether or not a source lists its request. The reference must answer
   its `old` and the candidate its `new`; an entry that does not reproduce fails the run, so the
   ledger cannot go stale.
4. A request is registered at most once across all ledger files. Its key is its JSON with sorted
   object keys.
5. A response that is not JSON fails the run.
6. Exit status: 0 when every response is identical or registered, 1 on any failure, 2 on a usage or
   input error (including a malformed ledger).

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
