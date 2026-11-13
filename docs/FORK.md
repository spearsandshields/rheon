# The dsntk engine crates in rheon: changes against upstream

rheon is a fork of <https://github.com/dsntk/dsntk-rs> (the DecisionToolkit
FEEL/DMN engine, Copyright (c) 2015-2026 Dariusz Depta, MIT OR Apache-2.0).
rheon is a permanent fork: the engine is developed here and contributing
changes back to dsntk is not a goal. The dsntk crates sit at the repository
root in upstream's directory layout, so individual upstream fixes can still be
taken by cherry-pick (see "Taking upstream changes" below). This file lists
every engine change in the order it was made. Until 2026-10-28 these commits lived on a `wasm` branch of a local
clone, consumed as a git dependency by a separate `dsntk-wasm` repository; that
repository's content now lives here (`wasm/`, `packages/`, `apps/`, `docs/`,
`scripts/`) and the branch became `main`.

Rules:

- Engine changes are small, self-contained commits.
- `fastnum-backend` is an unpublished side branch with the experimental fastnum
  number backend (see below); it is not needed for wasm.
- The `rheon-*` crates are published on crates.io (`cargo publish --workspace`, see
  "Publishing" below). The four upstream crates rheon never changed (`dsntk-common`,
  `-macros`, `-feel-grammar`, `-recognizer`) left the tree on 2026-11-09 and are used from
  crates.io at dsntk 0.3.0, pinned `=0.3.0` in the root manifest; the dsntk crates with
  manifest-only changes stay in the tree with `publish = false`.
