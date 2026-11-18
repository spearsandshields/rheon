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
- **Display renders scalars other than strings to measure them** `[contract]` (2026-11-14). To
  decide whether a list or context fits one 80-column line, `display` measures it with a writer
  that stops past the width left; a string stops the measure at once, but another scalar (a range
  with long string endpoints) is still rendered to text first, at most once per enclosing level
  within 80 bytes of it (`wasm/feel-wasm/src/display.rs`).
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

- **A name that is not in scope is a run of words** `[fork]` (2026-11-16). When no name in scope
  starts at a name token, the name runs to the next punctuation, to a keyword that can follow an
  operand (`and`, `between`, `else`, `in`, `instance`, `or`, `return`, `satisfies`, `then`)
  written as a word of its own, or to a symbol that is an operator (`.`, `/`, `-`, `+`, `*`). So
  with no `x` in scope `x - 1` is a subtraction and `x.y` a path (null, with an error about
  `x`). What stays in such a name: the other keywords (`x for y`) and an apostrophe (`Grace's
  age`). A name with a keyword or an operator symbol must be in scope to be read as one name: a
  name that is not in scope and has one (`Is in range`, `opt-in rate`) is read as the
  expression, often a syntax error. A name in scope is found by its text with single spaces
  between words and none around symbols, so a key written with other spacing (`Date  -  Time`)
  is not found. The name is read by `consume_name` in `feel-parser/src/lexer.rs`.
- **Decision-table bounds** `[fork]` (2026-10-18). Rules and outputs are indexed by clause position
  with no count check (`model-evaluator/src/decision_table.rs`): a short rule or a zero-output table
  panics.
- **Nanosecond rule** `[fork]` (2026-10-18). Time/date-time equality drops nanoseconds, ordering
  keeps them, so `=` and `<` can both hold.
- **Zoned `now()`** `[fork]` (2026-10-17). `now()` is zone-less, so
  `now() - date and time("…Z")` is null.
- **`sort` reads a non-boolean answer as "does not precede"** `[fork]` (2026-10-18). A
  non-boolean answer of the comparator silently means "does not precede" (the ordering itself
  cannot fail on any answer).
- **Globals in the engine** `[fork]` (2026-10-17). The tz-offset override is process-global, so
  the wasm layer serialises requests with `ENGINE_LOCK` (`wasm/feel-wasm/src/engine.rs`). The
  call depth and the abort of an evaluation are thread-local state owned by that evaluation
  (`rheon_feel::diagnostics`), cleared when it ends.

## Lexer and evaluation limits (2026-11-13)

- **Composite names** `[fork]`. A composite key `k1 . k2` (keys of nested contexts joined by
  ` . `) is a name in scope, as in upstream. A candidate name contains ` . ` only through a name
  part that is U+1680 alone (the only name character that trimming removes), and the lexer
  reads such a part only right after an additional name symbol (`.`, `/`, `-`, `'`, `+`, `*`).
  The composite search visits up to (nested contexts) × (separators in the candidate + 1)
  states per candidate.
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
  change together. These writes are also why a loop and a filter push their contexts shared and
  read `partial` back from the frame the body ran in (`feel-evaluator/src/iterations.rs`,
  `filters.rs`), and why a filter evaluates its expression in the enclosing scope in an empty
  frame of its own: no FEEL expression writes into a frame it did not push, so all of them would
  go with the writes.
- **Rust `==` on values is a tree walk** `[fork]`. The derived `PartialEq` of `Value` follows
  every path, so two values built apart that hold the same lists or contexts on many paths take
  time exponential in the sharing. FEEL equality visits each shared pair once, but the native
  DMN decision tables compare output values with `==` for the ANY and PRIORITY hit policies
  (`model-evaluator/src/decision_table.rs`, the output-value positions and the check that all
  results agree); no FEEL request reaches them. A pointer shortcut needs `==` to be reflexive for
  every value, which was not checked; routing them through FEEL equality changes answers.
