# Backlog: deferred work and open design questions

Things we have decided **not** to do yet, with enough context to pick them up later. Each entry
says what, why, what would trigger it, and a rough size. Nothing here is a commitment. When the
repository is on GitHub, entries that get picked up should become issues and be removed from
here; this file then keeps only the design-level questions.

Conventions: `[contract]` changes the wasm ABI or JSON contract (docs/CONTRACT.md, affects every
host); `[runtime]` is packages/feel-runtime; `[dotnet]` the future .NET package; `[playground]`
apps/playground; `[fork]` the dsntk-rs `wasm` branch; `[upstream]` something to raise with
DecisionToolkit/dsntk rather than carry ourselves. Dates are when the item was written.

## Contract

- **Contract version** `[contract]` (2026-10-17). `info` reports commits and the number backend but
  no contract version. Add `contract: 1` so hosts can refuse a mismatched module. Do it together
  with the first other contract change, so there is one bump. Hours.
- **Batch op** `[contract]` (2026-10-18). `batch` takes a list of requests and returns a list of
  responses in one call. Trigger: any bulk use (grid of expressions, validating a model, bursts of
  small evaluations). Why: a worker round trip costs 50–100 µs in the browser and a host-to-wasm
  transition in .NET, while a typical evaluation costs 10–20 µs, so for bursts messaging dominates
  and batching beats pooling. Half a day in Rust plus host API.
- **Handles for DMN** `[contract]` (2026-10-17). Stateless one-shot calls re-parse everything. DMN
  needs `load` → handle, `evaluate` by handle, `unload`; a trap or restart invalidates all handles.
  Hosts must replay loads after a worker/store restart ("sticky init" in the runtime's spawn path
  and in the .NET pool). Design this before the .NET API ships so the C# surface does not change.
  About a day plus host work.
- **Result field selection** `[contract]` (2026-10-17). Every result is serialised three ways
  (`display`, `feel`, `typed`). An optional `fields` list lets a server ask for `typed` only.
  Trigger: DMN-sized outputs. Small.
- **ABI out-parameters** `[contract]` (2026-10-17). `feel_call` returns `(ptr << 32) | len`, which
  ties the ABI to wasm32 and forbids a native cdylib. `feel_call(ptr, len, out_ptr, out_len) ->
  status` would also let hosts see "error response" without parsing. Only with a contract bump.
  Hours.
- **Host obligations section** `[contract]` (2026-10-17). CONTRACT §4 is JS-specific. Promote the
  host-neutral duties (tz offset default, `rawJson` splicing, lone-surrogate classification,
  out-of-memory detection on stderr, error kinds `timeout`/`crash`/`cancelled`) to a "host
  obligations" section and move the JS API to the runtime README, so the .NET wrapper is written
  against a checklist. Half a day.
- **Shared test corpus** `[contract]` (2026-10-17). Rust contract tests and TS tests each hold
  their own cases; .NET would be a third copy. A language-neutral `tests/corpus/*.json` (request,
  expected response subset) run by all three suites. One day; do it before the .NET tests exist.

## Runtime (browser and Node)

- **Engine pool** `[runtime]` (2026-10-18). `FeelEnginePool`: N workers sharing one compiled module,
  least-busy dispatch, same `evaluate`/`parse` API. Works for browser and Node through the existing
  `WorkerLike` abstraction. Default stays one worker; the playground never needs a pool. About a day.
- **Backpressure** `[runtime]` (2026-10-18). The queue is unbounded and the timeout clock starts at
  execution, so a flood is accepted and served late. Add `maxQueue` and a deadline that includes
  queue wait. Small.
- **Pool memory budget** `[runtime]` `[dotnet]` (2026-10-18). Limits are per instance (512 MiB cap,
  recycle above 256 MiB). A pool of N has a worst case of N × cap; give pools a total budget and a
  lower per-instance cap on servers and in browser tabs. Small once the pool exists.
- **Ship the wasm inside the runtime package** `[runtime]` (2026-10-17). The playground imports
  `../../../dist/feel.wasm`; the npm package's `files` list does not include the module, so a
  consumer gets a runtime without an engine. Either add the wasm to `@flowstone/feel-runtime` with
  a `./feel.wasm` export or create `@flowstone/feel-wasm` that runtime, playground, tests and the
  NuGet build all consume. Half a day.
- **Tighter types** `[runtime]` (2026-10-17). `ParseResponse` error kind is `string` while
  `EvaluateResponse` uses `ErrorKind`; `callJson` returns `any`. Small.
