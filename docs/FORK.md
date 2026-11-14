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
| 39 | `d39b7d6` | feel-parser: names found in an index over the borrowed input contexts; composite names searched, not flattened | upstream bug |
| 40 | `d39b7d6` | feel: the type of a list of contexts without building the item types | fork |
| 41 | `d39b7d6` | feel-evaluator: every evaluator that pushes a context restores the scope depth on return | upstream bug |
| 42 | `d39b7d6` | feel-evaluator: FEEL loops and filters move their contexts instead of cloning them | fork |
| 43 | `d39b7d6` | feel-number: decimal128 comparison, digit count and packing without 128-bit division | fork |
| 44 | `d39b7d6` | feel: function values share their definition (`Arc<FunctionDefinition>`) | fork |
| 45 | `d39b7d6` | feel-wasm: request input converted by value | fork |
| 46 | `d39b7d6` | feel-evaluator: subtraction null reasons from the evaluated operands (2^n - 1 diagnostics become n) | upstream bug |
| 47 | `d39b7d6` | wasm profile at opt-level 3 with a 10 MiB shadow stack | fork |
| 48 | `22afefd` | feel-parser: an iteration variable that starts with `in` is a syntax error, not a trap | upstream bug |
| 49 | `075af71` | feel-parser: a range literal's named temporal parameter keeps its name | upstream bug |
| 50 | `039128f` | feel-parser: list grammar actions append and complete each list once (the tails were quadratic) | upstream bug |
| 51 | `35f53e6`..`4aca592` except `cb925d0` | feel-parser: lexer and parser without per-token copies | fork |
| 52 | `cb925d0` | feel-parser: the composite-name search without recursion | fork |
| 53 | `6dcbbba` | feel-evaluator: a date range ending at the first or last supported date restarts at its start | upstream bug |
| 54 | `ff98977` | feel: strings share their text (`FeelString`) | fork |
| 55 | `5c2789f` | feel-wasm: test that no corpus request traps the module | fork |
| 56 | `cf5349d` | tools: identity check of full responses against the previous release | fork |
| 57 | `149beef` | ci: check response identity against the previous release | fork |
| 58 | `637cc8c` | tools: frozen temporal request corpus | fork |
| 59 | `77e25bf` | temporal: durations hold a symmetric range | upstream bug |
| 60 | `b4a7710` | temporal: a years and months literal is invalid when any component does not fit | upstream bug |
| 61 | `ae63142` | temporal: date and date-time arithmetic takes one signed step | upstream bug |
| 62 | `97decb9` | temporal: date plus or minus a days and time duration follows Table 58 | DMN 1.6 |
| 63 | `061ad26` | temporal: whole seconds of a duration are an i64 on every target | fork |
| 64 | `c6a7a69` | feel-evaluator: sort cannot panic on any predicate | upstream bug |
| 65 | `5fdaa0d` | feel-evaluator: one abort channel, reset and read by every entry point | fork |
| 66 | `8c4eee8` | feel: Display of lists, contexts and types writes as it goes | fork |
| 67 | `f1f09d6` | feel: one bounded renderer for message arguments and a cap per message | fork |
| 68 | `1f7c11e` | feel-evaluator: values in null reasons are previews | fork |
| 69 | `94eab15` | feel-parser: a syntax error quotes a preview of the input | fork |
| 70 | `032af5b` | model-evaluator: values in DMN messages are previews | fork |
| 71 | `30c2d9e` | feel-wasm: request echoes are previews and every message is capped | fork |
| 72 | `c7027b4` | regex: rheon-feel-regex tells engine limits from invalid patterns | fork |
| 73 | `a499385` | feel-evaluator: a regex engine limit aborts the evaluation | fork |
| 74 | `a27959d` | temporal: read fractional seconds as an exact integer | upstream bug |
| 75 | `2b9f135` | temporal: keep every time of day below 24:00:00 | upstream bug |
| 76 | `9ea1bf6` | feel-evaluator: build date and time(date, time, zone) from its parts | with 27 |
| 77 | `b67ace8` | diagnostics: report fractional seconds truncated to nanoseconds as warnings | fork |
| 78 | `e3af54d` | feel-evaluator: warn when time() seconds carry more than nine fractional digits | fork |
| 79 | `6c91a51` | model: bound how deeply the elements of a model nest | upstream bug |
| 80 | `e5a60d7` | model-evaluator: item definitions that form a cycle are a build error naming the cycle | upstream bug |
| 81 | `6512eef` | model-evaluator: an item definition type beyond the size or depth bound is a build error | upstream bug |
| 82 | `b25e023` | model-evaluator: decision-service function values hold a weak model reference | upstream bug |
| 83 | `078c1cd` | feel-wasm: display measures the inline width before rendering | fork |
| 84 | `3577ebe` | feel-wasm: no combination of edge operands traps the module | fork |

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
    fuzzer with the mpmath/decimal oracle and the FEEL corpus harness. Both
    were removed in item 94; the number oracle of item 90 judges the number
    core, and the corpus is kept in `tools/feel-corpus/`.
12. **`NUMBER_BACKEND`.** `rheon_feel_number::NUMBER_BACKEND` is `"dashu"` or
    `"intel"`, selected by the same feature as the backend itself; dsntk-wasm
    reports it as `numberBackend` in its `info` op. Since item 95 it is always
    `"dashu"`.
13. **Intel reference for `tools/`**: the patched `dfp-number-sys` crate the
    `tools/` harnesses build the Intel side with. It was vendored in full
    (17 MB, mostly Intel's C library); it is now `tools/dfp-number-sys.patch`
    on top of the crates.io 0.2.14 release, fetched by
    `tools/fetch-dfp-number-sys.sh`. The patch and the script were removed
    with the tools in item 94.
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
    longer than 40 characters are formatted as `9E+6144` in messages. (Other
    null reasons still quoted such numbers in full until commit 68 made the
    values in null reasons previews.)
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
    that turns pathological patterns into a null instead of a hang; since
    commit 73 a pattern beyond an engine limit stops the evaluation with an
    error);
    `replace`/`split` keep `regex`. With the `x` flag, whitespace inside
    `\p{...}` is stripped before compiling. TCK 1111 `K2-MatchesFunc-17`
    passes unpatched.

Commits 23-38 are described in their commit messages. Commits 39-54 make parsing and
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
    `5954a8f`) only right after a dot. One function decides which values carry nested names,
    for the index, the composite search and the contexts pushed while parsing (`4a6a585`); list
    item types are keyed by address, so `ParsingScope` is `Send` as at 0.3.0 (`74620b7`).
40. **List types.** `Value::type_of` of a list built the type of every item to find the common
    item type of a list of contexts. A context is now checked against a context type entry by
    entry, without building its type (`121517c`: one rule for the type of a list). The types are
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
    scope is restored (`910a535`); `ForExpressionEvaluator::evaluate` returns that `Value`.
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
    random values (`feel-number/src/decimal128/tests.rs`), and `tools/difftest/run.sh 100000` (removed in item 94)
    (3,900,001 cases) on this code gives the same mismatch set, cohorts and oracle verdicts
    against Intel as 0.3.0's core. The production code of `decimal128.rs` is unchanged since
    `d39b7d6`.
44. **Shared function definitions.** `Value::FunctionDefinition` holds an
    `Arc<FunctionDefinition>` (named fields, `FunctionDefinition::new`, `From<FunctionDefinition>
    for Value`) instead of six fields, so a name lookup or an argument copies a pointer. The
    invocation borrows the definition, and `closed_over` copies it (`Arc::make_mut`) only when
    the closure has entries to add. Arguments and results are coerced by value
    (`Value::try_into_coerced`, `into_coerced`, with the checks of `try_coerced`, `6e5b59d`).
    Without the copy of each argument the allocator placed strings differently, and a recursion
    that doubles a string 26 times hit the 512 MiB cap; commit 54 removes that (shared strings).
    `size_of::<Value>()` is 64 B natively (128 B at 0.3.0) and 64 B on wasm32 (as at 0.3.0:
    there the 48-byte `FeelNumber` sets it). `FunctionDefinition` derives `Debug` (`359a14a`),
    so the `Debug` text of a function value differs from 0.3.0's.
45. **Input by value.** The request's `context` and `input` are converted by value: strings
    and keys are moved (a key that trimming leaves unchanged becomes the `Name` without a
    copy), numbers are parsed from the JSON number's own text (`arbitrary_precision`), entries
    are inserted with `FeelContext::insert`, and the rest of the request is dropped before
    parsing. The path of a value in warnings and errors (`a.b[0]`) is written only when a
    message needs it, without recursion (`8dcba2b`). The texts and order of the warnings and
    errors are 0.3.0's (golden tests in `wasm/feel-wasm/tests/contract.rs`).
46. **Subtraction reasons.** The null reason of a failing subtraction evaluated both operands
    again to print them, so a chain of failing subtractions was exponential:
    `{a: "x", r: a - 1 - … - 1}.r` with 16 subtractions recorded 65,535 diagnostics
    (2^n - 1). The reason is formatted from the operands already evaluated: 16 diagnostics, with
    the same texts.
47. **Build profile.** `[profile.wasm]` builds at `opt-level = 3` (was `"z"`); its stack frames
    are larger, so the shadow stack is 10 MiB (was 8 MiB, `wasm/feel-wasm/build.rs`).
    `scripts/stack-headroom.mjs` (`npm run stack`, run by CI, `4fffdf0`) checks that five
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
    of helpers (`f8957b4`).
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
    the input contexts and the contexts pushed while parsing (`1e1f677`). A composite name
    2,000 levels down parses in a 256 KiB thread. Requests to feel.wasm cannot nest that deep
    (JSON contexts stop at about 125 levels, FEEL contexts at the AST limit), so this is
    robustness for direct callers of the library.
53. **Date range at the limit.** A date range that ends at the last supported date
    (262142-12-31, going up) or the first (-262143-01-01, going down) repeated its end date when
    an outer iteration context advanced, instead of restarting at its start: the iteration
    contexts are a cross product, inner to outer (DMN 1.6 §10.3.2.14).
54. **Shared strings.** `Value::String` holds a `FeelString`, a newtype over `Arc<String>`, so a
    name lookup, a string literal or a copied list item copies a pointer, not the text. The one
    operation that changes a string, `+` (FEEL and B-FEEL), appends in place when its left
    operand is not shared and otherwise allocates the result once at its final length. Every
    output, message and `Debug` text is unchanged. With commit 44's arguments moved instead of
    copied, `s + s` had left freed operand copies behind as holes too small for the next, longer
    string; now no operand is copied. A recursion that doubles a string argument 23 times grows
    linear memory by 32.06 MiB, 2.00x the 16 MiB result (0.3.0: 48.06 MiB, 3.00x), and 26
    doublings complete at a peak of 267.44 MiB (0.3.0: 393.44 MiB). Each string now carries a
    40-byte shared header: filtering 10,000 contexts (`list-filter-count`) makes 40,000 fewer
    allocations and peaks 3.3% higher natively (22.46 MB against 21.75 MB before this commit;
    51.6 MB at 0.3.0). Speed against the build before this commit: 1.006x on the geomean of
    the 24 requests.

Speed, measured 2026-11-13 on the final module (wasm, Node 26 on macOS arm64, median wall time
of a `feel_call` on a warm instance): 5.31x 0.3.0's speed on the geomean of 24 requests and
5.10x on 84 parse-heavy requests. For example, a formula over a 1,000-entry input context
7.13 ms → 0.46 ms; a 200-entry context literal with a 1,000-entry input context 188 ms →
1.36 ms; `x1 + … + x400` 55.0 ms → 0.29 ms; the 100-segment path `a.a.….a` 356 ms → 0.36 ms;
a 5,000-entry context literal 19.2 ms → 6.2 ms; `count(for i in 1..10000 return i)` 7.58 ms →
1.11 ms; filtering 10,000 contexts 38.3 ms → 11.1 ms. Full responses (result, type, warnings,
diagnostics, errors) of 1,174 requests (evaluate requests and a parse request made from each)
equal 0.3.0's except 52 registered differences, all from commits 39, 41, 46, 48, 49 and 53.

Commits 55-84 (0.5.0) make the engine robust at its limits: no input traps the module, every
message and evaluation is bounded, temporal values are exact at their edges and in their
fractions, and model loading is bounded. Each change of a wasm answer against 0.4.0 that the
identity check finds is registered with both answers and counted in its item, except the 544
number previews of the edge-operand product (item 84), instances of the registered message rule
that are not listed one by one; native-only changes are stated in their items with both answers.
The totals are under "Verification" below.

