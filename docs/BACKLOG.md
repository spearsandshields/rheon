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
  consumer gets a runtime without an engine. Either add the wasm to `@rheon/feel-runtime` with
  a `./feel.wasm` export or create `@rheon/feel-wasm` that runtime, playground, tests and the
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
  while keeping built-in multi-word names (`date and time`, `years and months duration`). The
  name is read by `consume_name` in `feel-parser/src/lexer.rs`.
  Workaround until then: parenthesise the path, `for i in (current(p).incomeItems) return …`. Hours.
- **Decision-table bounds** `[fork]` (2026-10-18). Rules and outputs are indexed by clause position
  with no count check (`model-evaluator/src/decision_table.rs`): a short rule or a zero-output table
  panics. Count checks in the parser, `.get()` in evaluation. Hours.
- **Call-depth flag reset** `[fork]` (2026-10-18). The sticky `CALL_DEPTH_EXCEEDED` added by the
  robustness commit is reset only by our wasm layer; native CLI/server/model-evaluator stay poisoned
  after one runaway. Reset on entry to every public evaluate function. Half a day.
- **Nanosecond rule** `[fork]` (2026-10-18). Time/date-time equality drops nanoseconds, ordering
  (added in 6f08eab) keeps them, so `=` and `<` can both hold. Decide once, document, test. Hours.
- **Grammar precedence for unary endpoints** `[fork]` `[upstream]` (2026-10-17). `> 0.1 + 0.2`
  parses as `(? > 0.1) + 0.2`; fix is in upstream's `feel-grammar/src/feel.y` (used from crates.io since 2026-11-09; bring it into the tree as `rheon-feel-grammar` first) plus Bison regeneration
  (bison 2.3 compatibility with the extractor unverified). Also cherry-pick upstream PR #112 for
  `? > 5` semantics.
- **Zoned `now()`** `[fork]` `[upstream]` (2026-10-17). `now()` is zone-less, so
  `now() - date and time("…Z")` is null. Proposal written, not applied.
- **`sort` comparator bypasses guards** `[fork]` (2026-10-18). Depth and finiteness guards are
  skipped for the user comparator; non-boolean result silently means equal. Small.
- **Globals in the engine** `[fork]` `[upstream]` (2026-10-17). The tz-offset override is a process
  atomic and the call depth is thread-local; thread both through the evaluator scope and drop
  `ENGINE_LOCK`. A day; likely an upstream request anyway.

## Lexer and evaluation limits (2026-11-13)

- **Iteration-variable mode outlives `item`** `[fork]`. Where the grammar expects an iteration
  variable, the lexer reads names in a mode that cuts the name before its `in` part; the mode
  ends only when the lexer reads a name with an `in` part (`consume_name`). A variable named
  `item` is returned before that check, so the mode stays on for the rest of the expression:
  `for item in[1] return item` is `[1]`, while `for x in[1] return x` is a syntax error, and
  with `"a in b": 5` in the context `for x in [1] return a in b` is `[5]`, while
  `for item in [1] return a in b` reads `a in b` as the `in` test (`[true]`).
- **`in` directly before `[`, `(` or a quote** `[fork]` is read as part of a name: with `x: "a"`
  in the context, `x in["a"]`, `x in("a")` and `x in"a"` are syntax errors, while `x in ["a"]` is
  `true`. The keyword `in` is recognised only when white space follows it.
- **Composite names** `[fork]`. A composite key `k1 . k2` (keys of nested contexts joined by
  ` . `) is a name in scope, as in upstream. A candidate name contains ` . ` only through a name
  part that is U+1680 alone (the only name character that trimming removes), and the lexer
  reads such a part only right after an additional name symbol (`.`, `/`, `-`, `'`, `+`, `*`).
  The composite search visits up to (nested contexts) × (separators in the candidate + 1)
  states per candidate.
