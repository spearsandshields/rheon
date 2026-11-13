# Known bugs and limitations

Bugs and limitations of the engine, the wasm module, the runtime and the playground that are
known and not fixed, with their cause where it is known. Tags: `[contract]` the wasm ABI or JSON
contract (docs/CONTRACT.md, every host); `[runtime]` packages/feel-runtime; `[playground]`
apps/playground; `[fork]` the engine crates; `[dmn]` DMN conformance. Dates are when the entry
was written.

## Contract

- **No contract version** `[contract]` (2026-10-17). `info` reports commits and the number backend
  but no contract version, so a host cannot recognise a module built for another contract.
- **Every result is serialised three ways** `[contract]` (2026-10-17): `display`, `feel` and
  `typed`, whether or not the host needs them.
- **The ABI packs the response into an i64** `[contract]` (2026-10-17). `feel_call` returns
  `(ptr << 32) | len`, which ties the ABI to wasm32 (no native cdylib), and a host sees an error
  response only by parsing it.

## Runtime (browser and Node)

- **No backpressure** `[runtime]` (2026-10-18). The queue is unbounded and the timeout clock starts
  at execution, so a flood is accepted and served late.
- **Tighter types** `[runtime]` (2026-10-17). `ParseResponse` error kind is `string` while
  `EvaluateResponse` uses `ErrorKind`; `callJson` returns `any`.

## Playground

- **Overlapping request loops** `[playground]` (2026-10-18). Evaluate, parse and URL-hash sync each
  implement "debounce, cancel previous, keep newest" separately; the `pending` counter is used as
  a boolean, and the tree and result loops can briefly show output from different keystrokes.
- **Syntax errors have no position** `[fork]` `[playground]` (2026-10-17). The playground locates
  them with lezer-feel, which can disagree with the engine. The engine's lexer is driven by the
  parser (scope and mode flags decide what a word is), so it is neither tolerant nor incremental,
  and the LALR parser cannot recover from an error.

## Engine

- **Unknown names swallow the following keyword** `[fork]` (2026-10-20). The lexer reads the
  longest run of words as one name and only cuts it back to a name known in scope. A path into
  an untyped value followed by a keyword (`for i in current(p).incomeItems return …`,
  `then i.amount else …`, `some id in item.ids satisfies …`) becomes one name including the
  keyword: a syntax error, upstream too (`for l in zz return l` is the smallest case). Legal per
  the FEEL grammar; hidden in DMN models because item definitions put every name in scope, so the
  TCK does not catch it. The name is read by `consume_name` in `feel-parser/src/lexer.rs`.
  Workaround: parenthesise the path, `for i in (current(p).incomeItems) return …`.
- **Decision-table bounds** `[fork]` (2026-10-18). Rules and outputs are indexed by clause position
  with no count check (`model-evaluator/src/decision_table.rs`): a short rule or a zero-output table
  panics.
- **The call-depth flag is not reset by every entry point** `[fork]` (2026-10-18). The sticky
  `CALL_DEPTH_EXCEEDED` (`feel-evaluator/src/invocation.rs`) is reset by the wasm layer and by
  `ModelEvaluator::evaluate_invocable`, not by `rheon_feel_evaluator::evaluate` and the other FEEL
  entry points: after one runaway, a native caller that evaluates FEEL directly on the same thread
  gets null from every invocation until it calls `reset_call_depth`.
- **Nanosecond rule** `[fork]` (2026-10-18). Time/date-time equality drops nanoseconds, ordering
  keeps them, so `=` and `<` can both hold.
- **Grammar precedence for unary endpoints** `[fork]` (2026-10-17). `> 0.1 + 0.2` parses as
  `(? > 0.1) + 0.2` (`feel-grammar/src/feel.y`).
- **Zoned `now()`** `[fork]` (2026-10-17). `now()` is zone-less, so
  `now() - date and time("…Z")` is null.