55. **No-trap corpus.** `wasm/feel-wasm/tests/corpus/no-trap.json` is a JSON array of module
    requests, exactly as a host sends them, and the contract test `corpus_requests_never_trap`
    sends each one through `feel_wasm::call`, natively and under Wasmtime. A request passes when
    the module answers it with JSON whose `ok` is a boolean and whose error, if any, is not
    `internal` (a caught panic), and when the next request, a function call (`abs(-1) + 1`),
    still answers `2`, so no call-depth or abort state lingers. Natively a panic fails the test
    with the request named; under Wasmtime a trap aborts the test binary, and the request is the
    last `no-trap:` line on stderr. Values are not asserted here: the corpus is also a request
    source of the identity check (commit 56), which pins every answer. The 66 initial requests
    are inputs at the engine's limits that 0.4.0 answers: temporal values at the edges of the
    supported range with checked operations, month-end arithmetic, regex patterns at the engine
    limits (catastrophic backtracking, `\w{10000}`, 65 nested groups), expressions at and beyond
    the 300-level nesting limit (as `evaluate` and `parse`, and as a FEEL context), recursion at
    and beyond the 200-call limit, and `sort` with consistent predicates on 0 to 10,000 items. An
    input that traps the module is added here in the commit that fixes it and stays as a
    regression seed. The test runs on the same 512 MiB native thread as
    `nesting_and_recursion_limits`, through one shared helper, because unoptimized native test
    builds use much bigger stack frames than the release module and a native stack overflow
    aborts the test binary instead of failing one test. Basis: no input may panic the engine
    (CONTRACT §1: on wasm a panic aborts the instance). No API change.

56. **Identity check against the previous release.** `tools/identity/` holds the check that every
    full response of a candidate `feel.wasm` equals the previous release's, except the differences
    registered with their basis. `run.sh <reference.wasm> <candidate.wasm>` sends every committed
    request source (so far the no-trap corpus of commit 55, each `evaluate` request also as
    `parse`) through both modules, each request byte for byte as written and every number read
    losslessly from its source text, and compares the whole responses, without `elapsedMicros`,
    `engineCommit` and the `version` of `info`; a trap is the response `{"trap": true}`. The
    ledger `registered/<change>.json` lists, per change that alters answers, each request with its
    `why` and the `old` and `new` answer, as the result text or the whole response. A difference
    that is not registered fails the run, and so does a registered entry that does not reproduce
    on both sides, a request registered twice, or a malformed ledger, so the ledger cannot drift
    from the code. `--record` drafts the entries of a change from a run. `baseline.json` names the
    release (`v0.4.0`) and the sha256 of its `feel.wasm` asset, and `fetch-baseline.sh` downloads
    that asset and fails closed on a different sum. Checked on 0.4.0: the module against itself
    exits 0; a registered entry with a wrong `new`, and one that does not differ, exit 1; 0.3.0
    against 0.4.0 exits 1 with its differences listed, and 0 once they are recorded and passed as a
    ledger; a malformed ledger, a duplicate registration, a mixed or invalid `old`/`new` and a
    request that calls `now()` exit 2. Basis: answers stay identical to the previous release except
    the changes registered with both answers. No API change.
57. **The identity check in CI.** The `wasm` job runs `tools/identity/fetch-baseline.sh` and
    `tools/identity/run.sh .tools/baseline/feel.wasm dist/feel.wasm` right after
    `npm run build:wasm`, with `GH_TOKEN` from the workflow's token for the release download. A
    pull request that changes an answer without registering it fails there. Checked locally with
    the same commands: a build whose call-depth message differs by one word fails the step with
    the four requests of the corpus that quote it; the unchanged build passes it; a download with
    a different sha256 is deleted and fails the step, and so does a failed download. No API
    change.
58. **Frozen temporal corpus.** `tools/identity/requests/temporal.json` holds 440 temporal requests
    and no expected answers: the previous release answers them, and every change of an answer is
    registered with the change that makes it. It covers fractional seconds of 1 to 9 digits (all
    zeros, a final 1, all nines, fractions whose binary product is known to be off, exact ones) and
    of more than 9 digits, in each literal form (`time`, `date and time` and `duration` texts and
    `@"…"` literals); the exact constructor paths (`time(h, m, s)` with fractional and computed
    seconds, `date and time(date, time[, zone])`) and carries at 24:00 and at the end of a year;
    the range edges of dates, date-times and both duration kinds, year 0 and years below 1000;
    daylight saving gaps and overlaps (Europe/Paris, America/New_York, the 30-minute shift of
    Australia/Lord_Howe, the skipped day of Pacific/Apia, a date beyond the zone rules' horizon);
    zoned against zone-less comparisons; and offsets at and beyond ±18:00, with seconds and with
    invalid minutes. With the evaluate requests also sent as parse requests and the no-trap corpus,
    the identity check sends 1,008 requests, all answered without a trap and identical between the
    build and 0.4.0. No API change.
59. **Durations hold a symmetric range.** A years and months duration holds -(2^63 - 1) to
    2^63 - 1 months, a days and time duration -(2^63 - 1) to 2^63 - 1 nanoseconds. Both types
    accepted -2^63 before, whose absolute value does not exist in an `i64`: its `Display`, `abs`
    and components overflowed, which trapped the module (`duration("P1M") *
    -9223372036854775808`, its `string`, `abs` and negation, a date or date and time plus or minus
    it, the `.days`, `.hours`, `.minutes`, `.seconds` of the days and time twin), or the saturating
    absolute value printed it one nanosecond off (`duration("PT0.000000001S") *
    -9223372036854775808`, a date and time difference of exactly -2^63 ns, sums and quotients that
    land there). Every constructor from a raw count now checks the range, so `abs` and negation
    are total, and a product, quotient, sum or difference of -2^63 is null with the reason of
    every other out-of-range result. The `value` of a date and time is computed from the instant
    difference in nanoseconds itself, not through a duration, so
    `date and time("1677-09-21T00:12:43.145224192Z").value` stays the exact `-9223372036.854775808`
    (DMN 1.6 §10.3.2.3.5: valuedt is the number of seconds since the epoch). The inputs that
    trapped are in the no-trap corpus; 34 registered differences, each with both answers, 10 of
    them combinations of the edge-operand product of item 84 (sums, differences and products of
    exactly -2^63 nanoseconds or months). Basis: no input may panic the engine (CONTRACT §1). API
    breaks (rheon-feel-temporal): `FeelYearsAndMonthsDuration::from_ym` and `from_m` and
    `FeelDaysAndTimeDuration::from_n` and `from_sn` return `Option<Self>`;
    `FeelDaysAndTimeDuration::from_s` takes an `i32`; the `ops::Add` and `ops::Sub` impls of both
    durations are removed (use `checked_add` and `checked_sub`), `checked_neg` is removed and
    `ops::Neg` is total; both derive `Default` (zero); `&FeelDate - &FeelDate` returns
    `Option<FeelYearsAndMonthsDuration>`; `FeelDateTime::nanos_since` is added.
60. **A years and months literal is invalid when any component does not fit.** The parser
    accepted a literal when one of its components was valid and silently left out a component
    that overflowed: `duration("P768614336404564650Y8M")` was `P768614336404564650Y`,
    `duration("P99999999999999999999Y3M")` was `P3M` and `duration("P1Y99999999999999999999M")`
    was `P1Y`. A literal is now valid when it has a component and every component it has fits the
    duration range, so these are null with `duration: invalid duration literal: '…'` (an `@"…"`
    literal with `invalid @ literal: …`). Basis: DMN 1.6 §10.3.2.3.8, a years and months duration
    is its years and months, valueymd = 12 × years + months; the literal must not mean a
    different duration. 6 registered differences. No API change.
61. **Date and date and time arithmetic takes one signed step.** A date or a date and time plus
    or minus a years and months duration moves the date by the signed number of months in one
    function, and a date and time plus or minus a days and time duration moves the instant by the
    signed number of nanoseconds with a checked step. Before, the sign was handled by `+` and `-`
    calling each other with the absolute value, and subtracting a days and time duration used
    chrono's `-=`, which panics outside the supported instants:
    `date and time("-262143-01-01T00:00:00Z") - duration("P1D")` and
    `date and time("-262143-01-01T00:00:00Z") + duration("-P1D")` trapped the module. They are null
    now, with the reasons of the same results after the last supported instant. The month step
    keeps the day of the month and takes the last day of a shorter month, as before: DMN 1.6 §9.4
    gives these operations the semantics of op:add-yearMonthDuration-to-date and
    op:subtract-yearMonthDuration-from-date (XML Schema Part 2, Appendix E), while Table 58's
    formula `date(…, e1.day)` has no value for such a day. Nothing in rheon-feel-temporal calls a
    chrono operation that can panic. Basis: no input may panic the engine (CONTRACT §1); Table 58,
    `valuedt⁻¹(valuedt(e1) ± valuedtd(e2))`. 8 registered differences, 5 of them combinations of
    the edge-operand product of item 84 (the first supported instant minus a duration). API break
    (rheon-feel-temporal): `FeelDate::add_months(u32)` and `FeelDate::sub_months(u32)` are removed;
    the signed month step is internal to the crate.
62. **Date plus or minus a days and time duration follows Table 58.** DMN 1.6 Table 58 defines it
    as `date(valuedt⁻¹(valuedt(e1) ± valuedtd(e2)))`, with the date taken as the date and time at
    UTC midnight (§10.3.2.3.5): the result is the date of that instant, so the signed nanoseconds
    are rounded down to whole days in one step. Before, whole days were counted from the unsigned
    components and, going back, one more day was taken only for a remainder of at least one
    second, so a sub-second remainder was ignored: `date("2020-01-02") - duration("PT0.5S")` was
    `2020-01-02` and is `2020-01-01` now, `date("2020-01-02") + duration("-PT0.000000001S")` is
    `2020-01-01` and `date("2020-01-02") - duration("P1DT0.5S")` is `2019-12-31`. Every other
    result is unchanged. Basis: DMN 1.6 Table 58; owner decision to follow it for the sub-second
    remainder. 12 registered differences, 9 of them combinations of the edge-operand product of
    item 84: a date minus one nanosecond is the day before (6), and from the first supported date
    it is out of range and null (3). No API change.
63. **Whole seconds of a duration are an `i64` on every target.**
    `FeelDaysAndTimeDuration::as_seconds` returned an `isize`, 32 bits on wasm32, so `time(h, m,
    s, offset)` read an offset of 2^32 seconds or more modulo 2^32 there:
    `time(10, 0, 0, duration("PT4294967296S"))` was `10:00:00Z` on wasm and null natively. Every
    target now answers null, `time: offset is out of range: …`, as 64-bit native builds did; the
    positions and lengths of the built-ins already go through 64-bit integers for the same
    reason (CONTRACT §5). Basis: results never depend on where the core runs. 4 registered
    differences (wasm only). API break (rheon-feel-temporal): `FeelDaysAndTimeDuration::as_seconds`
    returns `i64` (was `isize`).
64. **`sort` cannot panic on any predicate.** `sort` passed the user's `precedes` to the standard
    library's stable sort, which may panic when the comparison is not a total order (documented
    since Rust 1.81), and does on 1.95: `sort(for i in 1..33 return modulo(i, 7), function(x, y)
    modulo(x * 7, 5) < modulo(y * 3, 5))` trapped the module. DMN 1.6 §10.3.4.9 (Table 81) states
    no condition on `precedes`, so `sort` now owns its ordering (`feel-evaluator/src/bifs/sorting.rs`,
    no recursion, no `unsafe`, positions instead of moved values): up to 20 items by insertion,
    with the same calls of `precedes` as before; longer lists in runs (a run in which each item
    precedes the one before it is reversed, a run in which none does is kept, shorter runs are
    extended to 20 items by binary insertion), merged level by level, the right item first only
    when it precedes the left one, with galloping searches once one side wins 7 times in a row.
    For every predicate the result is a permutation fixed by the list and the answers of
    `precedes`; for a strict weak order it is the stable sort, so every such result is
    unchanged. An answer other than `true` still means "does not precede". Calls of `precedes`
    for 10,000 items against the library sort: pseudo-random 0.82-0.87x, ordered and reversed
    equal (9,999), a sorted list with a random tail 0.59x, seven distinct values 1.33x. Speed on
    the wasm module (median of 15 runs, Node, against 0.4.0): `count(sort(for i in 1..10000
    return modulo(i * 7919, 10007), function(x, y) x < y))` 44.2 ms → 39.3 ms (0.89x the time),
    the ordered list 1.03x, the reversed list 1.01x, 10,000 items of seven distinct values
    19.7 ms → 24.2 ms (1.23x), 1,000 items 0.90x, 20 items 1.00x. Basis: no input may panic the
    engine (CONTRACT §1). 3 registered differences: the trapping input, and two predicates that
    are not strict orders on 200 and 1,000 items, whose permutation differs. No API change.