- **List built-ins copy the items of their argument** `[fork]`. `insert before`, `insert after`,
  `remove`, `reverse`, `list replace` and `sublist` take their list by reference and copy its
  items into the new list, also when the caller held the only reference and the items could have
  been reused (`append(for i in 1..n return i, 0)` copies `n` items).
- **Loop variables are looked up by name** `[fork]`: every iteration resolves the variable
  through the scope stack (`FeelScope::get_value`).
- **Regular expressions are compiled on every call** `[fork]`: `matches` (`feel-regex`),
  `replace` and `split` compile their pattern for each invocation.
- **`sort` on long lists with few distinct values** `[fork]` (2026-11-14). `sort` merges runs of
  its own (`feel-evaluator/src/bifs/sorting.rs`), and on long lists with few distinct values it
  calls the predicate more often than the standard library's sort: 10,000 items of seven distinct
  values take 1.33 times the calls and 1.23 times the time of 0.4.0 (19.7 to 24.2 ms on the wasm
  module). A stable partition of runs of equal items would remove the difference.

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
  core creates a null with an error reason (`value_null!`, a note is not one); a few places build
  nulls directly and are not recorded.
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
  - *The closures of boxed function bodies are not lexical.* A FEEL function is a lexical closure
    (§10.3.2.13.2); the body of a boxed function of a DMN model still reads the names it does
    not bind through the scope of its invocation, and a function it returns closes over the
    names as they are where the body ends (`closed_over`, kept from dsntk). A function returned
    by a boxed
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
    included), boxed invocations and invocables in `evaluate_invocation`, and the callbacks of
    `sort` and `list replace` at the call.
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
- **Decision service values reach their model through the value** `[fork]`. A decision service's
  function value holds a weak reference to its model evaluator (so models are freed) and gives
  null once the model is dropped; the invocation should take the model from the call's session
  instead of from the value.
- **Element nesting bound** `[fork]` (2026-11-14). A model's elements nest at most 100 levels
  deep (`model/src/element_depth.rs`), because roxmltree's tokenizer recurses once per element:
  the measured ceiling on a 2 MiB thread of the unoptimized test build is 124 levels (aarch64,
  rustc 1.95.0; it overflows from 125 on). The bound keeps a margin of about a quarter below it
  because CI runs the at-bound tests on x86_64 Linux, whose ceiling could not be measured. Large
  machine-generated schemas want 256; that needs an XML parser that does not recurse per element.
- **Named types are expanded at each use** `[fork]` (2026-11-14). The size and depth bounds of an
  item definition type hold per definition, so a type is expanded wherever it is used, and the
  global type context, cloned for each decision evaluation, grows with the models times the node
  bound. Sharing named types removes both.
- **Structured errors** `[fork]` (2026-10-18). Unknown invocables, dangling hrefs and unresolved
  types come back as FEEL null; "empty FEEL type" carries no name.
- **No decision-table short-circuit or memoisation** `[fork]` (2026-10-18). Every cell of every
  rule is evaluated regardless of hit policy, and a diamond of required decisions re-evaluates
  shared nodes.

## Build and release

- **The DMN TCK is not run by CI.** `scripts/tck.sh` runs it locally (the runner, the patches and
  a TCK checkout of about 430 MB, each at the commit `scripts/tck-pins.txt` pins).
- **Builds on different hosts are not bit-identical.** The npm 0.2.0 `feel.wasm` and the release
  asset, both built from the same commit with the same rustc on macOS and on CI's Linux, differ
  by 310 bytes (string-constant order in the data section, a few bytes of code); behaviour is the
  same, and the release asset is the canonical build.
- **The npm 0.1.0 module carries a wrong engine commit.** Its `feel.wasm` reports
  `engineCommit d811cac…-dirty` (a stale local build copied by prepack); the engine code is that
  of the release. prepack refuses a module not built from HEAD.