- **An empty list in a `for` with a dependent variable** `[fork]`.
  `for x in [[1,2],[],[3]], y in x return y` is `[1, 2, 2, 3]`, not `[1, 2, 3]`: when the list of
  `y` is empty the iteration is not skipped, and `y` keeps its value from the iteration before.
  DMN 1.6 §10.3.2.14: an empty iteration context contributes no iteration.
  `feel-evaluator/src/tests/iterations.rs` `_0020` pins the sequence as it is.
- **Filters evaluate their expression once more in the outer scope** `[fork]`. After
  evaluating the expression for every item, the filter evaluates it once in the enclosing scope
  to decide between indexing and filtering (`FilterExpressionEvaluator::evaluate`). A predicate
  that names an item entry fails there: in strict mode `sum(items[amount > 100].amount)` is the
  error "context has no value for key 'amount'" (lenient mode returns the sum with two
  diagnostics). Indexing (`list[i]`) evaluates the index once per item as well.
- **Boxed contexts write into the context below them** `[fork]`. The boxed context evaluator
  (`build_context_evaluator`, `model-evaluator/src/boxed_expressions.rs`) pushes no context of
  its own when it evaluates: it writes each entry into the context on the top of the scope. The
  entries of a boxed context are therefore visible outside it, in an enclosing boxed context or
  boxed list: a boxed context with an entry `a`, whose value is a boxed context with the entry
  `b: 1`, and an entry `c: b` gives `c` the value 1 (DMN 1.6 §10.2.1.4: outside a boxed context
  its entries are reachable only through a qualified name, `a.b`). In a boxed `for` whose body
  is a boxed context with an entry named `partial`, that entry overwrites the iteration's
  `partial` and the results of the earlier iterations are lost (§10.3.2.14). Functions returned
  by boxed function bodies depend on these writes (*Closures are not lexical* below), so the two
  change together.
- **Values are cloned on lookup** `[fork]`. Function definitions are shared
  (`Arc<FunctionDefinition>`), and FEEL loops and filters move their contexts and items instead
  of cloning them. Contexts and lists are still cloned on every scope lookup, and a function's
  closure context on every call.
- **Loop variables are looked up by name** `[fork]`: every iteration resolves the variable
  through the scope stack (`FeelScope::get_value`).
- **Regular expressions are compiled on every call** `[fork]`: `matches` (`feel-regex`),
  `replace` and `split` compile their pattern for each invocation.

## DMN 1.6 (target since 2026-11-10)

The conformance target is DMN 1.6 (formal, September 2026), plus the purely additive FEEL
functions of the 1.7 beta (trim, intersection, insert after). Done 2026-11-10, each with Rust
tests and, where the TCK has none, TCK-format tests in `tck-addendum/` (see its MANIFEST):
1.6 namespace, single-argument rounding and number(from), the `value` property, singleton-list
conversion for filters and the boxed for, the descendant operator `e...name` (grammar change,
rheon-feel-grammar back in the tree), ONNX functions load, `date and time(date, time, timezone)`,
truncation of fractional list positions (a panic in `remove`), implicit conversions (TCK 1157)
and multiple imports (TCK 1160), B-FEEL, lenient and strict error modes with diagnostics, and the
FEEL fixes found on the way (`all`/`any` with a deciding item, numeric-literal check in number()).
Official TCK (patched): 3,392 of 3,407; only 0076 (Java, out of scope) and 0085 (runner) fail.
`scripts/tck.sh` runs it on demand; `docs/tck-baseline.txt` records the failing tests.

Open in 1.6:
- **B-FEEL arithmetic leaves a diagnostic** `[dmn]`. B-FEEL first applies the FEEL operator and
  converts the operands only when that fails, so `1 + "2"` in B-FEEL gives `"12"` with one
  recorded error ("incompatible types in addition"). In B-FEEL a null never means an error, so
  the FEEL attempt should not be recorded. Small: suppress collection around the first attempt.
- **Dialect of decision table entries** `[dmn]`. Input and output entries are unary tests and
  literal expressions that may carry their own `expressionLanguage`; the evaluator applies the
  model's dialect to the whole table. Not exercised by the TCK. Small.