65. **One abort channel, owned by one evaluation.** The call-depth limit was a sticky thread-local
    flag that only the wasm layer reset and read. `ModelEvaluator::evaluate_invocable` reset it
    and never read it, so a DMN evaluation that hit the limit served its cut-short value as the
    result (`null(function call depth limit (200) exceeded)`), and `evaluate_context`,
    `evaluate` and the other FEEL entry points neither reset nor read it: after one runaway
    request body, every function call of the native server's next body on that worker thread
    was null. Now `rheon_feel::diagnostics` holds one evaluation state (the call depth and the
    abort that stopped the evaluation, first reason kept) that is clear outside an evaluation
    and cleared by RAII when the evaluation ends, also on a panic. Every entry point opens or
    joins an evaluation (`Session::run` in rheon-feel-evaluator) and reports an abort instead of a
    value: `evaluate` and `ModelEvaluator::evaluate_invocable` return an `Outcome`,
    `evaluate_context` and the range-literal entry points return `Err` (`<EvaluationAborted>
    function calls are nested too deeply (more than 200 levels, e.g. runaway recursion);
    evaluation stopped`), and the server, the TCK handler and the CLI report it as an error. A
    prepared evaluator and a decision table evaluator run as one evaluation: inside a session
    they join it, outside one a stopped evaluation is null with the abort's text, so no partial
    value reaches an enclosing expression (`{f: function(n) f(n + 1), r: f(1)}.r = null` was
    `true`) and no hit policy answers after a limit stopped one of the table's entries (a FIRST
    table whose first rule's input entry recursed without end answered its second rule's
    output). A refused call returns a bare null that is never observed. Basis: DMN 1.6 §7.3.8
    (the evaluation reports diagnostic information; a limit is not a FEEL result), CONTRACT §3
    (a stopped evaluation is reported as an `evaluation` error, never as a partial result), and
    the rule that no state outlives one request. The wasm responses are identical, the call-depth
    `evaluation` and `context` errors included; the changed answers are native only: the
    runaway context body and the next body on the same thread, the DMN invocable, the CLI,
    and a prepared evaluator and a decision table evaluator outside a session. API breaks:
    rheon-feel adds `diagnostics::{Abort, evaluation, one_evaluation, abort, enter_call, Call}`
    and `From<Abort> for DsntkError`, with `Abort` and its variants `#[non_exhaustive]`
    (`Abort::limit` builds the engine-limit variant); rheon-feel-evaluator removes
    `reset_call_depth` and `call_depth_exceeded`, adds `Session` and `Outcome`, `evaluate` returns
    `Outcome<Value>`, `evaluate_context`, `evaluate_context_node`, `evaluate_range_literal` and
    `evaluate_range_literal_node` return `Err` on an abort, and `prepare` and
    `prepare_with_dialect` evaluators turn an abort into a null with its text;
    `evaluate_invocation` joins a running session, and outside one it is an evaluation of its
    own, whose stop is null with the abort's text (was `null(function call depth limit (200)
    exceeded)`, with the flag left set for the next caller on the thread);
    rheon-model-evaluator: `evaluate_invocable` returns `Outcome<Value>` and
    `evaluate_invocable_with_mode` returns `Outcome<(Value, Diagnostics)>`, and the evaluator of
    `build_decision_table_evaluator` turns an abort into a null with its text; unpublished:
    `dsntk-evaluator` re-exports `Session` and `Outcome`, `Workspaces::evaluate` returns `Err` on
    an abort.
66. **Display of lists, contexts and types writes as it goes.** The `Display` of a list (and of an
    expression list), of a context and of a context or function type built the text of every
    element, joined them into one `String` and then wrote it, and a range or a null with a
    reason built its parts the same way. A writer that stops early (a bounded message) therefore
    still paid for the whole text: a 100,000-item list offered all 688,895 bytes to a writer that
    wanted 200. Each element is now written straight to the formatter, separated by `, ` in one
    helper, so formatting stops at the first refused write. Every text is byte for byte the same.
    Basis: messages must be bounded in time and memory, not only in length. No API change.
67. **One bounded renderer for message arguments and a cap per message.** A null reason was
    `format!` of whatever its arguments printed, so a reason that quoted a long list, context or
    pattern held all of it: `(for i in 1..1000 return i) = 1` gave a 4,913-character reason, and
    1,000 comparisons of a 100,000-item list (`{big: for i in 1..100000 return i, r: for j in
    1..1000 return big = j}.r`) built about 690 MB of reasons and trapped the module at its 512 MiB
    cap. The new module `rheon_feel::message` has the one renderer and the one funnel: `arg` quotes
    a value, type, context, name or text whole when its text is at most 120 characters and as a
    preview otherwise (its first 120 characters, `…` and its size: `N items`, `N entries`, `N
    characters`), and a number whose plain text is longer than 40 characters in scientific notation;
    `compose` builds a message of at most 1,000 characters, a longer one keeping its first 999 and
    `…`, and stops formatting there. `value_null!` builds every reason through `compose`, so every
    reason is bounded by construction, even at a site that quotes a value without `arg`, and builds
    it out of line, in `values::null_with_reason`, which is cold and never inlined: a reason's
    arguments and formatting take no space in the stack frame of the code that returns the null,
    often an evaluator that recursion nests deeply. The recursion of the stack check
    (`scripts/stack-headroom.mjs --report`, a body nested d levels in 190 calls) reaches at least
    the depths of 0.4.0 on every host (0.4.0 → this build, macOS arm64): the Node main thread 17 →
    18 with Liftoff and 14 → 14 with TurboFan, the 2 MiB worker 37 → 40 and 31 → 31, the 3 MiB
    worker 60 → 64 and 50 → 50, the 4 MiB worker 64 → 89 and 64 → 69, Wasmtime with 512 KiB, 1, 2
    and 4 MiB 9, 23, 52, 64 → 10, 25, 57, 98; a level of `+` takes 227 bytes of the Liftoff stack,
    243 before. The type of a value is quoted as `values::TypeOf`, which writes the text of
    `type_of()` from the value itself: a context type entry by entry, a list type from its first
    item's type built with at most 1,000 type nodes (`list<…>` when it would hold more), declared
    types as they are; `type_of()` itself is the same body with an unlimited budget. The
    `MessageArg` impls live with their types. With streaming `Display` (item 66), a message
    allocates a bounded amount, never a whole value or type. This commit caps; the sites quote
    previews from the next ones on. 5 answers change: four reasons over 1,000 characters (a list
    compared with a number, in lenient and strict mode, a missing path entry of a long context, an
    invalid pattern of 5,001 characters) are cut, and the evaluation of 1,000 such reasons, which
    trapped, answers; `scripts/smoke.mjs` checks it on a fresh instance (22 MiB of growth; 0.4.0
    traps at 500 MiB). Item 68 gives four of them previews and registers them with those answers, so
    1 registered difference is this item's: the reason of the invalid pattern, which stays cut.
    Speed against 0.4.0: the suite geomean 1.00x, 1,000 error nulls 1.00x. Basis: messages name the
    value by a short preview and its size, at most 1,000 characters each (owner decision); CONTRACT
    §3 (numbers over 40 characters in scientific notation). API breaks (rheon-feel): `message::{arg,
    Arg, compose, MessageArg, Size}`, `values::TypeOf` and `values::null_with_reason` are added,
    with `MessageArg` for `TypeOf` and for the existing `Value`, `FeelType`, `FeelNumber`, `Name`,
    `FeelContext`, `Bip`, `str`, `String` and `serde_json::Value`; `value_null!` caps its message at
    1,000 characters and builds it in a closure that it passes to `values::null_with_reason`, so
    the macro's arguments are evaluated inside that closure and can no longer use `?`, `return` or
    `break`.
68. **Values in null reasons are previews.** Every null reason of the evaluator and of rheon-feel
    quotes its values, contexts, names, texts and numbers through `arg`, and the type of a value
    through `TypeOf`: the 199 `invalid_argument_type!` sites pass the value instead of its
    `type_of()`, and so do the other type messages (operators, `between`, `string join`,
    `sublist`, `time`, the iteration and range errors). A message therefore never builds the
    type of a whole context: `abs(c)` for a 10,000-entry context allocated the 10,000-entry type
    (about 1.1 MB) for its reason and now allocates at most 64 KiB more than reading `c`. The
    reason of `(for i in 1..1000 return i) = 1` is `equal err '[1, 2, 3, …, 32, … (1000 items)'
    =?= '1'` (the first 120 characters of the list, then its size), a missing entry of a long
    context quotes the context's preview with `… (1 entry)`, and a number whose plain text is
    longer than 40 characters is in scientific notation in every reason, as CONTRACT §3 already
    said for `sublist` and `substring`: `1e50 / "a"` gives `[division] incompatible types: 1E+50
    / "a"` (and two pinned tests, a duration divided by 1e-50, now read `… / 1E-50`). A short
    argument (a number, a boolean, a temporal value, a string of at most 118 bytes) is written
    straight, with no preview to watch it, and a long one streams its first 120 characters, so
    quoting allocates nothing. Speed against 0.4.0: the suite geomean 1.00x; 1,000 reasons that
    each quote two values and their types 0.93x (each operand now passes the renderer), 1,000
    reasons quoting a list or a context 1.06x and 1.16x. 5 registered differences: four answers
    of item 67, now with previews and registered here with them (the reasons of a list compared
    with a number in lenient and strict mode and of a missing path entry, and the evaluation of
    1,000 such reasons), and `1e50 / "a"`; the reason of a 5,001-character invalid pattern stays
    cut and registered with item 67, since the regex library quotes it. Basis: messages name the
    value by a short preview and its size (owner decision); CONTRACT §3. No API change beyond
    rheon-feel's `MessageArg::is_short` (with a default) and `MessageArg` for references and
    `FeelString`.
69. **A syntax error quotes a preview of the input.** The parser's syntax error echoed the whole
    input, so a 40,002-character expression with a trailing operator gave a 40,016-character
    `syntax` error, for `evaluate` and `parse` alike. `err_syntax_error` and
    `err_not_a_feel_name` now quote the input through `arg`: whole up to 120 characters, else its
    first 120 characters, `…` and `(40002 characters)`. Every shorter input keeps its full echo.
    2 registered differences (the evaluate and the parse request). Basis: messages name the input
    by a short preview and its size (owner decision); CONTRACT §3. No API change.
70. **Values in DMN messages are previews.** The item-definition, variable, boxed-expression and
    invocable messages of rheon-model-evaluator quoted whole values and built whole types: an
    input of the wrong type quoted all of it twice, as `expected type 'string', actual type is
    '<its whole type>' in value '<its whole text>'`. They now quote through `arg` and `TypeOf`, so
    `{Customer Name: for i in 1..1000 return i}` against a string item definition gives
    `expected type 'string', actual type is 'list<number>' in value '[1, 2, 3, …, 32, … (1000
    items)'`, and a 10,000-entry context gives the previews of its type and its text, each
    ending `… (10000 entries)`. The strict-mode text of `evaluate_invocable_with_mode` is built by
    `compose`, at most 1,000 characters. Native only (DMN is not in the wasm module); the TCK
    results are identical. Basis: messages name the value by a short preview and its size
    (owner decision). No API change.