- **XState** `[runtime]` (2026-10-18). Not now: the worker supervisor is a hand-written state
  machine that is correct and covered by 33 tests, and the UI has only two-state request loops.
  Reconsider when DMN sticky init and pooling grow the supervisor's state space. Cost would be
  ~14 KB gzipped plus a second runtime dependency for embedders.

## .NET package (not started)

- **Wrapper** `[dotnet]` (2026-10-17). `netstandard2.0;net8.0`, LangVersion 10, Wasmtime 48.x, wasm
  as embedded resource, one engine + compiled module per process, a pool of stores (one per slot;
  a store is never used by two threads at once), epoch interruption for per-request deadlines,
  store limits for memory, trapped stores dropped and replaced. `string contextJson` overload as
  the core API; typed helpers only for temporal values. Tests on .NET 8 and 10 including corpus
  parity; the .NET Framework 4.8 run is a Windows task (verify the native asset copy there).
- **NuGet id and naming** `[dotnet]` (2026-10-17). Pick the package id together with the npm
  scope and crate names so `info.engine` and docs stay consistent.

## Playground

- **Latest-wins helper** `[playground]` (2026-10-18). Evaluate, parse and URL-hash sync each
  re-implement "debounce, cancel previous, keep newest". One helper used three times; also fixes
  the `pending` counter used as a boolean and the tree/result loops briefly showing output from
  different keystrokes. Half a day. This is the real fix the XState question pointed at.
- **Split App.svelte** `[playground]` (2026-10-17). 935 lines holding URL state, theme, two request
  loops, tree cursor logic and the tree panel. Extract `urlState`, `theme`, the request helper
  above and a `SyntaxTreePanel` component; merge the duplicated error-title tables. Half a day.
- **Undefined-name hint** `[playground]` (2026-10-17). dsntk reports an unknown name as a syntax
  error; suggest "did you mean to add it to the context?" when the lezer tree shows a bare name.
  Small.
- **Positions from dsntk** `[fork]` `[playground]` (2026-10-17). Syntax errors carry no position;
  the playground locates them with lezer-feel. Tiers: (1) byte offset on syntax errors, days;
  (2) a `tokenize` op; (3) node spans through the LALR value stack, 1–2 weeks. A tolerant
  Rust lexer crate with a CodeMirror adapter is the alternative if lezer-feel ever diverges.
  Decided 2026-10-20: the editor keeps lezer-feel/lang-feel for highlighting, completion and
  squiggle placement; dsntk's lexer is parser-driven (scope and mode flags decide what a word
  is), not tolerant and not incremental, so it is not a highlighting lexer and will not be made
  one. Only tier (1), a byte offset on syntax errors, is wanted from the engine.

## Fork: fixes we owe before any upstream PR

- **Unknown names swallow the following keyword** `[fork]` (2026-10-20). The lexer reads the
  longest run of words as one name and only cuts it back to a name known in scope. A path into
  an untyped value followed by a keyword (`for i in current(p).incomeItems return …`,
  `then i.amount else …`, `some id in item.ids satisfies …`) becomes one name including the
  keyword: a syntax error, upstream too (`for l in zz return l` is the smallest case). Legal per
  the FEEL grammar; hidden in DMN models because item definitions put every name in scope, so the
  TCK does not catch it. Fix: after the scope lookup fails, cut the name at the first reserved
  keyword (`return then else satisfies in and or between instance of for if some every function`)
  while keeping built-in multi-word names (`date and time`, `years and months duration`). Same
  function as the `in` panic below (`consume_name` in `feel-parser/src/lexer.rs`); do both together.
  Workaround until then: parenthesise the path, `for i in (current(p).incomeItems) return …`. Hours.
- **Lexer panic on `in` as a name part** `[fork]` (2026-10-18). `for in.x in [1] return x` indexes
  one position back (`feel-parser/src/lexer.rs:713`), a native panic and a wasm trap. Hours.
- **Decision-table bounds** `[fork]` (2026-10-18). Rules and outputs are indexed by clause position
  with no count check (`model-evaluator/src/decision_table.rs`): a short rule or a zero-output table
  panics. Count checks in the parser, `.get()` in evaluation. Hours.
- **Call-depth flag reset** `[fork]` (2026-10-18). The sticky `CALL_DEPTH_EXCEEDED` added by the
  robustness commit is reset only by our wasm layer; native CLI/server/model-evaluator stay poisoned
  after one runaway. Reset on entry to every public evaluate function. Half a day.
- **Nanosecond rule** `[fork]` (2026-10-18). Time/date-time equality drops nanoseconds, ordering
  (added in 4d61eb1) keeps them, so `=` and `<` can both hold. Decide once, document, test. Hours.