- **`sort` comparator bypasses guards** `[fork]` (2026-10-18). Depth and finiteness guards are
  skipped for the user comparator; non-boolean result silently means equal.
- **Globals in the engine** `[fork]` (2026-10-17). The tz-offset override is process-global, so
  the wasm layer serialises requests with `ENGINE_LOCK` (`wasm/feel-wasm/src/engine.rs`), and the
  call depth is thread-local.

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
- **Freed string blocks are too small for longer strings** `[fork]`. A string is cloned on every
  lookup, and `s + s` allocates its result while both operand clones are alive, so the result is
  placed above them; the two clones, freed afterwards, leave holes of the length of `s` that no
  later, longer string fits. `{f: function(s, n) if n = 0 then string length(s) else f(s + s,
  n-1), r: f("ab", 23)}.r` has at most 48 MiB of live data, as at 0.3.0, but grows linear memory
  by 58.7 MiB (0.3.0: 48.1 MiB, where coercing each argument copied it once more, into the freed
  holes); with 24 doublings by 138.7 MiB (0.3.0: 96.1 MiB). Under the 512 MiB cap this decides
  whether one request completes: with 25 doublings (a 64 MiB string) memory peaks at 246 MiB,
  with 26 (a 128 MiB string) an allocation fails at the cap and the instance traps (a `crash`
  that the JS runtime reports as `out of memory:`), while 0.3.0 completes 26 doublings at a peak
  of 393 MiB. Recycling instances does not help, as it is a single request. Nothing leaks: every
  block is free when the request ends, but linear memory never shrinks, so hosts recycle
  instances that grew large (CONTRACT §1).
- **Loop variables are looked up by name** `[fork]`: every iteration resolves the variable
  through the scope stack (`FeelScope::get_value`).
- **Regular expressions are compiled on every call** `[fork]`: `matches` (`feel-regex`),
  `replace` and `split` compile their pattern for each invocation.

## DMN 1.6

Official TCK (patched): 3,392 of 3,407; only 0076 (Java, out of scope) and 0085 (runner) fail.
`scripts/tck.sh` runs it on demand; `docs/tck-baseline.txt` records the failing tests.

Open in 1.6:
- **B-FEEL arithmetic leaves a diagnostic** `[dmn]`. B-FEEL first applies the FEEL operator and
  converts the operands only when that fails, so `1 + "2"` in B-FEEL gives `"12"` with one
  recorded error ("incompatible types in addition"). In B-FEEL a null never means an error, so
  the FEEL attempt should not be recorded.
- **Dialect of decision table entries** `[dmn]`. Input and output entries are unary tests and
  literal expressions that may carry their own `expressionLanguage`; the evaluator applies the
  model's dialect to the whole table. Not exercised by the TCK.
- **Strict mode granularity** `[dmn]`. Strict evaluates to the end and then returns null with the
  first error (no side effects make the difference observable). Errors are recorded where the
  core creates a null with a reason (`value_null!`); a few places build nulls directly and are
  not recorded.
- **TCK conflicts** (tck-addendum/MANIFEST.md): 1151 006 and 1152 003/004 expect the DMN 1.5
  "no singleton conversion" rule for boxed iterators; rheon follows 1.6.
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
    OR (comparison_in.rs asserts AND). The spec also reads differently for `null <= null` (spec:
    `<` or `=`, so true) and `-` against a null input (rule 15.c reads as `? != null`); rheon gives null and
    a match.
  - *Smaller ones*: null range endpoints are handled differently by kind; `between` checks the
    kinds of all three operands first; `list in list` is containment of a multiset; `count(null)`
    is 1; the Any and Priority hit policies compare outputs with Rust equality, not FEEL
    equality; a collection item definition turns a list with a null item into null; a missing
    input has a null with a reason only when its typeRef is an item definition; the nulls of
    `eval_in_*` name internal functions.