- **Strict mode granularity** `[dmn]`. Strict evaluates to the end and then returns null with the
  first error (no side effects make the difference observable). Errors are recorded where the
  core creates a null with a reason (`value_null!`); a few places build nulls directly and are
  not recorded.
- **TCK conflicts** (tck-addendum/MANIFEST.md): 1151 006 and 1152 003/004 expect the DMN 1.5
  "no singleton conversion" rule for boxed iterators; rheon follows 1.6. Report to the TCK when
  it adds 1.6 tests.
- **DMN 1.7 beta** items deferred until 1.7 is formal: version-less namespace and the required
  `dmnVersion`, `InformationItem.defaultValue`, range types (`isRange`), iteration contexts in
  quantified expressions (`some x in 1..3`), B-FEEL null handling in collections (DMN17-112).
- **Spec typo**: the 1.6 number() footnote cites grammar rule 37 (`digits`); rheon uses rule 35.
- **Equality and comparison follow-ups** `[dmn]` (2026-11-12). Found while making null equality
  one rule (`eval_ternary_equality`) and `in` treat every null alike; each has its own cause:
  - *Ranges against unary comparisons.* `(< 10)` and the like compared with a range can give a
    wrong `true`, and the result depends on the operand order (the Unary* arms of
    `eval_ternary_equality`: inverted null check on the endpoint, wrong closure for `<=`).
  - *`is()` decides by its first argument.* `is(null, 1)` is null and `is(1, null)` is false;
    `is` never results in null (§10.3.2.2).
  - *`max` and `min` depend on the argument order* with nulls (max skips them after the first
    position, min never does), which carries into the C< and C> hit policies. Both reject dates,
    times and durations.
  - *Context equality depends on key order*: it returns null at the first pair it can not
    compare, even when a later pair differs. List equality treats such a pair as unequal.
  - *`!=` on values of different kinds* returns a null without a reason, so it records no
    diagnostic: strict mode stops on `"a" = 1` but not on `"a" != 1`.
  - *Lists of `!=` unary tests are combined with AND*; the spec defines a list of unary tests as
    OR (comparison_in.rs asserts AND). Needs a decision, as do `null <= null` (spec: `<` or `=`,
    so true) and `-` against a null input (rule 15.c reads as `? != null`); rheon gives null and
    a match.
  - *Smaller ones*: null range endpoints are handled differently by kind; `between` checks the
    kinds of all three operands first; `list in list` is containment of a multiset; `count(null)`
    is 1; the Any and Priority hit policies compare outputs with Rust equality, not FEEL
    equality; a collection item definition turns a list with a null item into null; a missing
    input has a null with a reason only when its typeRef is an item definition; the nulls of
    `eval_in_*` name internal functions.
- **Invocation follow-ups** `[dmn]` (2026-11-12). Every invocation now goes through
  `feel-evaluator/src/invocation.rs` (Table 64: FEEL and boxed invocations, invocables invoked
  with input data, `sort` and `list replace`). Left as they were:
  - *Closures are not lexical.* A function returned by a body closes over the names as they are
    where the body ends (`closed_over`, kept from dsntk), so `{f: function(x) {x: 10, g:
    function(y) x + y}.g, h: f(1), r: h(5)}.r` is 6, not 15 (§10.3.2.13.2: "FEEL functions are
    lexical closures"). Removing the rebuild needs a TCK run. A function returned by a boxed
    function body finds the entries of its boxed context through these writes (see *Boxed
    contexts write into the context below them* in "Lexer and evaluation limits"), so the two
    change together.
  - *Decision service as a function.* Its body reads the arguments with `scope.peek()`
    (decision_service.rs), so the invocation must push the parameters last, and it converts them
    a second time through the input data and input decision evaluators.
  - *Java and PMML BKMs are not external.* `build_bkm_evaluator` special-cases only ONNX; a BKM
    with `kind="Java"` or `"PMML"` returns its mapping context instead of calling the function.
  - *Binding types are not checked.* A binding's parameter `typeRef` is ignored (§7.3.6 binds by
    name); §7.3.4 asks it to be compatible with the value, so a model-load warning would fit.
  - *`try_coerced` lets any function value pass* whatever the target type (feel/src/values.rs).
  - *Call depth counting.* FEEL invocations are counted at the expression (argument evaluation
    included), boxed invocations and invocables in `evaluate_invocation`; `sort` and `list
    replace` callbacks are not counted.
  - *Unused values.* `Value::NamedParameter`, `Value::ParameterName` and
    `Value::PositionalParameters` are no longer built for invocations (`ParameterName` still is,
    for formal parameters).