- **Grammar precedence for unary endpoints** `[fork]` `[upstream]` (2026-10-17). `> 0.1 + 0.2`
  parses as `(? > 0.1) + 0.2`; fix is in `feel-grammar/src/feel.y` plus Bison regeneration
  (bison 2.3 compatibility with the extractor unverified). Also cherry-pick upstream PR #112 for
  `? > 5` semantics.
- **Zoned `now()`** `[fork]` `[upstream]` (2026-10-17). `now()` is zone-less, so
  `now() - date and time("…Z")` is null. Proposal written, not applied.
- **Remaining `insert(0)` tails** `[fork]` (2026-10-18). Nine list-tail grammar actions are still
  quadratic; only the list literal was fixed. One shared helper. Small.
- **Double evaluation in `build_sub`** `[fork]` (2026-10-18). Operands re-run to format the null
  reason, so side effects happen twice. Trivial.
- **`sort` comparator bypasses guards** `[fork]` (2026-10-18). Depth and finiteness guards are
  skipped for the user comparator; non-boolean result silently means equal. Small.
- **Split `tools/` off the `wasm` branch** `[fork]` (2026-10-17). The vendored Intel reference is
  97 % of the branch diff by files; move it and docs/FORK.md to a `wasm-tools` branch so the
  branch is a clean PR series. An hour, on the next rebase.
- **Globals in the engine** `[fork]` `[upstream]` (2026-10-17). The tz-offset override is a process
  atomic and the call depth is thread-local; thread both through the evaluator scope and drop
  `ENGINE_LOCK`. A day; likely an upstream request anyway.

## TCK failures: root causes found 2026-10-18, deliberately not fixed (DMN layer)

The patched TCK stands at 3480/3511. Besides the Java cases (0076, out of scope by design), the
remaining failures are all in the DMN layer, which the fork has not touched, so they are parked
until DMN-in-wasm starts. Root causes are established, so each is a short task when picked up.

- **1157 implicit conversions (12 tests)** `[fork]`. Deployment fails with "empty FEEL type"
  because `model-evaluator/src/type_ref.rs` maps only the DMN-XML spellings (`dateTime`,
  `dayTimeDuration`, `yearMonthDuration`) and not the FEEL names (`date and time`, `days and time
  duration`, `years and months duration`) that DMN 1.3+ models use in `typeRef`. Decision output
  variables fall back to `Any` silently, but context-entry variables, BKM formal parameters and
  invocation bindings hard-error (bisected: the "Implicit Conversions CE" context and the FUNCT
  invocation are the two triggers). Second part: `Value::coerced` in `feel/src/values.rs` has no
  date → date-and-time rule (midnight UTC), which tests 003/006/009 need. Singleton-list
  conversions already work (`[[1], 2]` came out once the type resolved). Half a day plus a
  `dmn_3_1157.rs` compatibility test; the model must be added to the `examples` crate.
- **1160 multiple imports (3 tests)** `[fork]`. In `model-evaluator/src/decision.rs` required
  decisions keep their import name (`b.DecisionB` is created with `create_entries`), but required
  *inputs* drop it: `required_input_data_references` holds only `DefKey`s and the build-time
  context sets the plain variable name. With `<import name="a">`, `a.person.name` is null while
  `person.name` (empty import name, test 03) works. Fix: keep `Option<Name>` for required inputs,
  nest under the import name at build time and at evaluation time; the TCK binds the plain name
  (DMN16-50, one binding). Half a day plus `dmn_3_1160.rs` with the three models in `examples`.
- **0085 decisionService_015 (2 tests)** — not an engine bug. The engine returns the correct
  context with both output decisions. dsntk-test-runner evaluates the decision service once per
  `resultNode` and compares the whole context against each node's expected value instead of
  selecting the component named by the node. Fix belongs in DecisionToolkit/dsntk-test-runner
  (`main.rs` `execute_tests`): when `invocableName` is set and differs from the result node name,
  pick that component. Hours.

### FEEL-side cases the TCK patches remove

Of the 20 extra failures on the unpatched TCK, two FEEL ones were fixed on 2026-10-18 (fork
commits c324518 and 44049be: scale range -6111..=6176 for the rounding functions, and regex
back-references via fancy-regex). The unpatched subset now stands at 289/298. Remaining:
- **Dates beyond chrono's range** (1115/1117: years outside ±262143): a different date core or
  clamping with a null reason. Large; low value.
- **0036 typeRefs** is DMN-layer (item definition type references).