71. **Request echoes are previews and every message is capped.** The wasm layer quoted request
    values whole: an invalid `dialect`, `language`, `errorMode`, `contextFormat`, `op` or
    `tzOffsetMinutes` was echoed in full, a JSON number's text and the path of a value in the
    input messages too, and a rounded input number in its plain digits
    (`a[0]: number 1.5e-6176 is too small for a 34-digit decimal and was rounded to 0.<6175
    zeros>2`). They now quote through `arg` (`invalid 'dialect' "xxx…… (200 characters) (expected
    "expression" or "unaryTests")`, `… was rounded to 2E-6176`); the path of an input value is
    written as it goes, never built as a text, and a long one is a preview with its size in
    characters in every message; the FEEL-context error quotes its value and `TypeOf`, and every
    `error.message`, warning and strict-mode text goes through `compose`, so no message of a
    response exceeds 1,000 characters (CONTRACT §3 states the rule). 11 registered differences: the
    echoes of a long `dialect` and `contextFormat` (each for evaluate and parse), `op` and a
    1,000-digit `tzOffsetMinutes`, the empty-key error under a 2,000-character key (evaluate and
    parse) and the number warning under it, a 5,002-character number and the rounded `1.5e-6176`;
    the identity check sends these numbers as written. Basis: messages name the value by a short
    preview and its size (owner decision); CONTRACT §3. No API change.
72. **rheon-feel-regex tells engine limits from invalid patterns.** The regex engines give up on
    some valid patterns: fancy-regex's backtracking budget and stack, the compiled-size limit of
    the automata, and the nesting limits of the parsers (64 levels in fancy-regex, 250 in
    regex-syntax). Every such failure was reported as an invalid pattern. The crate now has a
    typed `RegexError`: `Invalid` (an invalid pattern or flags, XQuery F&O FORX0001-FORX0004,
    whose FEEL result is null) or `LimitExceeded(Limit)` with `Limit::{Backtracking,
    BacktrackingStack, CompiledSize, Nesting}`. A pattern is called invalid only when the engine
    finds it invalid within its limits; one the engine cannot read within them is a limit:
    fancy-regex's recursion error, a build error with a size limit or a nest-limit syntax error,
    and every run-time failure are limits; the `regex` crate's `CompiledTooBig` is one, and on
    its syntax-error path the pattern is parsed again with the engine's defaults to tell a
    nesting limit apart. So a pattern that nests past fancy-regex's limit and is invalid further
    on (300 `(` and then `[`) is a nesting limit in `matches`, whose parser stops at the limit,
    and invalid in `replace` and `split`, whose parser reads the whole pattern before it checks
    the nesting. `replace` and `split` move into the crate with the replacement-group rewriting,
    the `q` quoting and the fallback to the pattern without flags when the flagged pattern is
    invalid, as they were; a limit on the flagged pattern is reported at once instead of falling
    back, since the flags apply to the pattern (XQuery F&O 3.1 §5.6.3, `fn:replace` takes the
    flags of `fn:matches`): `replace("x", "k{100000}", "y", "i")`, where only the flagged pattern
    exceeds the compiled-size limit (case folding gives `k` the Kelvin sign as a third form), was
    the answer of the pattern without flags, `"x"`. That answer change is registered with item 73,
    which gives it its final form. The built-ins keep their argument checks and messages, and
    every other FEEL answer is unchanged in this commit (the identity check includes the limit
    inputs). API breaks (rheon-feel-regex): `RegexError`, `Limit`, `replace` and `split` are added,
    and `regex` and `regex-syntax` become dependencies (both already in the build graph);
    rheon-feel-evaluator no longer depends on `regex`.
73. **A regex engine limit aborts the evaluation.** `matches`, `replace` and `split` turned an
    engine limit into the null of an invalid pattern, which B-FEEL then replaced by `false`, `""`
    or `[]`: `matches("aaaa…!", "^(a|a)*\1$")` (40 `a`) was null with `invalid regex pattern`,
    the same `or true` in B-FEEL was `true`, `split("a", "\w{10000}")` in B-FEEL was `[]`, and a
    DMN decision silently did not match. An implementation limit is not an XQuery F&O error, so
    DMN 1.6 Table 75 note 2's null does not apply, and chapter 11's defaults replace FEEL nulls,
    which an abort is not. A `LimitExceeded` from rheon-feel-regex now stops the evaluation
    through the abort channel (item 65) with `<function>: the pattern '<preview>' exceeds the
    regex engine's <limit>; evaluation stopped`; an invalid pattern keeps its null and text. The
    wasm answer is an `evaluation` error, a `context` error inside a FEEL-format context, and DMN
    reports the abort (`Outcome`, the server's `errors`). The backtracking limit of item 22
    therefore ends the evaluation as an error. CONTRACT §2 and §3 list the new cause. 18
    registered differences: the compiled-size, nesting and backtracking limits of the three
    functions, with and without flags, in FEEL and in B-FEEL, among them two inputs of the
    no-trap corpus; `replace("x", "k{100000}", "y", "i")`, where only the flagged pattern exceeds
    the limit (item 72), was `"x"`; and `matches` of 300 `(` and then `[`, invalid beyond the
    nesting limit, was the null of an invalid pattern. Basis: DMN 1.6 §7.3.8 and Table 75 note 2;
    CONTRACT §3 (a stopped evaluation is reported, never a partial result). API break
    (rheon-feel-regex): `is_match` and `is_match_with_flags` return `Result<bool, RegexError>`.
74. **Exact fractional seconds.** The time, date and time and days and time duration parsers read
    the fraction digits as a binary double and truncated the product with 10^9, which is one
    nanosecond low for many decimal fractions: of all fractions with 4, 5 and 6 digits, 167 of
    10^4, 1,710 of 10^5 and 17,066 of 10^6 (none with 1 to 3 digits), so `time("10:00:00.0157")`
    was `10:00:00.015699999`, `duration("PT0.000015839S")` was `PT0.000015838S` and
    `time("10:00:00.0157").value` was `36000.015699999`. A fraction that the double rounds to 1
    gave 10^9 nanoseconds: `time("10:00:00.99999999999999999")` showed the malformed
    `10:00:00.`, `…T23:59:59.99999999999999999Z` showed `…T23:59:59.Z` and
    `duration("PT1.99999999999999999S")` was `PT2S`. The digits are now read as an integer in one
    function shared by the three parsers: the first nine are the nanoseconds, the digits beyond
    them are truncated, as before. `date and time(date, time, zone)`, which formats its parts and
    parses the text again, is exact with it. Tests: every fraction of up to six digits and 100,000
    of seven to nine digits against the integer of their digits. Basis: DMN 1.6 §10.3.2.3.4,
    §10.3.2.3.6 and §10.3.2.3.7, a time, a date and time and a days and time duration are values of
    the XML Schema time, dateTime and dayTimeDuration datatypes, whose seconds are decimals. 1,508
    registered differences, each the exact digits of a fraction the double got wrong. No API
    change.
75. **Every time of day is below 24:00:00.** The time constructors checked the hour, minute and
    second but not the nanoseconds, and 24:00:00 was accepted with any fraction:
    `time("24:00:00.5")` and `@"24:00:00.5"` were `00:00:00.5`, `date and time("2020-01-01T24:00:00.5Z")`
    was `2020-01-02T00:00:00.5Z`, and `FeelTime::local_opt(10, 0, 0, 1_000_000_000)` built a time
    whose text was `10:00:00.`. A time of day now has an hour below 24, a minute and a second below
    60 and nanoseconds below 10^9, or is 24:00:00 with a zero fraction, which is midnight; one
    constructor checks it and every other one goes through it, `utc_opt` included, which kept an
    hour of 24 as it was. So these three are null with the reasons of other invalid literals,
    while `24:00:00`, `24:00:00.000` and `2020-12-31T24:00:00Z` (2021-01-01T00:00:00Z) are
    unchanged. The clock time of `now()` folds a leap second, which chrono gives as nanoseconds
    from 10^9 on, into the last nanosecond of the second before. Basis: XML Schema Part 2 §3.2.7,
    "'24' is permitted if the minutes and seconds represented are zero"; DMN 1.6 §10.3.2.3.4, a
    time is a value of the XML Schema time datatype. 3 registered differences. API breaks
    (rheon-feel-temporal): the unchecked `FeelTime::utc`, `FeelTime::local` and `FeelTime::offset`
    are removed and `FeelTime::midnight(FeelZone)` is added; `FeelTime::{utc_opt, local_opt,
    offset_opt, zone_opt}` and `FeelDateTime::{utc, local, offset}` return `None` for nanoseconds
    of 10^9 or more and for 24:00:00 with a fraction, and `utc_opt` turns 24:00:00 into midnight;
    the time and date and time parsers reject 24:00:00 with a fraction.
76. **`date and time(date, time, zone)` is built from its parts.** It formatted the date, the time
    and the zone as a literal and parsed that text again, so a date whose year has fewer than four
    digits, which `FeelDate` prints as `0999` or `-005` and the literal pattern does not accept,
    failed as `[core::date and time] invalid timezone 'Z'`. It now reads the zone designator alone
    with the rules of a literal's zone suffix (`FeelZone::from_designator`: `Z`, an offset within
    ±14:00, `@` and a zone name the zone database knows; never empty) and builds the date and time
    from the date and the time of day in that zone. Every input the literal could write gives the
    same answer, its failures the same `invalid timezone` reason; `date and time(date(999,12,31),
    time(23,59,0), "Z")` is now `0999-12-31T23:59:00Z`, the value `date and time(d, t)` already
    gave with a zoned time, and year 0 and the negative years above -1000 likewise, as
    `date(0,1,1)` gives them. Basis: DMN 1.6 §10.3.2.3.6, "The year must be in the range
    [-999,999,999..999,999,999]"; §10.3.4.1 Table 73. 4 registered differences (years 999, 0 and
    -5). API break (rheon-feel-temporal): `FeelZone::from_designator` is added.
77. **Fractional seconds truncated to nanoseconds are reported as warnings.** A time, date and
    time or days and time duration holds nanoseconds, so the digits beyond the ninth were dropped
    without a word: `time("10:00:00.1234567891")` is `10:00:00.123456789`. They are still
    truncated, and the evaluation now says so: the diagnostics collection of `rheon_feel` keeps a
    list of warnings beside its errors, filled by `diagnostics::warn` while a collection is active,
    with the same rules as the error messages (a repeat in a row once, at most 64, restored when a
    collection ends, also on a panic) but neither counted nor read by strict mode, since a
    truncation is no semantic error (DMN 1.6 §7.3.8 makes semantic errors the reason to report and
    return null). The three temporal parsers report the digits they dropped
    (`parse_reporting_truncation`, which `FromStr` and `TryFrom` call), and every producer quotes
    the literal or text through one message, `fractional seconds of '10:00:00.1234567891' truncated
    to nanoseconds; dropped digits: '1'`, built with `arg`, so a 100,000-digit fraction gives a
    message of a few hundred characters: an `@"…"` literal (parsed once when the expression is
    built, warning on each evaluation), `time`, `date and time` and `duration` of a string, and the
    `xsd:time`, `xsd:dateTime` and `xsd:duration` conversions of typed input. Only digits that hold
    a value warn: `10:00:00.1234567890` does not. The wasm module appends the evaluation's warnings
    to the response's `warnings` after those of the input and the unary tests, including the
    warnings of a FEEL-format context; `diagnostics.count` and strict mode are unchanged, and an
    evaluation stopped by a limit still answers its error without warnings. Native callers read
    them from `Diagnostics::warnings`, also from `ModelEvaluator::evaluate_invocable_with_mode`.
    CONTRACT §3 lists the warning. Basis: the owner rule that more than nine fraction digits are
    truncated to nanoseconds with a warning naming the literal and the dropped digits, never
    rounded or rejected; CONTRACT §3. 23 registered differences (the warnings of literals and texts
    with more than nine digits, in an expression, a loop and a FEEL-format context), and the 7
    entries of item 74 with more than nine digits now also hold their warning. API breaks
    (rheon-feel): `Diagnostics` has a private `warnings` field with the accessor `warnings()`, so
    its struct literals no longer compile outside the crate; `diagnostics::warn`,
    `values::truncated_fraction_warning` and `values::warn_truncated_fraction` are added;
    (rheon-feel-temporal) `FeelTime::parse_reporting_truncation`,
    `FeelDateTime::parse_reporting_truncation` and `FeelDaysAndTimeDuration::parse_reporting_truncation`
    are added.
78. **`time()` seconds with more than nine fraction digits warn.** `time(hour, minute, second[,
    offset])` takes the nanoseconds of an exact decimal seconds number and truncated the digits
    beyond the ninth silently, while the same truncation of a literal warns (item 77). It now
    warns with the same message, naming the seconds number as `arg` quotes it and the digits it
    lost: `time(10, 0, 0.1234567891)` gives `fractional seconds of '0.1234567891' truncated to
    nanoseconds; dropped digits: '1'`, and a computed number warns too (`time(10, 0, 10/3)`, 24
    dropped digits). The value is unchanged; no warning when every dropped digit is 0 or when the
    call gives no time (an offset out of range). The split of the seconds into nanoseconds and
    dropped digits is one function for both arities. Basis: the owner rule that a truncation to
    nanoseconds warns, for literals and `time(h, m, s)` alike, and only when a digit other than 0
    is dropped; CONTRACT §3. 2 registered differences. No API change.