- **Invocation follow-ups** `[dmn]` (2026-11-12). Every invocation goes through
  `feel-evaluator/src/invocation.rs` (Table 64: FEEL and boxed invocations, invocables invoked
  with input data, `sort` and `list replace`). Open:
  - *Closures are not lexical.* A function returned by a body closes over the names as they are
    where the body ends (`closed_over`, kept from dsntk), so `{f: function(x) {x: 10, g:
    function(y) x + y}.g, h: f(1), r: h(5)}.r` is 6, not 15 (§10.3.2.13.2: "FEEL functions are
    lexical closures"). A function returned by a boxed
    function body finds the entries of its boxed context through these writes (see *Boxed
    contexts write into the context below them* in "Lexer and evaluation limits"), so the two
    change together.
  - *Decision service as a function.* Its body reads the arguments with `scope.peek()`
    (decision_service.rs), so the invocation must push the parameters last, and it converts them
    a second time through the input data and input decision evaluators.
  - *Java and PMML BKMs are not external.* `build_bkm_evaluator` special-cases only ONNX; a BKM
    with `kind="Java"` or `"PMML"` returns its mapping context instead of calling the function.
  - *Binding types are not checked.* A binding's parameter `typeRef` is ignored (§7.3.6 binds by
    name); §7.3.4 asks it to be compatible with the value.
  - *`try_coerced` lets any function value pass* whatever the target type (feel/src/values.rs).
  - *Call depth counting.* FEEL invocations are counted at the expression (argument evaluation
    included), boxed invocations and invocables in `evaluate_invocation`; `sort` and `list
    replace` callbacks are not counted.
  - *Unused values.* `Value::NamedParameter`, `Value::ParameterName` and
    `Value::PositionalParameters` are not built for invocations (`ParameterName` still is,
    for formal parameters).

- **0085 decisionService_015 (1 test since the TCK update)** — not an engine bug. The engine returns
  the correct context with both output decisions. dsntk-test-runner (`main.rs` `execute_tests`)
  evaluates the decision service once per `resultNode` and compares the whole context against
  each node's expected value instead of selecting the component named by the node.

### FEEL-side cases the TCK patches remove

Of the 20 extra failures on the unpatched TCK, two FEEL ones were fixed on 2026-10-18 (fork
commits 4b5ca51 and 69e81cc: scale range -6111..=6176 for the rounding functions, and regex
back-references via fancy-regex). The unpatched subset stands at 289/298. Remaining:
- **Dates beyond chrono's range** (1115/1117): years outside ±262143 are not supported.
- **0036 typeRefs** is DMN-layer (item definition type references).

## DMN model evaluator

- **Cycle protection** `[fork]` (2026-10-18). Decision→decision and BKM→BKM cycles recurse until the
  stack traps; only item definitions are checked.
- **Evaluator leak** `[fork]` (2026-10-18). Decision services capture a strong `Arc` to their own
  evaluator; a host that reloads models never frees them.
- **Structured errors** `[fork]` (2026-10-18). Unknown invocables, dangling hrefs and unresolved
  types come back as FEEL null; "empty FEEL type" carries no name.
- **No decision-table short-circuit or memoisation** `[fork]` (2026-10-18). Every cell of every
  rule is evaluated regardless of hit policy, and a diamond of required decisions re-evaluates
  shared nodes.

## Build and release

- **The DMN TCK is not run by CI.** `scripts/tck.sh` runs it locally (the runner, the patches and
  a TCK checkout of about 430 MB).
- **Builds on different hosts are not bit-identical.** The npm 0.2.0 `feel.wasm` and the release
  asset, both built from the same commit with the same rustc on macOS and on CI's Linux, differ
  by 310 bytes (string-constant order in the data section, a few bytes of code); behaviour is the
  same, and the release asset is the canonical build.
- **The npm 0.1.0 module carries a wrong engine commit.** Its `feel.wasm` reports
  `engineCommit d811cac…-dirty` (a stale local build copied by prepack); the engine code is that
  of the release. prepack refuses a module not built from HEAD.