## DMN layer: before any DMN-in-wasm work

- **Cycle protection** `[fork]` (2026-10-18). Decision→decision and BKM→BKM cycles recurse until the
  stack traps; only item definitions are checked. Build a requirement graph at deploy (petgraph is
  already a dependency) and reject cycles. Half a day.
- **Evaluator leak** `[fork]` (2026-10-18). Decision services capture a strong `Arc` to their own
  evaluator; a host that reloads models never frees them. Use `Weak`. Hours.
- **Structured errors** `[fork]` (2026-10-18). Unknown invocables, dangling hrefs and unresolved
  types come back as FEEL null; "empty FEEL type" carries no name. Resolve import-prefixed typeRefs
  everywhere, fail deployment on dangling refs, return a result type that separates "model
  problem" from "null answer". 1–2 days.
- **Decision-table short-circuit and memoisation** `[fork]` (2026-10-18). Every cell of every rule
  is evaluated regardless of hit policy, and a diamond of required decisions re-evaluates shared
  nodes. A day each; only when DMN throughput matters.
- **Move `dsntk-examples` to a dev-dependency of `model`** `[fork]` (2026-10-18). 4 MB of TCK
  models embedded for tests only. An hour.

## Engine design conversations (upstream, not ours to carry)

- Share contexts/lists behind `Rc`/`Arc` instead of cloning on every scope lookup, function call
  and loop iteration (the main evaluation-performance lever).
- Split `Value` into FEEL values and evaluator intermediates (~20 of 45 variants are plumbing).
- One comparison helper instead of ~12 hand-written orderings; one null-reason format.
- A static table for built-in functions instead of two 90-arm dispatch matches.
- Hand-written recursive-descent parser if editor-grade error recovery is ever wanted; the LALR
  parser steers the lexer through flags and scope mutations, so recovery is impractical on it.
- A TCK-XML-driven test runner to replace 33k lines of hand-written compatibility tests.

## Licensing and repository

- **Rust-side third-party notices** (2026-10-18). The wasm embeds dsntk, dashu, chrono and
  other MIT/Apache crates; nothing reproduces their notices. `cargo-about` into the same
  notices file. An hour.
- **LICENSE files for this repository** (2026-10-18). Cargo.toml says MIT OR Apache-2.0 but there
  are no LICENSE-MIT / LICENSE-APACHE files.
- **First commit, fork push, https pin** (2026-10-17). dsntk-wasm has no commits and the fork is
  pinned by `file:///…`, so nobody else can build. Both wait for an explicit go-ahead. Once on
  GitHub, the six Cargo.toml `rev` lines switch to the https URL.
- **Report the fork fixes upstream** (2026-10-17). Candidates: c760d4d, 55c7406, 4d61eb1,
  f308a34, 5ecdede, cf21d52, plus the four "fixes we owe" above once done.


## Restructure of 2026-10-28 (flowstone)

The dsntk fork (`~/dev/dsntk-rs`, branch `wasm`) became the main repository `~/dev/flowstone`
(branch `main`, remote `upstream` = dsntk-rs) and absorbed the former `dsntk-wasm` workspace
(`wasm/feel-wasm`, `packages/`, `apps/`, `docs/`, `scripts/`). Items that came out of it:

- **Public build repository name** `[infra]`. The playground is still published to
  `spearsandshields/dsntk-wasm-build`; renaming it changes the demo URL, so it waits for the
  decision on the flowstone GitHub home. `scripts/deploy-public.sh` has the two values to edit.
- **DMN TCK in CI** `[infra]`. `.github/workflows/ci.yml` runs the engine suites, the wasm build
  and the JS side; the TCK (dsntk-test-runner + dsntk-tck-patches + the ~430 MB tck clone, see the
  procedure in the session notes) is still a manual run. A `workflow_dispatch` job is the plan.
- **Upstream's release workflows** (`.github/workflows/build-*.yml`) build dsntk binaries for five
  platforms on every push to `main`. Decide whether to keep, restrict or remove them.
- **DMN module** `[wasm]`. Same pipeline as `wasm/feel-wasm` pointed at `model-evaluator`
  (`wasm/dmn-wasm`, a decision-evaluate op in CONTRACT.md). Next milestone, together with the
  phase-1 lexer fixes from `docs/EDITOR-PLAN.md`.
- **GitHub home**. The `flowstone` handle belongs to another user; the repo goes under the
  owner's account or a suffixed organisation. Nothing is pushed yet.
- **`publish.sh`** (upstream's crates.io release script) is kept for merge friendliness; it must
  never be run from this repository.