- Crate naming (2026-10-28): a crate whose source rheon has changed is
  renamed `rheon-*` (package name and Rust path; the directory keeps
  upstream's name). A crate carried as upstream ships it, or with manifest-only
  changes such as the number-backend feature forwarding, keeps its `dsntk-*`
  name until it is changed. Renamed so far: `rheon-feel`, `-feel-number`,
  `-feel-parser`, `-feel-evaluator`, `-feel-temporal`, `-feel-regex`,
  `-model-evaluator` and, since 2026-11-09, `-model` (it imports `rheon-feel`
  types, so upstream's `dsntk-model` cannot replace it). `scripts/upstream-rename.pl` holds the mapping; extend
  its crate list when another crate is renamed.

## Base

- Upstream commit: `c76aee02187183b3659a9051f3ebb283630066f7`
  ("Prepared next development version.", workspace version `0.3.1-dev`,
  `git describe`: `v0.1.0-47-gc76aee0`).
- This is the commit recorded as `upstream-commit` in `wasm/feel-wasm/Cargo.toml` (`[package.metadata.rheon]`).
  Commits 1-4 below are the four patches `dsntk-wasm/patches/dsntk/0001..0004`
  applied in order, one commit each. After commit 4 the tree is
  byte-identical to `dsntk-wasm/third_party/dsntk` (verified with
  `diff -r`, excluding `target/`, `.git/` and `.bootstrap-stamp`).

## Engine commits (oldest first)

| # | SHA | Title | Origin |
|---|-----|-------|--------|
| 1 | `e1fc50dc2a3fd9b941712ee0e76c01a51bf74b99` | Build on wasm32: gate Java external functions, 32-bit isize/usize conversions | `0001-wasm32-support.patch` |
| 2 | `b1a04ca6aaec7a93d57ca4141587e7f2c0b5eb4f` | feel-temporal: process-wide local UTC offset override for hosts without a time zone | `0002-local-offset-override.patch` |
| 3 | `ec2ced32fff2b0fdc72a8173385774cbb654a567` | Robustness: no user-reachable panics, non-finite numbers are null, call depth limit, iterative AST drop | `0003-robustness.patch` |
| 4 | `55948221d3e11df370366696d3d52431820d7c9f` | Linear `for` loops and list literal parsing (were quadratic) | `0004-linear-for-and-list-parsing.patch` |
| 5 | `8c56f2c83c5fb28d3cdd5d8faa10444ffa4d4996` | Add docs/FORK.md | fork only |
| 6 | `1fd27a4` | feel-number: shared decimal128 core for pure-Rust backends | dashu spike |
| 7 | `dde684a` | feel-number: dashu-float backend, feature `dashu` (default) \| `intel` | dashu spike |
| 8 | `c75b0da` | Forward the number backend features through the whole workspace | fork |
| 9 | `ecf9feb` | Tests: per-backend expectations where Intel's rounding error is encoded, dashu regressions | fork |
| 10 | `e6b67e8` | feel-number: track the sign of NaN like Intel (Debug-visible only) | fork |
| 11 | `6af4e49` | tools: differential fuzz and FEEL corpus harnesses (Intel vs dashu) | dashu spike |
| 12 | `0bfbc9b` | feel-number: `NUMBER_BACKEND` constant naming the compiled-in backend | fork |
| 13 | `3fe6406` | tools: vendor the patched dfp-number-sys used as the Intel reference | fork only |
| 14 | `8542135` | feel-number: scale rounding keeps the scale when the rounded coefficient is ±1 | upstream bug |
| 15 | `7c16676` | feel-evaluator: stop a numeric `for` range when the step is lost to rounding | upstream bug |
| 16 | `6f08eab` | feel-temporal: time ± duration wraps at midnight; order times and date-times across offsets as instants | upstream bug |
| 17 | `cb5997f` | feel-parser: bound the name-prefix search in the lexer (path chains were cubic) | upstream bug |
| 18 | `58bbc18` | feel-evaluator: readable null reasons for time/exp/log/duration; huge numbers in messages in scientific notation | fork |
| 19 | `f619d8a` | feel-temporal: `is()` on times stays zone-strict now that `=` compares instants | with 16 |
| 20 | `3dbd70e` | model-evaluator: expectations for the new null reasons of time(), exp(), log() and duration() | with 18 |
| 21 | `4b5ca51` | feel-number: scale-based rounding accepts the full FEEL scale range -6111..=6176 | TCK 1141-1144 |
| 22 | `69e81cc` | feel-regex: back-references in `matches` patterns (fancy-regex) | TCK 1111 |
| 23 | `b3b616b` | model: accept the DMN 1.6 namespace | DMN 1.6 |
| 24 | `f893c57` | feel: single-argument rounding and number(from); trim, intersection, insert after (1.7 beta) | DMN 1.6 |
| 25 | `f3259a0` | feel: `value` property of temporal values; filters on a non-list; number() text must be a numeric literal; tck-addendum | DMN 1.6 |
| 26 | `18bcd98` | feel: truncate fractional list positions before the sign check (remove panicked) | upstream bug |
| 27 | `f9b8111` | feel: date and time(date, time, timezone) | DMN 1.6 |
| 28 | `4aa566d` | model: load ONNX functions; compatibility tests follow the 1.6 changes | DMN 1.6 |
| 29 | `62bcf99` | feel-grammar back in the tree as rheon-feel-grammar (tables reproduce byte for byte) | fork |
| 30 | `18f0fd1` | feel: descendant operator `e...name` (grammar, lexer, AST, evaluator) | DMN 1.6 |
| 31 | `070be1a` | model: implicit conversions, TCK 1157 12/12 | TCK 1157 |
| 32 | `b986c79` | model: multiple imports (per-model import names), TCK 1160 4/4; boxed for over a single value | TCK 1160 |
| 33 | `e56f08c` | feel: B-FEEL; all()/any() follow the deciding item | DMN 1.6 |
| 34 | `6baa444` | feel: diagnostics and lenient/strict error modes; wasm `language`, `errorMode`, `diagnostics` | DMN 1.6 |
| 35 | `5e04541` | feel: equality with null no longer depends on the operand order | upstream bug |
| 36 | `94ec326` | feel: null equality is one rule; a non-conformant argument ends the invocation | upstream bug |
| 37 | `f65f252` | feel: one invocation for every function call (Table 64) | DMN 1.6 |
| 38 | `a1d5a62` | feel: `in` treats every null alike | upstream bug |
| 39 | `ea795d2` | feel-parser: names found in an index over the borrowed input contexts; composite names searched, not flattened | upstream bug |
| 40 | `ea795d2` | feel: the type of a list of contexts without building the item types | fork |
| 41 | `ea795d2` | feel-evaluator: every evaluator that pushes a context restores the scope depth on return | upstream bug |
| 42 | `ea795d2` | feel-evaluator: FEEL loops and filters move their contexts instead of cloning them | fork |
| 43 | `ea795d2` | feel-number: decimal128 comparison, digit count and packing without 128-bit division | fork |
| 44 | `ea795d2` | feel: function values share their definition (`Arc<FunctionDefinition>`) | fork |
| 45 | `ea795d2` | feel-wasm: request input converted by value | fork |
| 46 | `ea795d2` | feel-evaluator: subtraction null reasons from the evaluated operands (2^n - 1 diagnostics become n) | upstream bug |
| 47 | `ea795d2` | wasm profile at opt-level 3 with a 10 MiB shadow stack | fork |
| 48 | `6e581a7` | feel-parser: an iteration variable that starts with `in` is a syntax error, not a trap | upstream bug |
| 49 | `1e4a58b` | feel-parser: a range literal's named temporal parameter keeps its name | upstream bug |
| 50 | `a9244b6` | feel-parser: list grammar actions append and complete each list once (the tails were quadratic) | upstream bug |
| 51 | `6faa802`..`984d22f` except `23fc84b` | feel-parser: lexer and parser without per-token copies | fork |
| 52 | `23fc84b` | feel-parser: the composite-name search without recursion | fork |
| 53 | `3cab20b` | feel-evaluator: a date range ending at the first or last supported date restarts at its start | upstream bug |

(The commits are on `main`, after the base commit; `git rev-parse <sha>` gives a full SHA.)

What each commit does:

1. **wasm32 support.** `reqwest` becomes a
   `cfg(not(target_arch = "wasm32"))` dependency of `feel-evaluator`; on
   wasm32 the Java external-function evaluator is replaced by a stub that
   returns a FEEL null ("external Java functions are not supported on
   wasm32"). The `isize`/`usize` conversions in `feel-number` no longer
   `compile_error!` on 32-bit targets; out-of-range values are a conversion
   error (FEEL null) instead of a panic. Java external functions are
   unsupported on wasm by decision; this is the final shape.
2. **Local offset override.** `feel-temporal` gains
   `set_local_offset_override`, `local_offset_override` and `local_now`.
   `chrono::Local` is always UTC on WASI, so an embedding host sets a fixed
   offset (dsntk-wasm sets it from the request's `tzOffsetMinutes`). Without
   an override nothing changes. This patch is kept by decision.
3. **Robustness.** Every user-reachable panic found in review becomes a FEEL
   null or an error: non-finite decimal results (`FeelNumber::is_finite`,
   `Display` never panics on Inf/NaN), `time()` with an out-of-range offset,
   unpaired `\u` surrogate escapes, huge positions/lengths (positions go
   through 64-bit integers so wasm32 and 64-bit agree), overflowing duration
   arithmetic. Adds `MAX_CALL_DEPTH` for nested function calls,
   `AstNode::depth`/`for_each_child`/`is_leaf`, an iterative `Drop` for
   `AstNode`, `from_str_reporting_rounding`, and a linear `Value::type_of`
   for lists (was exponential in nesting depth). The CLI `evaluator` crate
   gets a two-line fix so it compiles with the iterative drop.
4. **Linear loops.** `ForExpressionEvaluator` moved the `partial` list in and
   out of the iteration context instead of cloning it every iteration;
   `action_list_tail` appends and `action_list` reverses once instead of
   inserting at the front. `count(for i in 1..100000 return i)` went from
   over 30 s to 34 ms natively.
5. This file.
6. **Shared decimal128 core** (`feel-number/src/decimal128.rs`). The FEEL
   number with decimal128 semantics (34 digits, half-even, exponent
   -6176..=6111, cohorts), stored as a `Copy` triple (sign, `u128`
   coefficient, `i32` exponent) plus Inf/NaN. Exact operations are computed
   in the core (`u128`, or a small `U256` for wide sums, products, square
   roots and the cohort decision): `+ - *`, `sqrt` (exact integer root with
   remainder as sticky digit), rounding to integral, quantize, scalbn,
   comparisons, parsing (`bid128_from_string` rules), Intel-compatible
   `Debug`/`Display`, conversions, the integer-exponent `pow` (Intel's binary
   exponentiation, bit-identical) and the binary128 cohort emulation of
   `exp`/`ln`. The inexact operations go through a private trait `Backend`
   (`div`, `exp`, `ln` and one round of `exp(y ln x)` at a working
   precision); the backend returns the correctly rounded value with the side
   the exact result lies on, the core turns that into sticky digits and
   rounds exactly once (no double rounding in the subnormal range), and runs
   the precision-escalating Ziv loop (45/60/90/140 digits) for fractional
   powers, rounding exact ties half-even.
7. **dashu backend** (`feel-number/src/backend_dashu.rs`): dashu-float's
   non-panicking `Context` API at precision 34 (and at the working
   precisions for `pow`); dashu's own `sqrt`/`powf`/`powi` are not used
   (wrong rounding when the remainder equals the root, mis-certified near
   ties, non-terminating exact ties). Features in `feel-number`: `dashu`
   (default, pure Rust, no C toolchain) and `intel` (dfp-number-sys,
   `dfp_number.rs` untouched); exactly one must be on.
8. **Feature forwarding.** Every crate that depends on feel-number
   (feel-temporal, feel, feel-parser, feel-evaluator, model, model-evaluator,
   evaluator, workspace, gendoc, server, dsntk) has `dashu` (default) and
   `intel` features forwarding to its dsntk dependencies, and the workspace
   dependency table declares those crates with `default-features = false`.
   One selection covers the whole build graph; the fastnum/dashu spikes
   could only select the backend for the five-crate chain.
9. **Tests.** `feel-number/tests/pow.rs` `test_pow_002` (`12.2384283 ** 4.25`)
   and `model-evaluator` `dmn_3_0100::_0278` (TCK 0100
   `exponent_lhs_number_exp_rhs_number_005`, `5 ** 2.55`) encoded Intel's
   result, 1 ulp below the correctly rounded value; they are now per backend.
   Plus the spike's regression tests (scalbn clamping, exp/ln cohorts, pow
   with bases within 1e-33 of one, `(-1)^y` cohorts) and the core's own unit
   tests (256-bit helper against schoolbook string arithmetic, add/mul/sqrt
   against exact results, guards, subnormal single rounding, sqrt half-way,
   pow ties, scalbn padding, NaN sign).
10. **NaN sign.** `V::NaN` carries a sign (negation flips, `abs` clears, a
    NaN operand propagates its sign, invalid operations give `+NaN`) so that
    `Debug` prints `-NaN` exactly like `bid128_to_string`; `Display` and FEEL
    are unchanged (`NaN` / null).
11. **Tools** (`tools/`, see `tools/README.md`): the op-by-op differential
    fuzzer with the mpmath/decimal oracle and the FEEL corpus harness.
12. **`NUMBER_BACKEND`.** `rheon_feel_number::NUMBER_BACKEND` is `"dashu"` or
    `"intel"`, selected by the same feature as the backend itself; dsntk-wasm
    reports it as `numberBackend` in its `info` op.
13. **Intel reference for `tools/`**: the patched `dfp-number-sys` crate the
    `tools/` harnesses build the Intel side with. It was vendored in full
    (17 MB, mostly Intel's C library); it is now `tools/dfp-number-sys.patch`
    on top of the crates.io 0.2.14 release, fetched by
    `tools/fetch-dfp-number-sys.sh`.
14. **Scale rounding.** `unscale()` (both backends) returned `±1` without
    scaling back whenever the rounded integral was `±1`, so `round up(0.1, 1)`
    was `1`, `floor(0.19, 1)` was `1`, `round up(10, -1)` was `1`. The
    upstream tests `round up(±5.5, -6111)` = `±1` and `floor(-1.56, -6111)` =
    `-1` pinned the bug; they now expect `±1E+6111`.
15. **Lost-step `for` ranges.** `for i in 1e34..1e34 return i` never ended
    (`1e34 + 1 = 1e34` at 34 digits, so the counter never advanced and the
    list grew until the memory cap). The number-interval iterator stops when
    the next value equals the current one; the result is `[1e34]`.
16. **Temporal fixes.** `time + days and time duration` wraps around midnight
    with the nanosecond carry (`23:59:59 + PT1S = 00:00:00`; it was
    `24:00:00`, and the duration's fraction was dropped). Times and date-times
    whose zones have an offset are ordered as instants in UTC
    (`2026-10-17T10:00:00+01:00 < 2026-10-17T10:00:00Z`,
    `10:00:00+02:00 = 08:00:00Z`); ordering across different offsets used to
    be always false. A local value stays comparable only with another local
    value (as for subtraction), so `@"…T23:59:59Z" >= @"…T00:00:00"` is still
    false (upstream test `comparison_ge::_0030`).
17. **Lexer.** `.` is an additional name symbol, so after each `Dot` the lexer
    scanned the rest of an `a.a.a…` chain as one candidate name and tried
    every prefix (O(n³) for n steps; 400 steps took 2 s). Prefixes longer
    than any name the scope can match are skipped; the fallback name is
    unchanged. The bound counts the composite keys only for a candidate
    with a part that trims to empty, and the built-in property names only
    right after a dot (39).
18. **Messages.** `time("10:00:00+15:00")` said `time_1`, `exp(14150)` said
    `exp`, `duration("P")` said `duration`, `log(0)` said nothing; they now
    say what is wrong, and type errors use the `invalid argument type` form.
    `sublist([1,2,3], 9e6144)` embedded 6,145 digits in its reason; numbers
    longer than 40 characters are formatted as `9E+6144` in messages.
19. **`is()` on times.** `FeelTime::is` compares the fields and the zone as
    written (TCK 0103 `is(@"20:00:50+00:00", @"21:00:50+01:00")` is false),
    since `==` now compares instants.
20. **TCK expectations** for the reason texts changed by commit 18.
21. **Scale range of the rounding functions.** `round up(5.5, 6176)` was null
    because the scaled intermediate `n * 10^scale` must fit decimal128
    (adjusted exponent <= 6144), which was enforced as if it were the scale
    limit. `check_scale` (both backends) now rejects only scales outside the
    FEEL range -6111..=6176 and returns a value unchanged when it has no digit
    below 10^-scale; `decimal()` accepts 6176. TCK 1141-1144 `017_a` pass
    unpatched (dsntk-tck-patches had rewritten them to 6144).
22. **Back-references in `matches`.** `feel-regex` compiles `matches` patterns
    with `fancy-regex` (back-references, look-around, a backtracking limit
    that turns pathological patterns into an error instead of a hang);
    `replace`/`split` keep `regex`. With the `x` flag, whitespace inside
    `\p{...}` is stripped before compiling. TCK 1111 `K2-MatchesFunc-17`
    passes unpatched.

Commits 23-38 are described in their commit messages. Commits 39-53 make parsing and
evaluation faster and fix the upstream bugs found on the way; apart from the fixes marked
"upstream bug", every response is the same as 0.3.0's (measured below).

39. **Name index.** For every parse 0.3.0 converted the whole evaluation scope into a parsing
    scope (a deep copy of every input context), and for every name the lexer read it built
    the set of all flattened keys in scope, the composite keys `k1 . k2 . … . kn` of nested
    contexts included. That set grows exponentially with the nesting depth: a path
    `k0.k1.….k99` into a 100-level context ran into the 512 MiB cap. `ParsingScope<'a>` now
    borrows the input contexts (`FeelScope::frames`) and indexes each one on first use
    (`FrameIndex`: one walk with an explicit stack over the nested contexts, the context types
    and the item type of a list of contexts whose items all conform to it). A name that
    contains the composite separator ` . ` is found by a search over the states (node, offset
    in the name), each visited at most once. The lexer's prefix bound (17) is computed without
    the set: the composite keys count only for a candidate with a part that trims to empty
    (only U+1680 trims away, and only such a part can put ` . ` into a candidate), the
    built-in property names (14 bytes at most, derived from `bip::BUILT_IN_PROPERTY_NAMES`,
    `1d0b247`) only right after a dot. One function decides which values carry nested names,
    for the index, the composite search and the contexts pushed while parsing (`9b8d01a`); list
    item types are keyed by address, so `ParsingScope` is `Send` as at 0.3.0 (`736a442`).
40. **List types.** `Value::type_of` of a list built the type of every item to find the common
    item type of a list of contexts. A context is now checked against a context type entry by
    entry, without building its type (`364254f`: one rule for the type of a list). The types are
    the same; randomized tests compare the rule with 0.3.0's at every nesting level.
41. **Scope depth.** The filter (`only number or boolean indexes`) and the context literal (an
    entry that is a null) returned early without popping the context they had pushed, so the
    caller popped the wrong one: `for i in 1..3 return [1,2]["x"]` was `[null]` (two results
    lost), and in `{r: for i in 1..3 return [1,2]["x"], s: i}` the loop variable was still
    visible after the loop (`s: 3`). Every evaluator that pushes a context (filter, context
    literal, invocation, B-FEEL operands, iterations) records the depth and restores it on every
    return (`FeelScope::{depth, truncate, pop_frame}`): three nulls, and `s: null`.
42. **Loops and filters move their contexts.** FEEL `for`, `some` and `every` cloned the
    iteration context into the scope for every iteration, and the filter cloned every item.
    They now move the context into the scope and back (`partial` is assigned in place,
    `FeelContext::assign_entry`). The filter moves each item: a context with an `item` entry
    becomes the frame itself, another context is copied once for its entries, and the frame
    `{item: …}` is reused from item to item. The iteration advances with `next()` alone, and
    `some` and `every` stop iterating at the deciding item (0.3.0 stopped evaluating there but
    ran the iteration to its end; results and diagnostics are the same). If an evaluation
    removed the iteration or item context, which no evaluator does, the loop, filter, `some` or
    `every` returns null with an internal-error reason instead of a wrong result, after the
    scope is restored (`65858db`); `ForExpressionEvaluator::evaluate` returns that `Value`.
    The model evaluator's boxed iterators and filters keep copying (`new()`), because a boxed
    context writes its entries into the context on the top of the scope (docs/BACKLOG.md); the
    FEEL builders use `for_feel()`. `count(for i in 1..10000 return i)` natively: 132
    allocations (+17 reallocations) per call, 40,199 (+26) at 0.3.0.
43. **Decimal fast paths.** `digit_count` comes from the bit length (times 1233/4096, then one
    comparison with a power of ten) instead of 128-bit divisions. Two finite numbers with equal
    exponents are compared by their coefficients; otherwise only the coefficient with the
    larger exponent is scaled (`checked_mul`; an overflow means that side is larger).
    `round_pack_u128` returns at once for a coefficient below 10^34 with the exponent in range.
    Exact: unit tests compare each path with the general algorithm on every edge value and on
    random values (`feel-number/src/decimal128/tests.rs`), and `tools/difftest/run.sh 100000`
    (3,900,001 cases) on this code gives the same mismatch set, cohorts and oracle verdicts
    against Intel as 0.3.0's core. The production code of `decimal128.rs` is unchanged since
    `ea795d2`.
44. **Shared function definitions.** `Value::FunctionDefinition` holds an
    `Arc<FunctionDefinition>` (named fields, `FunctionDefinition::new`, `From<FunctionDefinition>
    for Value`) instead of six fields, so a name lookup or an argument copies a pointer. The
    invocation borrows the definition, and `closed_over` copies it (`Arc::make_mut`) only when
    the closure has entries to add. Arguments and results are coerced by value
    (`Value::try_into_coerced`, `into_coerced`, with the checks of `try_coerced`, `593f3ad`).
    Without the copy of each argument the allocator places strings differently: a recursion
    that doubles a string argument 23 times grows linear memory by 58.7 MiB (0.3.0: 48.1 MiB)
    with the same live data (docs/BACKLOG.md, *Freed string blocks are too small for longer
    strings*).
    `size_of::<Value>()` is 64 B natively (128 B at 0.3.0) and 64 B on wasm32 (as at 0.3.0:
    there the 48-byte `FeelNumber` sets it). `FunctionDefinition` derives `Debug` (`96776fd`),
    so the `Debug` text of a function value differs from 0.3.0's.
45. **Input by value.** The request's `context` and `input` are converted by value: strings
    and keys are moved (a key that trimming leaves unchanged becomes the `Name` without a
    copy), numbers are parsed from the JSON number's own text (`arbitrary_precision`), entries
    are inserted with `FeelContext::insert`, and the rest of the request is dropped before
    parsing. The path of a value in warnings and errors (`a.b[0]`) is written only when a
    message needs it, without recursion (`649a317`). The texts and order of the warnings and
    errors are 0.3.0's (golden tests in `wasm/feel-wasm/tests/contract.rs`).
46. **Subtraction reasons.** The null reason of a failing subtraction evaluated both operands
    again to print them, so a chain of failing subtractions was exponential:
    `{a: "x", r: a - 1 - … - 1}.r` with 16 subtractions recorded 65,535 diagnostics
    (2^n - 1). The reason is formatted from the operands already evaluated: 16 diagnostics, with
    the same texts.
47. **Build profile.** `[profile.wasm]` builds at `opt-level = 3` (was `"z"`); its stack frames
    are larger, so the shadow stack is 10 MiB (was 8 MiB, `wasm/feel-wasm/build.rs`).
    `scripts/stack-headroom.mjs` (`npm run stack`, run by CI, `b845690`) checks that five
    constructs nested to the AST depth limit evaluate on Node's main thread, in a Node worker
    with a 4 MiB stack and in Wasmtime with a 4 MiB wasm stack, and that a recursion of 190
    calls whose body is nested 60 levels (the depth 0.3.0 reaches in the TurboFan worker and in
    Wasmtime, measured on macOS arm64) evaluates in the worker and in Wasmtime. Measured on
    the final module (macOS arm64, Node 26, Wasmtime 49.0.3):
    - engine time: an opt-level z build of the same code takes 1.85x as long (geomean of 24
      requests, two alternating runs);
    - size: 3,786,626 B raw, 1,018,652 B gzip, 667,524 B brotli (opt-level z: 3,160,691 /
      840,786 / 552,386; 0.3.0: 3,129,141 / 830,762 / 545,186);
    - initial linear memory: 11.38 MiB (0.3.0: 9.38 MiB);
    - the deepest body (nesting levels) with which `npm run stack -- --report`'s recursion of
      190 calls still evaluates: Node worker with a 4 MiB stack 64 with either V8 compiler
      (0.3.0: 49 Liftoff, 60 TurboFan), Wasmtime with a 4 MiB wasm stack 64 (0.3.0: 60),
      Wasmtime's default 512 KiB 8 (0.3.0: 6), Node's main thread 17 Liftoff, 14 TurboFan
      (0.3.0: 10, 14). At 64 the 10 MiB shadow stack runs out, not the host's native stack.
48. **`in` at the start of an iteration variable.** Where the grammar expects an iteration
    variable, the lexer cuts the name before its first `in` part. When the first part was `in`
    the cut underflowed (`index - 1`): `for in+1 in [1] return 1`, `for in[1] return 1` and
    `some in.x in [1] satisfies true` panicked natively and trapped the wasm instance. Such an
    `in` is now the keyword `in` (a name never starts with a literal terminal symbol,
    DMN 1.6 §10.3.1.4), so these are syntax errors; `for in1 in [1] return in1` is `[1]` as
    at 0.3.0. After a variable named `item` the mode stays set (docs/BACKLOG.md), so
    `for item in[1] return item` (a trap at 0.3.0) is `[1]`.
49. **Range literal with a named parameter.** The grammar action for the named temporal
    parameter of a range literal read the colon's slot, so
    `range("[date(from: \"2012-01-01\")..date(\"2013-01-01\")]")` built `date` with an unnamed
    string and was null ("invalid range literal"). `feel.y` models the parameter like a named
    parameter of an invocation (tables regenerated with Bison 3.8.2): the range
    `[2012-01-01..2013-01-01]`.
50. **List grammar actions.** The nine list-tail actions inserted each item at the front
    (`insert(0)`, quadratic in the length of the list; commit 4 fixed only the list literal).
    Every list is now built by appending and completed once by its consumer, through one pair
    of helpers (`e3bfd1b`).
51. **Lexer and parser without per-token copies.** Each step is exact (the token streams, ASTs
    and errors are compared on exhaustive and randomized inputs):
    - token values move into the AST; the parser's stacks start with room for 16 states,
      16 values and 8 nodes;
    - each name chain is scanned once per parse, and its parts are byte ranges into the input,
      not strings;
    - a candidate without an empty part skips the composite search, and probing the contexts
      pushed while parsing allocates nothing;
    - the candidate bound uses only the names that share the token's first word;
    - the input index is built on first use, and a one-part name needs no lookup unless the
      scope decides its token;
    - string literals without escapes and runs of digits take one exact allocation;
    - the keyword window (12 characters) is read only after a keyword's initial;
    - the lexer reads the input `&str` in place (0.3.0 copied it into a `Vec<char>`, 4 bytes
      per character).

    Memory, measured natively on the final code against the code of commit 39: `x1 + x2 + … +
    x400` with the 400 names in the context takes 2,953 allocations per call (164,153), and its
    peak live memory is 172,541 B (147,801 B), because the parts of every name chain are kept
    for the whole parse (32 B per part natively, 16 B on wasm32). The parse of `1 + 2` peaks at
    2,338 B (1,338 B), the stacks reserved up front.
52. **Composite search without recursion.** The first composite lookup converted every input
    context into a parsing context with a recursive conversion (stack depth = nesting depth).
    The search runs on the index's nodes with an explicit stack, and one implementation serves
    the input contexts and the contexts pushed while parsing (`4608025`). A composite name
    2,000 levels down parses in a 256 KiB thread. Requests to feel.wasm cannot nest that deep
    (JSON contexts stop at about 125 levels, FEEL contexts at the AST limit), so this is
    robustness for direct callers of the library.
53. **Date range at the limit.** A date range that ends at the last supported date
    (262142-12-31, going up) or the first (-262143-01-01, going down) repeated its end date when
    an outer iteration context advanced, instead of restarting at its start: the iteration
    contexts are a cross product, inner to outer (DMN 1.6 §10.3.2.14).

Speed, measured 2026-11-13 on the final module (wasm, Node 26 on macOS arm64, median wall time
of a `feel_call` on a warm instance): 5.31x 0.3.0's speed on the geomean of 24 requests and
5.10x on 84 parse-heavy requests. For example, a formula over a 1,000-entry input context
7.13 ms → 0.46 ms; a 200-entry context literal with a 1,000-entry input context 188 ms →
1.36 ms; `x1 + … + x400` 55.0 ms → 0.29 ms; the 100-segment path `a.a.….a` 356 ms → 0.36 ms;
a 5,000-entry context literal 19.2 ms → 6.2 ms; `count(for i in 1..10000 return i)` 7.58 ms →
1.11 ms; filtering 10,000 contexts 38.3 ms → 11.1 ms. Full responses (result, type, warnings,
diagnostics, errors) of 1,174 requests (evaluate requests and a parse request made from each)
equal 0.3.0's except 52 registered differences, all from commits 39, 41, 46, 48, 49 and 53.

Verification after commits 21-22 (native, dashu): feel-number tests on both
backends, feel-regex, feel-evaluator 3376 passed / 13 failed (Java RPC
tests) / 3 ignored, model-evaluator compatibility tests (all but TCK 0076,
Java). Unpatched TCK subset: 289/298 (remaining: 0036 typeRefs, 1115/1117
dates beyond chrono's year range). Patched TCK: unchanged, 3377/3407 result
rows (same 30 non-successes as before: 0076, 0085 015, 1157, 1160).

Verification after commits 14-20 (native, dashu): feel-number 21 unit +
228 integration tests pass; feel-temporal 57 (lib) + the `tests/` suite,
feel-parser 461+; feel-evaluator 3370 passed, 13 failed (the
`external_functions` tests, which need the Java RPC service), 3 ignored;
model-evaluator 3651 passed, 18 failed (all TCK 0076, Java external
functions), 20 ignored. Clippy is clean for the touched crates.

### Known, intended differences between the dashu core and the Intel backend

- `exp`, `ln`, FEEL `log`, `pow` with a non-integer exponent or an exponent
  outside `i32`: the core is correctly rounded, Intel (binary128 inside) is
  1 ulp off in 11-16% of random inputs, 2-9 ulp in about 0.3%, rarely more.
- Scaling to the overflow edge: Intel's `bid128_scalbn` pads the coefficient
  only to about 33 digits before reporting overflow, the core to 34, so
  `floor(999999999999999E+6116, 14)` is finite here and null (overflow) on
  Intel.
- `ilogb + scale` outside `i32` (a zero with a negative scale, e.g.
  `ceiling(0, -1)`, or a scale near the `i32` limits): Intel overflows the
  addition (panic with overflow checks, as in dsntk-wasm release builds;
  wrapping otherwise, which then reports the invalid scale with the wrapped
  number); the core computes it in `i64` and reports the invalid scale with
  the clamped value. Both are null in FEEL, the reason text differs in the
  quoted number. `FeelNumber::new(n, i32::MIN)` likewise saturates instead
  of overflowing `-s`.
- The cohort (trailing zeros, visible only through `Debug` and `abs()`) of
  an `exp`/`ln` result that is an integer or a short dyadic fraction follows
  an emulation of Intel's binary128 conversion; it differs in about 1.5e-5
  of such results (`exp(74.971090)`, `exp(log(2.5))`, `log(exp(7))`).

## Verification (native, cargo 1.95.0 / rustc 1.95.0)

Commits 39-53 (2026-11-13): workspace 8,488 passed, 0 failed, 24 ignored (CI's skips); clippy
without warnings; feel.wasm tests 56 native and 57 under Wasmtime; runtime 33; the randomized
and exhaustive tests at full size (`RHEON_FULL_TESTS=1`, release, with the parser corpus) 867
passed, with no unregistered difference against the frozen reference front end; DMN TCK via
`scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, no result changed against
`docs/tck-baseline.txt`.

DMN 1.6 work (commits 23-34, 2026-11-10): workspace 8,383 passed, 0 failed (CI skips);
feel.wasm contract tests 53 under wasmtime; runtime 33; DMN TCK via `scripts/tck.sh`
(patched, DecisionToolkit runner): official 3,392 of 3,407 (0076 Java 14, 0085 runner 1), was
3,377; tck-addendum 312 of 312 runnable cases (error-result cases are skipped by the runner and
pinned by Rust tests). Failing tests: `docs/tck-baseline.txt`.

Earlier numbers:

Default features (`dashu`), at commit 10:

```sh
cd ~/dev/rheon
cargo build --workspace                       # no dfp-number-sys in `cargo tree`
cargo test -p rheon-feel-number               # 21 unit + 207 integration tests
cargo test -p rheon-feel-temporal -p rheon-feel-parser -p rheon-model
cargo test -p rheon-feel-evaluator --lib -- --skip external_functions
cargo test -p rheon-model-evaluator
cargo clippy -p rheon-feel-number --lib --tests
```

| Crate | dashu (default) | intel (`--no-default-features --features intel`) |
|-------|-----------------|-----------------------------------------------|
| rheon-feel-number | 228 passed, 0 failed (21 unit, 207 integration) | 207 passed, 0 failed |
| rheon-feel-temporal | 79 passed | 79 passed |
| rheon-feel-parser | 461 passed | 461 passed |
| rheon-model | 45 passed | 45 passed |
| rheon-feel-evaluator | 3371 passed, 13 failed, 3 ignored | 3370 passed, 14 failed, 3 ignored |
| rheon-model-evaluator | 3650 passed, 19 failed, 20 ignored | 3650 passed, 19 failed, 20 ignored |

The 13 feel-evaluator failures are `tests::external_functions::*` (Java RPC
server at `127.0.0.1:22023`, same on both); the 14th on intel was
`tests::bifs::bif_now::_0001` (`now()` crossing a second boundary, flaky).
The 19 model-evaluator failures are `dmn_3_0076::*` (18, Java external
functions) and `dmn_3_0103::_0017` (`time_006`, depends on the current
Australia/Melbourne DST offset), identical on both backends; before commit 9
dashu also failed `dmn_3_0100::_0278` (`5 ** 2.55`, Intel's 1-ulp value).

The Intel runs need the patched `dfp-number-sys` (the crates.io version has
FFI signature bugs), injected without committing a machine-specific path:

```sh
cargo test --no-default-features --features intel \
  --config "patch.crates-io.dfp-number-sys.path=\"$(tools/fetch-dfp-number-sys.sh)\"" \
  -p rheon-feel-number
git checkout Cargo.lock          # --config patch.crates-io rewrites the lock file
```

`cargo check --workspace --no-default-features --features intel` (with the
same `--config`) succeeds and `cargo tree` then shows no dashu crate.

### Differential results (`tools/`, commit 10)

- `tools/difftest/run.sh 200000`: 7,800,001 cases (200k per op, 39 ops), 58.8 s
  wall (263 s CPU), 184,687 Display mismatches, all judged by the oracle:
  0 `INTEL correct, DASHU WRONG`; exp/ln/log/pow and transcendental chains
  `dashu correct, intel wrong` (<=1 ulp: 122,747; 2-9 ulp: 1,668; >=10 ulp:
  10); scale ops and `new` at the overflow edge `dashu correct, intel wrong
  (Inf/PANIC vs finite)` (1,485); the Err-vs-panic rows of the `ilogb +
  scale` overflow (about 1.9% of each scale op, judged "both correct");
  31,062 `round_quantize` rows outside the quantum range (no oracle, Intel
  panics in `bid128_scalbn` of the quantum, the core reports NaN -> null) and
  the chain steps without a recorded operand. 6 cohort-only (Debug)
  differences (2 `exp`, 4 chains through `ln`), the known emulation limit.
  Bit-identical (0 mismatches): parse, `+ - * / %`, assign ops, neg, abs,
  trunc, frac, sqrt, square, pow with an `i32` exponent, FEEL modulo,
  `decimal`, comparisons, predicates, formatting, the ten integer
  conversions, `From<int>`, constants, and the 200k random exact-op chains.
- `tools/evaluator-corpus/run.sh`: 17,217 FEEL expressions. `corpus` (3313):
  1 mismatch, `now()`. `gen` (8636): 115 Display mismatches, 112 judged dashu
  correct and 3 `ceiling(0, -1)` variants (both null, reason text) + 2
  Debug-only cohorts. `extra` (1090): 44 mismatches: 20 exp/log/pow where
  dashu is correctly rounded (one of them, `(1+1e-33) ** (-1e34+1)`, needs
  300 mpmath digits: truth `...05508826375`, dashu `...088`, Intel `...089`),
  6 x `floor(999999999999999E+6116, 14)` & co. (Intel overflow quirk, null vs
  finite), 18 zero-with-negative-scale rows (both null, reason text). `dmn`
  (4178): 1 mismatch, `5 ** 2.55` (dashu correct). 0 panics on either side.
  Per corpus 0.02-0.5 s per backend.

The same tree, as `dsntk-wasm/third_party/dsntk`, also passes dsntk-wasm's
`cargo test --target wasm32-wasip1` under wasmtime for commits 1-4; the
dashu commits have not been run on wasm32 from this repository yet (the
dashu spike built feel-evaluator with `dashu` for `wasm32-wasip1` and
`wasm32-unknown-unknown` without a C toolchain).

## The `fastnum-backend` branch

`fastnum-backend` = commits 1-12 of the table above as they were on 2026-10-16 (before the history
rewrite, so with other hashes) + one commit adding
`feel-number/src/backend_fastnum.rs` behind feature `fastnum` (fastnum's
D256 division, the spike's exp/ln series, `pow` as `exp(y ln x)` at a fixed
77 digits, every result rounded once by the core from its digit string),
forwarded through the same manifests. It is experimental: not correctly
rounded in the strict sense (a 77-digit result within its own error of a
34-digit tie can round to the wrong neighbour), and the exact tie
`16 ^ -12.5` lands on whichever side the error falls on. Verified:
`cargo test -p rheon-feel-number --no-default-features --features fastnum`
228 passed, clippy clean, `cargo check --workspace --no-default-features
--features fastnum` succeeds. `dashu` stays the default.

## Taking upstream changes

Since the rename, upstream commits that touch the renamed crates no longer
apply as they are: their manifests and `use rheon_feel…` paths name the old
crates. Take upstream fixes one at a time, rewriting the patch through the
rename script before applying it:

```sh
git fetch upstream
git format-patch -1 --stdout <upstream sha> | perl scripts/upstream-rename.pl | git am -3
```

`upstream-rename.pl` rewrites `dsntk-<crate>` and `dsntk_<crate>` for every
renamed crate and leaves the others alone. A patch that touches `common`, `macros`,
`feel-grammar` or `recognizer` has no target here, because those crates are used from
crates.io: either wait for the next dsntk release and bump the `=0.3.0` pins in the root
manifest, or bring the crate back into the tree as `rheon-<crate>` and add it to the
rename script. Resolve the remaining conflicts by hand, run the
verification below, and record the cherry-pick in the table above. Whole
upstream releases are not merged any more.

## History rewrite of 2026-11-11

The 73 rheon commits from 2026-10-16 on were rewritten with `git filter-repo` to remove the
maintainer's home directory from four documentation commits and to give one commit the same
noreply author as all the others. Trees are unchanged; upstream's signed commits and the
`dsntk/*` tags are untouched. The commit hashes cited in the docs were updated in the same
push; crates.io still records the pre-rewrite hash of 0.2.0 (`3209ccd`, now `a91330d`).

## Publishing to crates.io

The ten `rheon-*` crates share the workspace version (`[workspace.package] version`,
rheon's own line; `0.1.0` was the first release, `0.2.0` added `rheon-feel-grammar`, `0.3.0` changed the API of
`rheon-feel` and `rheon-feel-evaluator`, `0.4.0` changes the API of `rheon-feel`, `rheon-feel-parser`
and `rheon-feel-evaluator`, see below) and pin each other with `=<version>` in
the root manifest. Every other workspace member is `publish = false`, so one command
publishes the set in dependency order:

```sh
cargo login                          # once: a crates.io token with the publish-new and publish-update scopes
cargo publish --workspace --dry-run  # packages and verify-builds everything, uploads nothing
cargo publish --workspace
```

crates.io rate-limits the creation of new crate names (a burst of five, then one every ten
minutes; the 429 response names the time to retry). A release that introduces new crates
therefore stops part-way, and `--workspace` then refuses to continue because the first
crates "already exist". `scripts/publish-crates.sh` handles both: it asks crates.io which
crates still miss the workspace version, publishes only those, and sleeps until the named
retry time. The first release (0.1.0, 2026-11-09) took about 35 minutes this way. For a release: bump the workspace version and the `=` pins in the root manifest
together, run the verification above, commit, tag `vX.Y.Z`, publish.

`rheon-feel-wasm` is the tenth published crate: its build script takes the commit from
`.cargo_vcs_info.json` inside a packaged crate and the upstream commit from its own manifest.
The built module reaches users through the npm package `@rheon/feel-runtime` (which ships
`feel.wasm`) and the GitHub Release that `.github/workflows/release.yml` creates for every
`vX.Y.Z` tag (`feel.wasm`, its sha256, the npm tarball and the playground build).

The API changes of 0.4.0 (commits 39-53):

- Changed:
  - `rheon-feel`: `Value::FunctionDefinition` holds an `Arc<FunctionDefinition>` instead of six
    fields. `Name` implements `Borrow<str>`, so `map.contains_key(&"d".into())` on a map keyed
    by `Name` no longer infers its type (write `&Name::from("d")`). The `Debug` text of a
    function value is the derived `Debug` of `FunctionDefinition`.
  - `rheon-feel-parser`: `ParsingScope<'a>` has a lifetime parameter (invariant) and borrows
    the input contexts; `From<&FeelScope>` is removed, so outside the crate a scope is built
    with `Default`; `pop()` returns `()`; `flattened()` is removed. `ParsingScope` is no longer
    `UnwindSafe`: it holds references to the input contexts, and `FeelContext` is not
    `RefUnwindSafe`, so a closure that uses a scope needs `AssertUnwindSafe` for
    `catch_unwind`. It is `Send` and not `Sync`, as at 0.3.0.
  - `rheon-feel-evaluator`: `ForExpressionEvaluator::evaluate` returns `Value` (was `Values`).
    `FilterExpressionEvaluator` has a private field, so it is built with `new()`, `default()`
    or `for_feel()`, not as `FilterExpressionEvaluator {}`.
- Added: `FunctionDefinition` (with `new` and `From<FunctionDefinition> for Value`);
  `FeelScope::{frames, depth, truncate, pop_frame}`; `FeelContext::{insert, assign_entry,
  get_entry_mut}`; `Value::{try_into_coerced, into_coerced}`; `Name::from_parts`;
  `bip::BUILT_IN_PROPERTY_NAMES`; `for_feel()` on `ForExpressionEvaluator`,
  `SomeExpressionEvaluator`, `EveryExpressionEvaluator` and `FilterExpressionEvaluator`.
- Kept: `FeelScope::contexts()`. The value types of `rheon-feel`, `AstNode`, `ClosureBuilder`,
  `FilterExpressionEvaluator`, `Dialect` and `ModelEvaluator` are `Send` and `Sync`, and
  `FeelScope` and `ParsingScope` are `Send`, as at 0.3.0; each crate's `send_sync` test checks
  this at compile time.

### The pre-rename procedure (historical)

Until 2026-10-28 the crates kept upstream's names and the plan was to merge or
rebase onto upstream releases; the procedure is kept for reference.


```sh
cd ~/dev/rheon
git fetch origin
git checkout main && git merge --ff-only origin/main     # keep main a mirror
git checkout wasm
git rebase main                                         # replay the commits
git rebase wasm fastnum-backend                         # optional side branch
```

`upstream` is <https://github.com/DecisionToolkit/dsntk> (the crates.io name is
`dsntk`; the old <https://github.com/dsntk/dsntk-rs> URL redirects there).
Upstream has not moved since `c76aee0`, so today the rebase is a no-op.
Replaying the branch onto a moved base (tried against the head of upstream
PR DecisionToolkit/dsntk#112, 27 commits ahead) first conflicts in commit 3
(`ec2ced3`, Robustness) in `feel-evaluator/src/filters.rs` and
`feel-parser/src/parser.rs` (one hunk each); commit 4 overlaps PR #112
itself, and commit 8 touches every manifest. Resolve conflicts per commit
(each one is small and self-contained) and compile every replayed commit:

```sh
git rebase -x 'CARGO_BUILD_JOBS=4 cargo check -p rheon-feel-evaluator' upstream/main
```

Then:

1. Rerun the verification above (suites, `tools/difftest/run.sh`,
   `tools/evaluator-corpus/run.sh`).
2. Rebuild and re-verify the wasm side in this repository: `npm test`,
   `npm run build:wasm`, `npm run smoke`, `npm run build` (the module's
   `engineCommit` follows HEAD automatically), and update `upstream-commit`
   in `wasm/feel-wasm/Cargo.toml` (`[package.metadata.rheon]`).

Since 2026-10-28 the engine commits live on `main` of this repository together
with the wasm, runtime and playground code, and the seven changed crates carry
`rheon-*` names, so this rebase procedure no longer applies as written; the
cherry-pick procedure above replaces it.

If a commit was merged upstream, `git rebase` drops it automatically when
the patch is identical; otherwise drop it by hand with `git rebase -i`.

## Layout note

This is the full upstream workspace (CLI, server, model evaluator, ...) so
that `cargo test` of any crate works unchanged. dsntk-wasm only needs
`common`, `macros`, `feel`, `feel-number`, `feel-temporal`, `feel-regex`,
`feel-grammar`, `feel-parser` and `feel-evaluator`; with the default
features none of them needs a C toolchain any more.