79. **The elements of a model nest at most 100 levels deep.** The XML parser (roxmltree's tokenizer
    recurses once per element), the schema validation, the model parser and the evaluator builders
    all follow the element tree recursively, and nothing bounded it: a model with 100,000 nested
    `itemComponent`s or boxed contexts overflowed the stack inside the XML parser and aborted the
    process. `rheon_model::parse` now reads the text once, without recursion, before the XML parser
    runs, the way the tokenizer reads it (start, end and empty-element tags change the depth;
    comments, CDATA sections, processing instructions, quoted attribute values and text do not;
    where the tokenizer stops with an error, so does the check), and rejects a document whose
    elements nest deeper than 100 levels, the root element being level 1, with `<ModelParserError>
    elements are nested more than 100 levels deep`. Every later recursion over the elements is
    bounded by it. The bound is the stack proof: a model nested to it is parsed, built and evaluated
    on a 2 MiB thread of the unoptimized test build (nested boxed lists and boxed contexts; tests at
    the bound and one past it). The measured ceiling on that thread is 124 levels (aarch64, rustc
    1.95.0): the XML parser alone overflows it from 125 levels on, and parsing, building and
    evaluating a model all pass at 124. The bound keeps a margin of about a quarter below that
    ceiling, rather than the 256 levels asked for or the measured 124, because CI runs the at-bound
    tests on x86_64 Linux, whose ceiling could not be measured; the deepest model of the repository
    and the TCK nests 12 levels. Old and new (native only; the wasm module parses no models): a
    model nested 101 to 124 levels deep parsed and now gives the error; one nested deeper aborted
    the process and now gives the error; a malformed document nested more than 100 levels deep
    reports the nesting, not the XML error found later in the text. Basis: the owner rule that
    robustness comes first (a model file must not abort the host); the owner decision to keep the
    bound at 100 with that margin; DMN 1.6 defines no nesting limit, so the bound is an
    implementation limit, reported as an error. API break (rheon-model): `parse` rejects a document
    whose elements nest more than 100 levels deep.
80. **Item definitions that form a cycle are a build error naming the cycle.** The type, value
    and context evaluators of item definitions follow their type references at every use,
    recursively, and the only guard was a cycle validator in `rheon_model::parse` with four
    defects: it added a second node for a reference placed before its definition, so a cycle of
    two or more definitions (`tA{x: tB}`, `tB{y: tA}`) was never closed; it had no edges for
    function items (`tF` with `outputTypeRef="tF"`); it checked one file, while the evaluators
    resolve over all models of a build (two models with one namespace); and its search recursed,
    so a long chain of definitions overflowed in the validator itself. Each of these aborted the
    process with a stack overflow. The validator is removed, and one check runs at the start of
    the build, before anything follows a reference, over the item definitions of all models: one
    node per key (the last definition added under it, as in the evaluators), edges from the
    references the evaluators follow (the type reference, the components' type references at any
    depth, a function item's parameter and output types; the rule for function items is now one
    function, `referenced_item_definition`, shared with the type evaluator), and an iterative
    depth-first search that names the first cycle it closes:
    `<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tA`, each name quoted by
    `arg` and the message capped at 1,000 characters, so a ring of 10,000 definitions is named
    in a bounded message. A reference into an imported model is not followed by the evaluators
    and has no edge. Old and new (native only; the wasm module builds no models): the cycles above,
    a 3-cycle through type references, a function taking its own type, a cycle through a nested
    component, two models with one namespace and a 10,000-definition ring aborted the process and
    are now that error; `DMN_N_0088` and `1001.dmn` (a self-referencing `iTreeNode`) failed
    `parse` with `<ModelValidatorError> cyclic dependency between item definitions`, so a
    workspace skipped only that file, and now parse and fail the build with
    `item definitions form a cycle: iTreeNode -> iTreeNode`, so their workspace fails to deploy
    like any other build error. Basis: DMN 1.6 §7.3.3 (a type reference names an item definition
    or a built-in type; no recursive types are defined, so recursive item definitions stay
    rejected); the owner rule that robustness comes first. API breaks: (rheon-model) `parse` no
    longer rejects item definition cycles, the `<ModelValidatorError>` kind is gone and the
    `petgraph` dependency is removed; (rheon-model-evaluator) `ModelEvaluator::new` returns the
    new `<ItemDefinitionError>` for a cycle.
81. **An item definition type beyond the size or depth bound is a build error.** The type
    evaluator builds a definition's type as a tree and expands a referenced definition at each
    use, so 30 definitions that each use the next one twice built a type of 2^31 nodes (the
    process ran out of memory) and a chain of 100,000 definitions, each a context of the next,
    overflowed the stack while it was built. The graph check of item 80 now also computes, in
    the order its search finishes the definitions and without building any type, the number of
    nodes and the depth of each definition's type, by the rules the type evaluator builds by: a
    built-in type is one node; a context is one node and its entries, a list one node and its
    item type, a function one node, its parameter types and its result type; a referenced
    definition counts its whole type at each use; a reference to no definition counts one node.
    Depth: a built-in type is 1, a context, list or function one more than its deepest part. A
    type of more than 100,000 nodes is `<ItemDefinitionError> item definition 't16' expands to more
    than 100000 type nodes`, one nested more than 256 levels deep `item definition 't1' nests
    types more than 256 levels deep`, naming the first definition finished that exceeds a bound.
    Tests check that the counts are those of the built types, and that a type of exactly 100,000
    nodes and one exactly 256 levels deep build, and that an input of each is checked, evaluated
    and tested with `instance of` on the default 2 MiB thread of the unoptimized test build (the
    deep one overflows that thread from about 600 levels on, aarch64, rustc 1.95.0). The largest
    type of the repository and the TCK models has 51 nodes and the deepest 6 levels. Old and new
    (native only): 17 levels of doubling (a type of 131,071 nodes) built and now give the node
    error for `t16`, and 30 levels exhausted memory and now give it at once; a chain of 257
    definitions built and now gives the depth error for `t1`; a chain of 100,000 aborted the
    process and now gives it for `t99744`. What remains: the bounds hold per definition, so a type
    is still expanded at each use and the global type context, cloned for each decision
    evaluation, grows with the models times the node bound; sharing named types is later work.
    Basis: the owner limits for item definition types (100,000 nodes, 256 levels), each with its
    stack proof; DMN 1.6 defines no type size limit, so the bounds are implementation limits,
    reported as build errors. API break (rheon-model-evaluator): `ModelEvaluator::new` returns
    `<ItemDefinitionError>` for a type over 100,000 nodes or 256 levels.