- **0085 decisionService_015 (1 test since the TCK update)** — not an engine bug. The engine returns the correct
  context with both output decisions. dsntk-test-runner evaluates the decision service once per
  `resultNode` and compares the whole context against each node's expected value instead of
  selecting the component named by the node. Fix belongs in DecisionToolkit/dsntk-test-runner
  (`main.rs` `execute_tests`): when `invocableName` is set and differs from the result node name,
  pick that component. Hours.

### FEEL-side cases the TCK patches remove

Of the 20 extra failures on the unpatched TCK, two FEEL ones were fixed on 2026-10-18 (fork
commits 4b5ca51 and 69e81cc: scale range -6111..=6176 for the rounding functions, and regex
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

## Engine design conversations (upstream, not ours to carry)

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
- **Report the fork fixes upstream** (2026-10-17). Candidates: 8542135, 7c16676, 6f08eab,
  cb5997f, 58bbc18, f619d8a, plus the four "fixes we owe" above once done.


## Restructure of 2026-10-28 (rheon)

The dsntk fork (`~/dev/dsntk-rs`, branch `wasm`) became the main repository `~/dev/rheon`
(branch `main`, remote `upstream` = dsntk-rs) and absorbed the former `dsntk-wasm` workspace
(`wasm/feel-wasm`, `packages/`, `apps/`, `docs/`, `scripts/`). Items that came out of it:

- **Public demo** `[infra]` (2026-11-09, live): the `pages` job in `ci.yml` deploys the playground to
  `https://spearsandshields.github.io/rheon/` on every push to `main`. `scripts/deploy-public.sh` is
  gone; the old `spearsandshields/dsntk-wasm-build` repository still exists and can be deleted.
- **CI note** (2026-11-09): during the first half hour after the repository was created, every run
  whose `pages` job was eligible ended as "failure" with some or all jobs never created and no error
  shown anywhere (API, logs, check runs). Probes narrowed it to a job-level `if` reading `github.ref`,
  then the same probe passed unchanged twenty minutes later: a transient GitHub-side fault, not the
  workflow. The gate is now `github.event_name != 'pull_request'`, which is equivalent and simpler.
  If runs ever vanish like that again, re-dispatch before changing anything.
- **DMN TCK in CI** `[infra]`. `.github/workflows/ci.yml` runs the engine suites, the wasm build
  and the JS side; the TCK (dsntk-test-runner + dsntk-tck-patches + the ~430 MB tck clone, see the
  procedure in the session notes) is still a manual run. A `workflow_dispatch` job is the plan.
- **Upstream's release workflows** (`.github/workflows/build-*.yml`) build dsntk binaries for five
  platforms on every push to `main`. Decide whether to keep, restrict or remove them.
- **DMN module** `[wasm]`. Same pipeline as `wasm/feel-wasm` pointed at `model-evaluator`
  (`wasm/dmn-wasm`, a decision-evaluate op in CONTRACT.md). Next milestone, together with the
  phase-1 lexer fixes from `docs/EDITOR-PLAN.md`.
- **GitHub home** (decided 2026-11-09): `spearsandshields/rheon` under the owner's account; the
  manifests and the README point there. A transfer to an organisation later keeps working through
  GitHub's redirects. Nothing is pushed yet.
- **crates.io** (2026-11-09, done; user: "yes, do it for me... the ones we did not overwrite yet,
  can use those as the dependencies"). The eight `rheon-*` crates are published from the workspace
  (`cargo publish --workspace`, see `docs/FORK.md` "Publishing"); upstream's `publish.sh` is
  deleted. `dsntk-common`, `-macros`, `-feel-grammar` and `-recognizer` were identical to dsntk
  0.3.0 and now come from crates.io (pinned `=0.3.0`; their directories are gone). `model` is
  renamed `rheon-model` because it imports `rheon-feel` types. `dsntk-examples` (fork-adjusted
  TCK fixtures, tests only) is a dev-dependency of `model` now and, like the other `dsntk-*`
  members, `publish = false`. `rheon-feel-wasm` followed as the ninth crate once its build script learned to read `.cargo_vcs_info.json`.
- **Crate rename** (2026-10-28, done). The user decided on a permanent fork with no priority on
  upstream feedback, and that crates whose source rheon changed carry the `rheon-` prefix:
  `rheon-feel`, `-feel-number`, `-feel-parser`, `-feel-evaluator`, `-feel-temporal`,
  `-feel-regex`, `-model-evaluator` and, since 2026-11-09, `-model` (directories unchanged). The others
  keep `dsntk-*` until they are changed. Upstream fixes are taken by cherry-pick through
  `scripts/upstream-rename.pl` (see `docs/FORK.md`). The `dsntk` CLI and the other manifest-only crates keep
  their names and are `publish = false` (2026-11-09).
- **0.1.0 released** (2026-11-10): nine crates on crates.io, `@rheon/feel-runtime@0.1.0` on npm
  (`rheon` org on npm, 2FA on the account; `npm publish` needs an interactive prompt for the
  one-time code), GitHub Release `v0.1.0` from `release.yml` (`feel.wasm` + sha256, npm
  tarball, playground zip). Known wart: the npm 0.1.0 tarball's `feel.wasm` reports
  `engineCommit d811cac…-dirty` (pre-rewrite hash) because prepack copied a stale local build; the engine code is
  identical, the stamp is wrong. prepack now refuses a module not built from HEAD. Fixed in the
  next npm release; the release asset is stamped correctly (3fcc8af, the pre-rewrite hash of caf5ea4).
- **0.2.0 released** (2026-11-11, DMN 1.6): ten crates on crates.io (`rheon-feel-grammar` published
  for the first time; the whole set went up in 40 s, no rate limit), `@rheon/feel-runtime@0.2.0`
  on npm (user's prompt, 2FA), GitHub Release `v0.2.0` from `release.yml`. Preceded by a
  main-thread review of the 1.6 engine diff (b20412c: Drop guard in the diagnostics collector,
  no per-reference clone of the input data in decisions, one `variable::result_type`, dead code,
  `cargo fmt` over the tree) and `cargo fmt --check` in CI (rustfmt pinned in
  rust-toolchain.toml). The npm module is stamped `3209ccd` clean (prepack guard; the pre-rewrite hash of
  today's a91330d). The npm `feel.wasm` and the release asset are both built from that commit with the same rustc but differ
  by 310 bytes (string-constant order in the data section, a few bytes of code): cross-host
  builds (macOS vs CI Linux) are not bit-identical. Same behaviour; the release asset is the
  canonical build. Option for a later release: let CI publish npm too (automation token with
  provenance) so one build feeds both. Local upstream tags (`v0.0.x`, `v0.3.0`, `v0.3.0-rc.0`)
  deleted with the user's OK; GitHub keeps only the `dsntk/…` twins. The old
  `dsntk-wasm-build` repo: deletion approved by the user, waiting for a `gh` token with the
  `delete_repo` scope (no rush).
- **SECURITY.md / CONTRIBUTING.md** (2026-11-09): deliberately not added yet (user). Add SECURITY.md when
  the npm package or the server is something people deploy; until then GitHub's "private
  vulnerability reporting" toggle is the reporting channel. The `repository`/`homepage` fields of
  the workspace and the npm packages still name upstream; they follow the GitHub-home decision.
- **Upstream release workflows** (2026-11-09, done): `build-*.yml` now run on `workflow_dispatch`
  only, so a push to `main` no longer starts five dsntk binary builds.