82. **Decision-service function values hold a weak model reference.** Each decision service is
    also a function value, stored in the model evaluator, and its body held an `Arc` of that
    model evaluator, so every model with a decision service referred to itself and was never
    freed: a host that reloads models (the server, a workspace) kept every version. The body now
    holds a `Weak` reference and upgrades it for each call; during an evaluation the caller holds
    the model, so the upgrade succeeds and every answer is unchanged (TCK and the compatibility
    tests of 0085-0088, 0092 and 1130). The invariant, in the module and `FunctionBody` docs: a
    value never holds a strong handle to the state that holds it. Old and new (native only): a
    model built from `3_0085.dmn` stayed alive after its last `Arc` was dropped and is now freed;
    a decision service value called after its model was dropped evaluated the service with the
    leaked model and is now `null(decision service decisionService_001 cannot be called after its
    model was unloaded)`. The remaining step, the invocation taking the model from the call's
    session instead of from the value, is later work (BACKLOG). Basis: DMN 1.6 §10.4 ("A
    decision service is semantically equivalent to a FEEL function"); the owner rule that memory
    never accumulates beyond a request. API break (behaviour, rheon-model-evaluator): a
    decision-service function value called after its model was dropped is null with that reason.
83. **The wasm display measures the width of a line before rendering it.** `display` writes a list
    or context on one line when that line fits 80 columns, else expanded. It built the one-line
    text of every container to compare its length and kept it while it rendered the children, so
    a result nested d levels deep around a string of s bytes held about d × s bytes at once: a
    2 MiB string inside 280 lists filled the 512 MiB memory cap and trapped the module. Now one
    one-line writer writes either into the output or into a measure that counts the bytes and
    stops once they pass the width left on the line (strings and keys are counted as serde_json
    escapes them, without a copy, and a string longer than that width stops the measure at
    once), and each node is written once. The output is byte-identical: `indent + width <= 80`
    holds exactly when `width <= 80 - indent`, and past indent 80 no container fits, as before;
    tests pin the 80/81-byte boundary at indent 0, 2 and 78, a width counted in bytes, the
    escapes of strings and keys at the boundary, empty containers and nesting past indent 80.
    `scripts/smoke.mjs` evaluates the 280-level probe on a fresh instance: the 0.4.0 module
    traps after growing by 498 MiB, the new one answers with the expanded display and grows by
    26.5 MiB (bound 32 MiB). Speed against 0.4.0 (engine benchmarks): geomean 1.00x, larger
    results 1.05-1.10x (`ctx200-*`), every workload at 0.96x or above. 1 registered difference:
    the probe, which trapped and now answers. What remains: a scalar other than a string (a
    range with long string endpoints) is still rendered to text when it is measured, at most
    once per enclosing level within 80 bytes of it. Basis: CONTRACT §3, the `display` rule;
    the owner rule that robustness comes first. No API change.
84. **No combination of edge operands traps the module.**
    `wasm/feel-wasm/tests/corpus/edge-operands.json` is a deterministic product of expression
    templates over 38 operands at the engine's edges: numbers at the limits of the 64-bit and 32-bit
    integers and of decimal128 (`1E+6144`, `-9.999999999999999999999999999999999E+6144`, `1E-6176`),
    both duration kinds at the ends of their range and at one unit, dates and date and times at the
    ends of the supported range and in a daylight saving gap and overlap, times at the ends of the
    day, null, `""`, `[]` and `{}`. The contract test `edge_operand_products_never_trap` applies
    each of the 15 unary templates (the value, `string`, `abs`, negation, the duration and date
    properties, `.value`, `= itself`, `floor`, `decimal`) to every operand and `+`, `-` and `*` to
    every ordered pair, 4,902 requests, with the checks of the no-trap corpus (commit 55) after each
    one; with `RHEON_FULL_TESTS` set (natively only, Wasmtime passes no environment) it adds `/`,
    `<`, `=`, `modulo`, `max` and `**`, 13,566 requests. It is the seed set of combinations for a
    later fuzzer, where the no-trap corpus holds single requests. Against the engine before commit
    59 the test fails natively on its first trapping request, `duration("-PT0.000000001S") + date
    and time("-262143-01-01T00:00:00Z")` (an `internal` error, ``DateTime - TimeDelta`
    overflowed``), and aborts the test binary under Wasmtime with that request as the last
    `no-trap:` line; on this tree it passes in both, and the full product passes under Wasmtime too
    (checked with the switch forced on). Run time: 0.8 s native debug, 1.0 s under Wasmtime; full
    size 0.24 s native release, 2.3 s native debug and 2.9 s under Wasmtime. Answers are not
    asserted, and the product is not a source of the identity check: its full size's 13,528 distinct
    requests through 0.4.0 and this build differ in 568 answers. The 24 that are not message changes
    are registered one by one, with both answers, in the ledger of the change that makes them: 7
    sums, differences and products of exactly -2^63 nanoseconds, which 0.4.0 printed 1 ns off, and 3
    of exactly -2^63 months, which trapped, are null (item 59); 5 date and times at the first
    supported instant minus a duration, which trapped, are null (item 61); 6 dates minus one
    nanosecond are the day before, and 3 at the first supported date are null (item 62, Table 58).
    The other 544 are null reasons that quote `1E+6144`,
    `-9.999999999999999999999999999999999E+6144` or `1E-6176` in scientific notation where 0.4.0
    wrote up to 6,177 plain digits: instances of the registered message rule (item 68, CONTRACT §3),
    not listed one by one. Basis: no input may panic the engine (CONTRACT §1: on wasm a panic aborts
    the instance). No API change.
85. **Automated checks of the identity tool.** `tools/identity/test.mjs` (`npm run test:identity`,
    run in CI after the identity step) checks `equal.mjs` on two real modules, the previous release
    and the build (`IDENTITY_REFERENCE`, `IDENTITY_CANDIDATE`; a missing module fails the tests): a
    module against itself exits 0 and leaves a `today()` request and its parse twin out; a
    request that traps both sides is `{"trap": true}` on both and the next request runs on a fresh
    instance; requests of the committed ledger with a 40-digit integer and `1.5e-6176` added to
    their context differ between the modules and exit 1, `--record` drafts them with those numbers
    as written, and the draft passed as a ledger exits 0; the same draft exits 1 when the candidate
    does not make the change and when the reference does not answer `old`; an entry with a wrong
    `old` or `new` exits 1; a context number beyond a double is sent as written; two requests that
    differ only beyond a double's precision are two requests; thirteen kinds of malformed ledger,
    a request registered twice and six usage errors exit 2 with their message. The checks that need
    a difference between the modules skip with a message when the two modules answer every request
    of the committed ledger alike. The checks found one defect: a JSON number where the ledger
    expects an entry or a response object was taken for an object, so `"old": 2` was accepted as a
    response that never matched (exit 1 instead of 2) and a number entry was reported as an
    unknown field `rawJSON`; both are input errors now, through one predicate for "a JSON object"
    that the canonical form uses too. A planted defect in the tool (the `old` check skipped, lossy
    JSON, the clock exclusion off) fails the checks. No API change.
86. **One module session for the Node tools.** `scripts/feel-abi.mjs` holds the raw ABI once:
    `callText` returns the response text, decoded as strict UTF-8 (bytes that are not UTF-8 throw
    `MalformedResponse`), `call` parses it, and `session(module)` answers request texts one after
    another and, after a trap, resolves to `{ trap }` with the error and continues on a fresh
    instance. The identity check (`tools/identity/equal.mjs`) and the Node host of the stack check
    (`scripts/stack-headroom.mjs`) used their own copies of that loop; both use the session now.
    The smoke test's `call` now rejects a response that is not UTF-8 instead of decoding it with
    replacement characters, and the stack check fails on a response that is not JSON instead of
    reporting it as a trap. Checked: the identity tool's checks (item 85) pass before and after,
    the identity run prints the same lines, `npm run smoke` and `npm run stack` pass, and the
    stack report is the same with the old and the new scripts. No API change.
87. **The identity check answers the reference per source.** A unit is one source file (its
    requests, then the parse twins of its `evaluate` requests, without the clock requests) or one
    ledger file (the `request` of each entry). The reference answers each unit in order on a fresh
    instance, so its answers to a unit are a function of the module and the unit, which a cache of
    them needs; a request that several units list takes the answer of the first. The candidate
    answers the work list on one instance as before, and the work list, the comparison, the
    output and `--record` are unchanged. Responses are held and compared as their canonical text
    (sorted keys, every number as written), parsed again only where a result text or a draft
    needs the object. A request in several units is now evaluated once per unit by the
    reference: on the committed sources 2,627 requests as before, with 50 module panic messages on
    stderr instead of 30 and the same stdout; the time is unchanged (8.1 s and 8.2 s). Checked: a
    new test runs two sources in both orders and gets the same summary and the same failure lines
    (it passes before the change too: it pins the property that the reference keeps no state
    between requests); the identity tool's checks pass; a `--record` draft of 40 registered
    requests is byte-identical to the one the tool wrote before. No API change.
88. **The identity check caches the reference's answers.** `equal.mjs --cache <dir>` keeps the
    answers of the reference to each unit (item 87) in
    `<dir>/<reference sha256>/<16 hex of the unit path's sha256>-<name>.jsonl`, a header, one line
    per request (the sha256 of the request text and the canonical answer, or a trap) and a trailer
    with the sha256 of every byte before it. A file is used only when the digest is right, the
    header equals the run's (format, reference, unit bytes, unit kind, `--parse-too`, the sha256 of
    `equal.mjs` and `scripts/feel-abi.mjs`, Node's version, architecture and `execArgv`) and the
    request hashes are the unit's in order; otherwise the unit is evaluated and the file written
    again through a rename, and an existing file is reported as rejected with the reason. A trap
    depends on the host, so every cached trap is evaluated again on a fresh instance, and one that
    answers rejects the file. A unit with a response that is not JSON is not cached. A new
    reference directory removes the directories of other references (only directories named by a
    sha256), so there is at most one file per unit path; deleting the cache is always safe. The
    candidate is never cached. `--verify-cache` evaluates the reference in full, fails with
    `CACHE MISMATCH` on any cached answer that differs and rewrites the files. The summary line
    ends with `reference: <n> answers from the cache, <m> evaluated`. `run.sh` passes `--cache
    .tools/identity-cache`; CI restores no cache, so its run is uncached. Before the change the
    four new tests fail with exit 2 (`unknown option --cache`). They check that no cache, a cold
    cache and a warm cache give the same exit status, the same output apart from those counts and
    the same `--record` draft, for identical modules, a trapping request, a trap entry, entries
    with a wrong `old` or `new`, unregistered differences and their draft replayed; that a flipped
    byte, a cached trap the module answers, another `--parse-too`, an edited source and a file that
    is not a cache file are rejected and evaluated; that another reference gets its own directory
    and removes the old one; and that a planted wrong answer with a valid digest misleads a cached
    run until `--verify-cache` reports it, exits 1 and rewrites the file. Measured on the
    committed sources (2,627 requests, 30 of them trapping 0.4.0): 8.4 s cold, 7.5 s warm, 8.4 s
    with `--verify-cache`; the warm run answers 2,701 reference answers from the cache and evaluates
    the 50 trapping ones again, which take 5.6 s, two memory exhaustions of 0.4.0 most of it. No
    API change.
89. **Finding the commit behind an identity difference.** `tools/identity/bisect.sh
    <reference.wasm> <entries.json>` is the step of `git bisect run`: it builds `dist/feel.wasm` of
    the checked-out commit and runs `equal.mjs` with the entries (a `--record` draft, or a part of
    it) as the only ledger and no source, with the reference's answers cached. Every entry
    reproduced is bad (exit 1), any other outcome good (exit 0); a failed build or an input error
    of `equal.mjs` skips the commit (125), and a usage error ends the bisect (128). It runs the
    `equal.mjs` next to it and builds the work tree it runs in, so a copy kept outside the tree
    bisects commits older than the script; a third argument checks a given module without a
    build. Before the script exists the new test fails (bash exits 127); it checks that entries
    recorded from the reference against the candidate are bad with the candidate, good with the
    reference, skipped with a module that cannot be loaded, and that a usage error exits 128.
    `tools/identity/README.md` gains "While developing": the committed sources after every build,
    the larger sets once per group of changes, `--record` then bisect on a failure,
    `--verify-cache` before a release, with the measured times (against 0.4.0: committed sources
    8.4 s uncached and 7.5 s cached; with the sets kept outside the repository, 187,677 requests,
    7.0 min uncached, 6.5 min cached and 7.1 min verifying; 0.4.0's trapping requests, evaluated
    again on every run, take most of the reference's time). Checked with a real bisect between
    0.4.0 (good) and the tree before commit 85 (bad), 31 commits, with the entries of
    `registered/sort-own-order.json` and a copy of the script outside the tree: five steps, each a
    wasm build, ended at `c6a7a69` (item 64), the change that registered them. No API change.
90. **A number oracle that judges every number result.** `tools/number-oracle/run.sh [quick|full]
    [module.wasm]` sends requests to a `feel.wasm` and judges each number result against its truth
    under DMN 1.6, computed in Python (`decimal` with a decimal128 context, exact rationals, and
    mpmath 1.3.0 pinned by hash in `requirements.txt`): every operation applied to the exact values
    of its operands and rounded once to 34 digits, half even, with gradual underflow; null for no
    value (§10.3.2.3.1); the scale functions by their rounding modes within -6111..6176, a scale
    without its fractional part (Table 77, footnotes 1 and 2); `modulo` by its formula over exact
    values; `odd` and `even` by the parity of an integer, false for any other number; `**`
    correctly rounded, with exponents within ±999,999,999 (Table 61); `number()` by footnote 1 of
    Table 73; and one canonical text per value (plain digits, no trailing zeros, no `-0`) in
    `display`, `feel`, `typed` and `string()`.
    `exp`, `log` and fractional `**` are decided by an interval rule (mpmath at rising precision
    with guard digits for the condition, both ends of the enclosure rounded once, a rounding
    boundary such as a tie decided exactly), and `exp` and `log` must also agree with Python's
    correctly rounded functions. 188 families, 2,522 fixed edge vectors in literal form, and a
    random part per family sent as JSON numbers in the context, 200 cases each in a quick run and
    20,000 in a full one. `known-wrong.json` lists the classes (family glob and verdict) the engine
    is known to get wrong, each with its reason; any other wrong answer fails the run, and so does
    an entry that no failing edge vector matches, so a change that repairs a class must delete its
    entry. `scripts/feel-answers.mjs` answers a file of requests with a module, one response line
    per request. The oracle replaces the differential tools as the judge of the number core: they
    compared the two backends and, for the scale functions, re-ran the engine's own algorithm, so
    they could not see a defect both backends share. Before the truths, `oracle.py --self-test`
    (Table 77's examples, the conversion examples of Table 73 and §10.3.2.3.1, Table 40's values,
    the domain rules, and second formulations of `modulo`, integer powers and `sqrt`) fails on
    every planted wrong truth but one that agrees on every case (integer powers through Python's
    own `power`). With an empty exemption list the quick run against the build fails: 4,354 of
    35,924 cases in 142 classes, all of them known defects of the number core: a zero printed with
    a minus sign (430 cases), trailing zeros of the scale (592), the rounding functions with a
    scale (null for a zero or a tiny number, 1,922; wrong values for subnormals, near the scale
    limits and in `round half down`, 671), `modulo` beyond 34-digit quotients (248), integer
    powers that are not correctly rounded (408), exponents outside Table 61's range that are
    evaluated (60), and `string()` results that carry those texts (23). The listed classes cover
    them; the full run, 2,896,576 cases in 5.3 min, has no other wrong answer, no trap and no
    undecided case. No API change.
91. **CI runs the number oracle.** Job `wasm` runs the quick size of the oracle (item 90) on the
    build after the identity check: a virtual environment of the runner's `python3`, mpmath
    installed with `pip install --require-hashes` from `tools/number-oracle/requirements.txt`, and
    `run.sh quick` with `PYTHON` pointing into it. A planted exemption for a healthy family
    (`sqrt/perfect`, `value`) makes the same command fail as a stale entry (exit 1); without it
    the command passes, 35,924 cases in 6.4 s. The install from PyPI was not run here (no network
    on the development machine); there the environment held the same mpmath 1.3.0 files, checked
    against the wheel's `RECORD`. No API change.
92. **The number oracle's edge vectors are an identity source.** `cases.py --edges` prints the
    oracle's fixed edge vectors (2,535 requests in literal form, among them the examples of Table
    77 and Table 40 and the edge vectors that back each class of `known-wrong.json`) as a JSON array,
    and `tools/identity/requests/numbers.json` is that output, committed. The oracle's `run.sh`
    fails (exit 1) when the file is not the generator's output, so a number answer that changes on
    an edge is a registered difference, and every registered new answer is one the oracle judges.
    The oracle now sends the edge vectors of every family before the random parts, so a random
    case can never displace an edge vector of a later family and the vectors that keep an
    exemption alive are the same in every size. Against 0.4.0 one edge differs, by a rule
    registered before: the warning on a context number rounded to a subnormal quotes `2E-6176` in
    scientific notation (the 40-character rule of CONTRACT §3). Its entry is in
    `registered/number-edges-message-previews.json`; a check outside the repository confirmed that
    old and new are equal once the message texts are removed. The committed sources grow from
    2,627 to 7,359 requests (with the parse twins); the run takes 8.4 s uncached and 7.5 s cached,
    as before, since the time is the trapping requests of 0.4.0. Checked: one edited request in
    the committed file fails the oracle's run, which passed before the check; the identity tool's
    checks pass. No API change.
93. **A frozen regex request corpus.** `tools/identity/requests/regex.json` holds every distinct
    `matches`, `replace` and `split` expression that was harvested from the feel-evaluator unit
    tests (120) and from the DMN models under `examples/` and `bbt/` (77), one of them with its FEEL
    context, and 27 more requests on what the regex built-ins must settle under XQuery 1.0 and
    XPath 2.0 Functions and Operators: unknown, repeated, padded and non-string flags, `q` beside
    other flags, `x` and `#`, replacement strings with `$0`, `$10`, a lone `$`, `$$`, `${1}`, `\$`,
    `\\` and `$name`, and patterns that match the empty string in `replace` and `split`
    (224 requests). They hold no expected answers; the previous release answers them, and they
    are identical in the build. The harvested corpora they come from are removed with the
    differential tools (item 94), so the expressions are kept here, frozen. The W3C test suite of
    the regular expressions is not imported. No API change.
94. **The Intel differential tools are removed.** `tools/difftest` (the op-by-op fuzzer of the
    number core, item 11) and `tools/evaluator-corpus` (the FEEL corpus through the evaluator)
    built the number crate twice, once with Intel's BID128 library, and judged where the two
    backends disagreed; `tools/fetch-dfp-number-sys.sh` and `tools/dfp-number-sys.patch` fetched
    and patched that library for them (item 13). The number oracle (item 90) judges every number
    result of the module against DMN 1.6 instead, without a second implementation. Every
    operation of the fuzzer that a FEEL expression reaches has a family in the oracle (parsing,
    `+ - * /`, negation, `abs`, `sqrt`, `exp`, `ln` and `log`, the four kinds of `**`, `modulo`,
    `decimal` and the six rounding functions, `odd` and `even`, comparisons, formatting, and chains
    of operations through nested expressions); the rest (`%`, the assign
    operators, `trunc`, `frac`, `square`, the quantum rounding, the integer comparisons and
    conversions, `new`, the constructors from integers and the constants) are internal to the
    number core and keep their unit tests. The four FEEL corpora of the second tool (17,217
    expressions) are kept as data in `tools/feel-corpus/`, because the parser's corpus test reads
    them (`RHEON_PARSER_CORPUS`). The `tools/` ignore list keeps `__pycache__/` only, and the
    root one loses the fetched library's directory. The `intel` feature of the number crate is not
    part of this change. No API change.
95. **One number backend, no backend features.** DMN 1.6 §10.3.2.3.1 bases FEEL numbers on
    IEEE 754-2008 decimal128; the repository builds one implementation of it, the pure-Rust core
    on dashu-float, and the second one, Intel's BID128 library through `dfp-number-sys` behind
    the `intel` feature, is gone with its binding `feel-number/src/dfp_number.rs` (843 lines).
    `rheon-feel-number` has no features: the three dashu crates are plain dependencies and
    `dfp-number-sys` leaves the workspace table and `Cargo.lock`. The `default`, `dashu` and
    `intel` features that forwarded the choice go from `rheon-feel`, `rheon-feel-temporal`,
    `rheon-feel-parser` (which keeps `parsing-tables`), `rheon-feel-evaluator`, `rheon-model`,
    `rheon-model-evaluator` and the unpublished members (`dsntk-evaluator`, `dsntk-workspace`,
    `dsntk-gendoc`, `dsntk-server` and `dsntk`, the last two keep `tck`), and the workspace table
    no longer declares them with `default-features = false`. `NUMBER_BACKEND` is the constant
    `"dashu"`, so `info.numberBackend` is unchanged. The tests that held a second expectation
    for an Intel build (`feel-number/tests/pow.rs`, TCK 0100 `_0278`) keep the correctly rounded
    one. CONTRACT.md §3, the README and the engine comparison no longer promise bit-identity
    with Intel's library: they state the single rounding of §10.3.2.3.1 and point to the number
    oracle and its list of known deviations; the integer `**` caveat stays. Test first: `cargo
    check --workspace --all-features` failed on the `compile_error!` that forbade both backends
    at once; it passes now. Identity: no difference. Rust API break: the Cargo features `intel`
    and `dashu` (and `default`) of `rheon-feel-number`, `rheon-feel`, `rheon-feel-temporal`,
    `rheon-feel-parser`, `rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator` are
    gone, so a dependency written with `features = ["dashu"]` or `features = ["intel"]` no longer
    resolves; `default-features = false` is harmless.
96. **A number has no negative zero.** DMN 1.6 §10.3.2.3.1: "There is no distinction between -0
    and 0." The number core kept the IEEE 754 sign of a zero, so `-0`, `0 * -5`, `0 / -5`,
    `round half up(-0.4)`, `round down(-0.5)`, `floor(-0.0)`, `sqrt(-0.0)` and `string(-0)`
    printed `-0` (or `-0.0`, `-0.00`, …), and `string(-0) = "0"` was false. Now the one
    constructor of `FeelNumber` (`FeelNumber::of`) clears the sign of a zero; the arithmetic on the
    unpacked value keeps the IEEE signs inside one operation, and every result passes through that
    constructor, so no zero with a sign is ever stored. The point fix that cleared the sign in
    `ceiling` without a scale is gone. Test first: `feel-number/tests/minus_zero.rs` (a parsed,
    negated, multiplied, divided, rounded, square-rooted and raised zero prints `0` and debugs as
    `+0E…`) and the contract test `a_zero_has_no_sign` (`display`, `feel`, `typed`, `string()`)
    failed on the parent with `-0`. Identity: 101 registered differences in
    `registered/no-negative-zero.json`, all on the number oracle's edge vectors: 99 texts of a zero
    that lose the minus sign, `string(-0) = "0"` (false to true) and `string length(string(-0))`
    (2 to 1). The oracle's exemptions for a negative zero and for `string(-0)` are deleted. Rust
    behaviour break: a `FeelNumber` zero has no sign: `"-0".parse()`, `-FeelNumber::zero()` and
    `0 * -5` give `+0` (Debug `+0E+0`), so `1 / -0` is `+Inf` with the operators of the crate.
97. **One canonical text for every number.** DMN 1.6 §10.3.2.3.1 represents a number as a pair
    `(p,s)` and notes that "many different pairs have the same value. For example, value(1,0) =
    value(10,1) = value(100,2)"; Table 73 has `string(1.1) = "1.1"`. The owner's decision is
    canonical values in every text form. It departs from the example texts of Table 40, which
    writes some results with their scale (`decimal(1, 2)` as `1.00`, `.10 * 30.00` as `3.0000`,
    `1.0*10**3` as `1000.0`), and keeps their values. A `FeelNumber` carried a flag beside its value: a
    literal, a JSON number, `decimal`, `abs` and the scale functions printed the trailing zeros of
    their cohort (`1.50`, `decimal(1, 2)` as `1.00`, `abs(exp(1e-34))` as 34 digits), the
    arithmetic trimmed them except in a zero (`1.10 - 1.10` printed `0.0`, `0 * 1e-10`
    `0.000000000`). The flag is gone (`FeelNumber` is the value alone, 32 bytes instead of 48),
    and `Display` writes one text per value: plain digits without an exponent, no trailing zeros
    after the decimal point, `0` for every zero. The cohort stays in the value, where only `Debug`
    shows it, and no rule depends on it. `Display` honours width, fill, alignment and the sign
    flag and ignores a precision (`{:.2}` truncated the digits instead of rounding them, and no
    caller used it). `display`, `feel`, `typed`, `string()`, JSON output and the numbers quoted in
    messages all go through it; CONTRACT §3 states the rule. Test first: the new cases of
    `feel-number/tests/to_string.rs` and the contract test `a_number_has_one_text` (with the
    zeros of `zero_with_huge_exponent_is_displayed_as_0`) failed on the parent with the padded
    texts. Pinned texts updated: `feel-number` (`to_string`, `exp`, `pow`, `round` and the unit
    tests), feel-evaluator (`string({a: 1.0, b: 2.0})`, a reason quoting `0.0`, the text of an
    interval `(1.0..5.0)`), model-evaluator (TCK 0014 `rate`, 0058 `1000000.00`, 1100 `0.0`, and
    three decision tables with `Discount: 0.10`), and the bench copies of the three TCK tests,
    checked by grep since no gate compiles the benches. The TCK runner compares decimals by value,
    so no TCK result changes. Identity: 78 registered differences in
    `registered/canonical-number-text.json`, all on the number oracle's edge vectors: 75 texts
    that lose their trailing zeros, `string(1.50) = "1.5"` (false to true) and two string lengths;
    27 entries of `registered/no-negative-zero.json` (item 96), whose new text was a zero with
    trailing zeros, are amended in place to `0`. The oracle's exemptions for texts with trailing
    zeros and for the `string()` results that carried them are deleted. Rust API break:
    `FeelNumber`'s `Display` (and so `to_string()` and `Jsonify`) prints the canonical text and
    ignores a precision; `FeelNumber` is 32 bytes.
98. **NaN carries no sign.** FEEL has no NaN (DMN 1.6 §10.3.2.3.1: "There is no value for
    notANumber, positiveInfinity, or negativeInfinity. Use null instead."); the number core's NaN
    only marks an invalid operation until the evaluators turn it into null. It carried a sign
    that followed Intel's library (flipped by negation, cleared by `abs`, kept when propagated,
    and the subtraction did not invert it), visible only in `Debug` (`+NaN`, `-NaN`). The sign is
    gone with its special cases in negation, subtraction, `abs` and the power function; `Debug`
    prints `NaN`. Test first: the unit test `nan_has_no_sign` failed on the parent with `+NaN`;
    the test that pinned the Intel signs is deleted. Identity: no difference (a NaN never reaches
    a response). Rust behaviour break: `Debug` of a NaN `FeelNumber` is `NaN`.
99. **`exp` and `ln` results need no cohort rule.** The cohort (exponent) of a correctly rounded
    `exp` or `ln` result followed an emulation of Intel's binary128 to decimal128 conversion
    (`exp(1E-40)` as `1E+0`, `exp(1E-34)` with 34 digits): the result was evaluated a second time
    at 45 digits to decide it, and a fractional power was padded to 34 digits. Since item 97 no
    text shows a cohort, and no rule of the number core depends on it, so the emulation is gone:
    `binary128_cohort`, `wide_digits`, `full_precision` and the 45-digit precision, the second
    evaluation of every `exp` and `ln`, the precision argument of the backend's `exp` and `ln`
    (always 34 digits now), and the three `U256` helpers only the emulation used. A result is
    the value the backend rounded once. Test first: none can fail, since no response shows a
    cohort; the tests that pinned the cohorts (`exp_ln_cohorts_follow_intel`,
    `tests/exp.rs`, `tests/ln.rs`) are value tests now (value and canonical text, at the former
    cohort cases) and passed on the parent and after the change. Identity: no difference; the
    number oracle's `exp`, `log` and fractional power families have no new wrong answer. No API
    change; `Debug` shows other cohorts for these results.
100. **Rounding to a scale is one step.** DMN 1.6 Table 77: `floor`, `ceiling`, `round up`,
    `round down`, `round half up` and `round half down` "Return n with given scale and rounding
    mode", and footnote 1: "Scale is in the range [−6111..6176]". The number core scaled the
    number to an integer first (`n * 10^scale`, with the scale clamped to -6176..6144 like Intel's
    `bid128_scalbn`), rounded that intermediate and scaled it back; `round half down` subtracted
    one tenth of the last place kept and then rounded half up. So `round half down(2.56)` was 2,
    `round half down(1.1251, 2)` 1.12 and `round half down(-2.51)` -2; `ceiling(0.25, 6145)` was
    0.025 and `floor(1e-10, 6150)` 0.0000000000000001 (the clamp); and every number whose
    intermediate left the exponent range was null with "invalid scale": a zero at any non-zero
    scale (`round half up(0, 2)`, `floor(0, 1)`, `ceiling(0, -1)`), a tiny number at a coarse
    scale (`floor(1e-200, -6000)`, `round up(1e-6000, -6000)`). Now one private step,
    `round_at`, rounds a value to a multiple of `10^exponent` on its coefficient: a value with no
    digit below that position is itself (never padded), otherwise the dropped digits are compared
    with half a unit once and the kept coefficient is incremented by the mode; the result always
    fits. `FeelNumber::round_to(scale, mode)` checks the scale range (the only error) and calls
    it; the six methods and `round` (`decimal`) delegate to it, and rounding to an integral value
    is the same step at exponent 0. The modes are one enum, `RoundingMode` (`HalfEven`,
    `HalfUp`, `HalfDown`, `Up`, `Down`, `Ceiling`, `Floor`), which replaces the private `Rnd`;
    `check_scale`, `settled_at`, `unscale`, `quantize_v`, `ilogb_v` and the scaled intermediate go.
    Test first: `rounding_to_a_scale_is_one_step` (`feel-number/tests/round.rs`), a table of 341
    numbers and scales with the result of every mode computed by Python's `decimal`, failed on
    the parent with 392 wrong results (the half-down values, the clamped scale and the spurious
    errors); the unit tests that pinned the old values are updated (`round half down` of 2.51,
    -2.51, 1.151, a number at scale 6145, a NaN at a scale, the clamped scale of `round`).
    Identity: 643 registered differences in `registered/rounding-to-a-scale.json`, all on the
    number oracle's edge vectors of the six functions: 558 nulls that are now numbers, 85 wrong
    values corrected; `decimal` is unchanged. The oracle's twelve exemptions of the six
    functions are deleted, and its quick run judges every rounding result right. Rust
    behaviour break: the six rounding methods of `FeelNumber` give these values, accept a zero
    and a tiny number at every scale of the range, and `round` with a scale outside it is NaN
    (was a value at the clamped scale).

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

History: the Intel backend was removed in item 95; these were the differences while both
existed.

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

Commits 55-84 (2026-11-14), on the tree of commit 84: workspace 8,639 passed, 0 failed, 24
ignored (CI's skips); clippy without warnings; feel.wasm tests 69 native (8 unit, 61 contract) and
70 under Wasmtime (8, 62); runtime 33; smoke passes, with its two memory checks growing by 22.1 MiB
and 26.5 MiB (bound 32 MiB); the stack check passes, and its report reaches at least 0.4.0's
depth on every host (item 67); the playground builds; the randomized and exhaustive tests at full
size (`RHEON_FULL_TESTS=1`, release, with the parser corpus, now with `rheon-feel-wasm`) 971 passed.
DMN TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test
result identical to 0.4.0's. Full responses against the 0.4.0 release module, each request sent as
written: 187,677 requests (the committed sources, each evaluate request also as parse, the ledger,
and request sets kept outside the repository: the fraction sweep, the duration, message and regex
probes and the performance workloads), 1,645 registered differences reproduced (counted per change
in items 59-83), none unregistered; 52 of the requests trap 0.4.0, none traps the build. The
full-size edge-operand product of item 84 (28,110 requests with its parse twins), which is not an
identity source, differs in 568 answers: its 24 answer changes that are not message changes are
among the registered ones, and its other 544 answers differ only in null reasons that quote a number
in scientific notation, instances of the registered message rule (item 68) that are not listed one
by one. Speed against 0.4.0 (engine benchmarks, 24 workloads): geomean 1.01x, every result equal,
from 0.94x to 1.11x.

Commits 39-53 (2026-11-13): workspace 8,488 passed, 0 failed, 24 ignored (CI's skips); clippy
without warnings; feel.wasm tests 56 native and 57 under Wasmtime; runtime 33; the randomized
and exhaustive tests at full size (`RHEON_FULL_TESTS=1`, release, with the parser corpus) 867
passed, with no unregistered difference against the frozen reference front end; DMN TCK via
`scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, no result changed against
`docs/tck-baseline.txt`.

Commit 54 (2026-11-13): workspace 8,500 passed, 0 failed (CI's skips); feel.wasm tests native
and under Wasmtime, runtime 33 and smoke pass; full responses of 1,382 requests identical to
the build before it; DMN TCK per-test results unchanged; the stack check passes, every depth at
least as deep as before (Wasmtime with 512 KiB, 1 MiB, 2 MiB: 9, 23, 52 nested levels, was 8,
21, 48).

DMN 1.6 work (commits 23-34, 2026-11-10): workspace 8,383 passed, 0 failed (CI skips);
feel.wasm contract tests 53 under wasmtime; runtime 33; DMN TCK via `scripts/tck.sh`
(patched, DecisionToolkit runner): official 3,392 of 3,407 (0076 Java 14, 0085 runner 1), was
3,377; tck-addendum 312 of 312 runnable cases (error-result cases are skipped by the runner and
pinned by Rust tests). Failing tests: `docs/tck-baseline.txt`.

Earlier numbers (the `intel` and `dashu` features below were removed in item 95):

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

The two tools were removed in item 94; the number oracle (item 90) judges the number core since.

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
and `rheon-feel-evaluator`, `0.5.0` that of `rheon-feel`, `rheon-feel-temporal`, `rheon-feel-regex`,
`rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator`, see below) and pin each other with `=<version>` in
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

The API changes of 0.4.0 (commits 39-54):

- Changed:
  - `rheon-feel`: `Value::FunctionDefinition` holds an `Arc<FunctionDefinition>` instead of six
    fields. `Name` implements `Borrow<str>`, so `map.contains_key(&"d".into())` on a map keyed
    by `Name` no longer infers its type (write `&Name::from("d")`). The `Debug` text of a
    function value is the derived `Debug` of `FunctionDefinition`. `Value::String` holds a
    `FeelString` (a shared `Arc<String>`) instead of a `String`: a pattern that binds the
    `String` by value no longer compiles (`into_string()` returns it, without a copy when the
    text is not shared); `value_string!` accepts anything that converts into `FeelString`.
  - `rheon-feel-parser`: `ParsingScope<'a>` has a lifetime parameter (invariant) and borrows
    the input contexts; `From<&FeelScope>` is removed, so outside the crate a scope is built
    with `Default`; `pop()` returns `()`; `flattened()` is removed. `ParsingScope` is no longer
    `UnwindSafe`: it holds references to the input contexts, and `FeelContext` is not
    `RefUnwindSafe`, so a closure that uses a scope needs `AssertUnwindSafe` for
    `catch_unwind`. It is `Send` and not `Sync`, as at 0.3.0.
  - `rheon-feel-evaluator`: `ForExpressionEvaluator::evaluate` returns `Value` (was `Values`).
    `FilterExpressionEvaluator` has a private field, so it is built with `new()`, `default()`
    or `for_feel()`, not as `FilterExpressionEvaluator {}`.
- Added: `FunctionDefinition` (with `new` and `From<FunctionDefinition> for Value`); `FeelString`
  (with `From<FeelString> for Value`);
  `FeelScope::{frames, depth, truncate, pop_frame}`; `FeelContext::{insert, assign_entry,
  get_entry_mut}`; `Value::{try_into_coerced, into_coerced}`; `Name::from_parts`;
  `bip::BUILT_IN_PROPERTY_NAMES`; `for_feel()` on `ForExpressionEvaluator`,
  `SomeExpressionEvaluator`, `EveryExpressionEvaluator` and `FilterExpressionEvaluator`.
- Kept: `FeelScope::contexts()`. The value types of `rheon-feel`, `AstNode`, `ClosureBuilder`,
  `FilterExpressionEvaluator`, `Dialect` and `ModelEvaluator` are `Send` and `Sync`, and
  `FeelScope` and `ParsingScope` are `Send`, as at 0.3.0; each crate's `send_sync` test checks
  this at compile time.

The API changes of 0.5.0 (commits 59-83; the item of each change is in parentheses):

- Changed:
  - `rheon-feel-temporal`: `FeelYearsAndMonthsDuration::from_ym` and `from_m` and
    `FeelDaysAndTimeDuration::from_n` and `from_sn` return `Option<Self>`, and
    `FeelDaysAndTimeDuration::from_s` takes an `i32` (59). `ops::Neg` of both durations is total,
    and `&FeelDate - &FeelDate` returns `Option<FeelYearsAndMonthsDuration>` (59).
    `FeelDaysAndTimeDuration::as_seconds` returns `i64` (was `isize`) (63).
    `FeelTime::{utc_opt, local_opt, offset_opt, zone_opt}` and `FeelDateTime::{utc, local, offset}`
    return `None` for nanoseconds of 10^9 or more and for 24:00:00 with a fraction, `utc_opt` turns
    24:00:00 into midnight, and the time and date and time parsers reject 24:00:00 with a
    fraction (75).
  - `rheon-feel`: `value_null!` caps its message at 1,000 characters and builds it in a closure
    that it passes to `values::null_with_reason`, so its arguments are evaluated inside that
    closure and can no longer use `?`, `return` or `break` (67). `Diagnostics` has a
    private `warnings` field, read with `warnings()`, so its struct literals no longer compile
    outside the crate (77).
  - `rheon-feel-evaluator`: `evaluate` returns `Outcome<Value>`; `evaluate_context`,
    `evaluate_context_node`, `evaluate_range_literal` and `evaluate_range_literal_node` return
    `Err` on an abort; the evaluators of `prepare` and `prepare_with_dialect` turn an abort into a
    null with its text; `evaluate_invocation` joins a running session, and outside one a stopped
    invocation is null with the abort's text (was `null(function call depth limit (200)
    exceeded)`, with the flag left set for the next caller on the thread) (65). The crate no
    longer depends on `regex` (72).
  - `rheon-feel-regex`: `is_match` and `is_match_with_flags` return `Result<bool, RegexError>`
    (73); `regex` and `regex-syntax` become dependencies (72).
  - `rheon-model`: `parse` rejects a document whose elements nest more than 100 levels deep (79);
    it no longer rejects item definition cycles, the `<ModelValidatorError>` kind is gone, and
    the `petgraph` dependency is removed (80).
  - `rheon-model-evaluator`: `ModelEvaluator::evaluate_invocable` returns `Outcome<Value>` and
    `evaluate_invocable_with_mode` returns `Outcome<(Value, Diagnostics)>`, and the evaluator of
    `build_decision_table_evaluator` runs as one evaluation, turning an abort into a null with its
    text (65). A decision-service function value called after its model was dropped is null with
    that reason (82). `ModelEvaluator::new` returns `<ItemDefinitionError>` for item definitions
    that form a cycle (80) and for a type over 100,000 nodes or 256 levels (81).
- Removed:
  - `rheon-feel-temporal`: the `ops::Add` and `ops::Sub` impls of both durations (use
    `checked_add` and `checked_sub`) and their `checked_neg` (59); `FeelDate::add_months(u32)` and
    `FeelDate::sub_months(u32)` (61); `FeelTime::{utc, local, offset}` (75).
  - `rheon-feel-evaluator`: `reset_call_depth` and `call_depth_exceeded` (65).
- Added:
  - `rheon-feel-temporal`: `Default` (zero) for both durations and `FeelDateTime::nanos_since`
    (59); `FeelTime::midnight(FeelZone)` (75); `FeelZone::from_designator` (76);
    `parse_reporting_truncation` on `FeelTime`, `FeelDateTime` and `FeelDaysAndTimeDuration` (77).
  - `rheon-feel`: `diagnostics::{Abort, evaluation, one_evaluation, abort, enter_call, Call}` and
    `From<Abort> for DsntkError`, with `Abort` and its variants `#[non_exhaustive]` and
    `Abort::limit` building the engine-limit variant (65); `message::{arg, Arg, compose,
    MessageArg, Size}`, `values::TypeOf` with its `MessageArg` impl, `values::null_with_reason`
    (the cold function behind `value_null!`), and `MessageArg` for
    `Value`, `FeelType`, `FeelNumber`, `Name`, `FeelContext`, `Bip`, `str`, `String` and
    `serde_json::Value` (67); `MessageArg::is_short` (with a default) and `MessageArg` for
    references and `FeelString` (68); `diagnostics::warn`,
    `values::truncated_fraction_warning` and `values::warn_truncated_fraction` (77).
  - `rheon-feel-evaluator`: `Session` and `Outcome` (65).
  - `rheon-feel-regex`: `RegexError`, `Limit`, `replace` and `split` (72).
- Unpublished members: `dsntk-evaluator` re-exports `Session` and `Outcome`, and its `evaluate`
  returns `Outcome`; `dsntk-workspace`'s `Workspaces::evaluate` returns `Err` on an abort (65).
- Kept: `rheon-feel-number`, `rheon-feel-grammar`, `rheon-feel-parser` and `rheon-feel-wasm` have
  no API change. The `Send` and `Sync` properties that each crate's `send_sync` test checks are
  those of 0.4.0. The wasm module's ABI and its request and response JSON are unchanged; what
  changes there are answers, registered with both answers as "Verification" states.

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

1. Rerun the verification above (the suites, the identity check
   `tools/identity/run.sh` and the number oracle `tools/number-oracle/run.sh full`).
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
