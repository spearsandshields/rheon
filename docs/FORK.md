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
| 85 | `c4efc4f` | tools: automated checks of the identity tool | fork |
| 86 | `5247f8b` | scripts: one module session for the Node tools | fork |
| 87 | `4c63a17` | tools: the identity check answers the reference per source | fork |
| 88 | `5aeaa6c` | tools: cache the reference answers of the identity check | fork |
| 89 | `52f3aae` | tools: find the commit behind an identity difference | fork |
| 90 | `75f0541` | tools: a number oracle that judges every number result | fork |
| 91 | `0c50018` | ci: run the number oracle | fork |
| 92 | `063a27d` | tools: the number oracle's edge cases are an identity source | fork |
| 93 | `3ba44fd` | tools: frozen regex request corpus | fork |
| 94 | `397762a` | tools: remove the Intel differential tools | fork |
| 95 | `1d2087d` | feel-number: one number backend, no backend features | fork |
| 96 | `1526888` | feel-number: a number has no negative zero | upstream bug |
| 97 | `8fbf3e6` | feel-number: one canonical text for every number | fork |
| 98 | `a0a3bee` | feel-number: NaN carries no sign | fork |
| 99 | `c5bcbba` | feel-number: exp and ln results need no cohort rule | fork |
| 100 | `63e021c` | feel-number: round_to rounds to a scale in one step | upstream bug |
| 101 | `cb193b6` | feel-evaluator: the rounding built-ins share round_to | fork |
| 102 | `01bde71` | feel-number: new() scales without a clamp | upstream bug |
| 103 | `913b90a` | feel-number: exact modulo | upstream bug |
| 104 | `321f364` | feel-number: integer powers are correctly rounded | upstream bug |
| 105 | `c118fec` | feel-evaluator: an exponent outside Table 61's range gives null | upstream bug |
| 106 | `5083e22` | feel-temporal: a time offset prints its sign from its value | upstream bug |
| 107 | `55541f6` | feel-evaluator: time() takes every offset within ±14:00 and none with a fraction of a second | upstream bug |
| 108 | `e2ed204` | feel-temporal: the minutes of an offset literal are below 60 | upstream bug |
| 109 | `1ad19fc` | feel-temporal: an offset literal has no seconds | DMN 1.6 |
| 110 | `59fbd66` | feel-wasm: the offset of a zoned date and time is displayed with its seconds | fork |
| 111 | `43648ef` | feel-evaluator: second and seconds are exact decimals | upstream bug |
| 112 | `37c8fc2` | feel-temporal: the years and months of a duration are magnitudes | upstream bug |
| 113 | `4efa7f0` | feel-temporal: years and months duration(from, to) has one rule for both signs | upstream bug |
| 114 | `20efaa2` | feel-evaluator: the value of a time in a zone without a known offset is null | upstream bug |
| 115 | `4cda64c` | feel-evaluator: a years and months duration that drops a fraction of a month warns | fork |
| 116 | `1a77981` | feel-evaluator: pin the nulls that are no errors | DMN 1.6 |
| 117 | `384fc80` | feel: a missing context entry is a defined null with a note | DMN 1.6 |
| 118 | `26f2cdb` | feel-evaluator: stddev of fewer than two numbers is a defined null | DMN 1.6 |
| 119 | `1279ceb` | model-evaluator: a decision table without a matching rule is a defined null | DMN 1.6 |
| 120 | `70f3936` | model-evaluator: a missing input variable is a defined null whatever its type | DMN 1.6 |
| 121 | `5ed694d` | feel-evaluator: if takes the else branch unless the condition is true | upstream bug |
| 122 | `0cae1b8` | feel-evaluator: != reports like = | upstream bug |
| 123 | `0a1d4c5` | feel-evaluator: a null operand of an arithmetic operator is an error | upstream bug |
| 124 | `3adac0a` | feel-evaluator: a unary comparison compares with the value of its path endpoint | upstream bug |
| 125 | `4da3dcf` | model-evaluator: one collection evaluator for every simple type | fork |
| 126 | `c9202e8` | model-evaluator: an item definition that references another keeps its allowed values | upstream bug |
| 127 | `3069fdb` | model-evaluator: allowed values are tested against each element of a collection | upstream bug |
| 128 | `08f630e` | feel-evaluator: an iteration context sees the variables bound before it | upstream bug |
| 129 | `f846d99` | feel-evaluator: a repeated iteration variable gives null | DMN 1.6 |
| 130 | `7377b32` | regex: matches and replace read their flags by one rule | upstream bug |
| 131 | `38c6bf4` | regex: a replacement string follows the F&O rules | upstream bug |
| 132 | `a4d650d` | regex: replace and split reject a pattern that matches the zero-length string | upstream bug |
| 133 | `0590e61` | ci: document the published crates with warnings denied | fork |
| 134 | `f5c2713` | ci: run on Node 24 | fork |

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

Commits 55-84 (0.5.0) make the engine robust at its limits: the inputs that trapped the module
through a panic no longer do (an input that exhausts the module's memory still traps), every
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

Commits 85-133 make answers follow DMN 1.6 where 0.4.0 gave a wrong one without a report: numbers
(one backend, one canonical text, exact rounding, modulo and integer powers), time offsets and the
temporal properties, the nulls that the specification defines, `if`, `!=`, the null operands of
arithmetic, unary comparisons with a path endpoint, allowed values of item definitions, iteration
contexts, and the flags, replacement strings and zero-length patterns of the regex built-ins. Tools
come first: the identity check answers and caches the reference per source, a number oracle judges
every number result against exact arithmetic, and frozen request corpora pin the number edges and
the regex built-ins; the last commit makes CI document the published crates with rustdoc's warnings
denied. Each change of a wasm answer against 0.4.0 that the identity check finds is registered with
both answers and counted in its item; changes that only DMN or the Rust API show are stated in
their items with both answers. The totals are under "Verification" below.

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
101. **The rounding built-ins share `round_to`.** `floor`, `ceiling`, `round up`, `round down`,
    `round half up` and `round half down` of feel-evaluator went through six copies of the same
    argument checks, each calling its own method of `FeelNumber`; now one private helper takes
    the number, the scale, the `RoundingMode` and the built-in's two names and calls
    `FeelNumber::round_to` (item 100), keeping every reason text byte for byte (`[core::round
    down] invalid argument type, …`, `[core::round_up] invalid scale: …`, `[core::floor]
    <FeelNumberError> invalid scale, …`). `decimal` calls `round_to(scale, HalfEven)` after
    truncating the scale and keeps its reason `scale is out of range`. `round_to` and
    `RoundingMode` become public with these callers (feel-evaluator now depends on
    `rheon-feel-number` directly); the seven methods they replace are removed. The helper builds
    its type reasons with `invalid_argument_type!`, which, like the module form of `value_null!`
    behind it, takes the function name as any expression, not only as a literal. The tests of
    `feel-number` and the two rounding benches call `round_to`. Test first: none new; the
    built-in tests, the rounding table of item 100 and the identity run are the proof.
    Identity: no difference. Rust API break: `FeelNumber::ceiling`, `floor`, `round_up`,
    `round_down`, `round_half_up`, `round_half_down` and `round` are removed; use
    `round_to(scale, RoundingMode::…)`, which returns `Err` only for a scale outside
    `-6111..=6176` (`round(&scale)` took the scale as a number and gave NaN there).
102. **`FeelNumber::new` scales without a clamp.** `new(n, s)` multiplied `n` by `10^-s` through
    a scaling step that clamped the power to -6176..6144 like Intel's `bid128_scalbn`, so
    `new(890, 6177)` was `890E-6176` (ten times too large) and `new(1, -6177)` was `1E+6144`
    instead of an overflow. Since item 100 nothing else used that step; it goes, and `new` packs
    `n · 10^-s` with the one rounding of the core (half-even, gradual underflow, infinity beyond
    the range). Test first: `test_debug_004` and the unit test of the padding near the overflow
    edge failed on the parent with the clamped values (`+890E-6176`, `1E+6144` for `new(1,
    -6145)`). Identity: no difference (no FEEL path passes such a scale). Rust behaviour break:
    `new(n, s)` with `s` beyond the range of the exponent is the rounded value or infinite
    (`new(890, 6177)` is `89E-6176`, `new(1, i32::MAX)` is 0, `new(1, i32::MIN)` is infinite).
103. **`modulo` is exact.** DMN 1.6 Table 77: "modulo (dividend, divisor) = dividend -
    divisor*floor (dividend/divisor)", and "In case either dividend or divisor is negative, the
    result has the same sign of the divisor"; the owner's decision is the formula over the exact
    values, rounded once. The built-in computed it in 34-digit steps (a division, a floor, a
    product and a difference, each rounded), so a quotient beyond 34 digits or near an integer
    lost the remainder: `modulo(1e40, 3)` was 1000000, `modulo(1e40, 7)` 0, `modulo(5e33, 0.7)` 0,
    `modulo(1 - 2e-34, 1/3)` was -1E-34 (the wrong sign), `modulo(-1E-6176, 1E+6000)` a negative
    number, and `modulo(1e6000, 1e-6000)` and `modulo(9e6144, 1e-6176)` were null (the quotient
    overflowed). Now `FeelNumber::modulo` takes the truncated remainder `|a| mod |b|` exactly on
    the coefficients (with `ea >= eb` the power `10^(ea-eb)` is taken modulo the divisor's
    coefficient by binary exponentiation with exact 256-bit products, new `U256::rem_u128`; with
    `ea < eb` it is a `u128` remainder, or `|a|` itself when `|b|` is the larger), keeps it with
    the divisor's sign when the signs agree, and otherwise subtracts it from `|b|`, the only
    rounding. `None` means a zero divisor; the built-in keeps its reason `[core::modulo] division
    by zero`. Test first: `the_remainder_is_exact` of the built-in's tests (with `modulo(1e40, 3)`
    1000000 on the parent) failed; the Table 77 examples are tests of the built-in and of
    `FeelNumber::modulo` (`feel-number/tests/modulo.rs`, which replaces the tests of `%`); the
    contract test that listed `modulo(9e6144, 1e-6176)` and `modulo(5, 1e-6176)` among the nulls
    of an overflow now asserts 0 for both. Identity: 13 registered differences in
    `registered/exact-modulo.json`, all on the number oracle's edge vectors; the two requests of
    the contract test are new edge vectors of its `modulo/huge-quotient` family, so they are in
    the committed source. The oracle's two `modulo` exemptions are deleted, and its quick run
    judges every `modulo` result right. Rust API break: `impl Rem` and `impl RemAssign` for
    `FeelNumber` (`%` was the same 34-digit-step floor modulo) are removed; use
    `FeelNumber::modulo`, which returns `None` for a zero divisor.
104. **Integer powers are correctly rounded.** DMN 1.6 Table 61: `e1 ** e2` is "(p,s) such that
    value(p,s) = value(e1)^value(e2) + ε", "p is limited to 34 digits", "ε is rounding error",
    with the one rounding of §10.3.2.3.1; the owner's decision is correctly rounded. An integer
    exponent within `i32` was a binary exponentiation that rounded every product to 34 digits,
    so the error grew with the exponent: `1.05 ** 360` ended in …541 instead of …542, `(1 +
    0.05/12) ** 360` in …413408 instead of …41355, `1.1 ** 100000` was 294 ulp off,
    `1.0001 ** 1000000` about 89,000 ulp, `2 ** -20000 = 0.5 ** 20000` was false and `3 ** (4 **
    5)` 1 ulp off. Now `pow_integer` takes the base `c0 · 10^e0` without trailing zeros and `n =
    |y|`: (1) `y > 0` and `c0^n` within 256 bits (`U256::checked_pow`, square and multiply with
    checked products, no allocation): that exact power, rounded once; (2) `y < 0` and `c0` a
    power of two or of five: `1/c0` is the exact `5^k · 10^-k` or `2^k · 10^-k`, raised the same
    way, so ties such as `2 ** -50` are exact; (3) otherwise while `c0^n` has at most 16,384
    bits (about 4,900 digits): the exact power from the backend (`Backend::int_pow`,
    `UBig::pow`), or for `y < 0` one correctly rounded division `1 / c0^n`, moved to the
    exponent `±e0 · n` and rounded once into the range; (4) beyond that the certified
    `exp(y ln x)` loop with the integer as exponent. An inexact integer power is never a tie, so
    that loop no longer falls back to the tie assumption for it: it goes on from 140 to 280 and
    560 working digits, and a result still undecided there (within about 10^-520 of a half-way
    point, relatively; no such operand is known) is rounded as its computed digits say;
    CONTRACT §3 states this bound. The `(-1)^y` emulation of Intel's cohort for an exponent
    outside `i32` goes with the binary exponentiation; `|x| = 1` gives ±1 by the parity of `y`
    on the exact path. Test first: `integer_powers_are_correctly_rounded`
    (`feel-number/tests/pow.rs`, the exact powers rounded once, ties included) failed on the
    parent with 12 wrong powers; the cohort test of `(-1)^y` is a value test now. Pinned values
    updated: `3 ** (4 ** 5)` in feel-evaluator, and the payment amounts of TCK 0008, 0009, 0003,
    0005, 0014, 0040 and 0041 in model-evaluator with their bench copies (checked by grep; each
    new value is the one the number oracle computes for the same formula); the TCK runner
    compares decimals with an epsilon, and those folders, 0075 and 0100 pass as before.
    Identity: 16 registered differences in `registered/integer-powers.json`, all on the number
    oracle's edge vectors (`(1 + 0.05/12) ** 360` is …41355, not …413550: the value ends in a
    zero, which the canonical text drops). The oracle's exemption for wrong integer-power values
    is deleted; its quick run judges every power right but the exponents outside Table 61's
    range. Speed, per evaluation against 0.4.0 (`feel.wasm`, a random 34-digit `x`): `x ** 2`
    1.0×, `x ** -3` 0.9×, `x ** 12` 1.35×, `x ** 100` 7.8× (0.14 ms, an exact power of 3,400
    digits), `x ** 1000000` and `1.0001 ** 1000000` 8.4× (0.43 ms, the certified loop; it was a
    binary exponentiation of 20 rounded products). Rust behaviour break: `FeelNumber::pow` with
    an integer exponent gives the correctly rounded value; `Debug` of an integer power shows the
    cohort of the exact power of the base without its trailing zeros (`10 ** 2` is `+1E+2`, was
    `+100E+0`; `1.50 ** 2` is `+225E-2`, was `+22500E-4`), and `(-1).pow(y)` for a huge `y` has
    the cohort of 1.
105. **An exponent outside Table 61's range gives null.** DMN 1.6 Table 61: in `e1 ** e2`,
    "value(e2) is a number in the range [-999,999,999..999,999,999]"; the owner's decision is
    null outside it. The evaluator computed every power: `1 ** 1e10`, `(-1) ** 1e10`,
    `1 ** 1000000000` and `1 ** 1000000000.5` were 1, `0.5 ** 1e10` and `0 ** 1000000000` 0, and
    `2 ** 1e10` null as an overflow. Now `**` checks the exponent first, for every base and every
    exponent value, integer or not, before the shortcuts `1 ** y` and `x ** 0` of the number
    core: outside the range it is null with the reason `exponentiation exponent is out of range
    -999999999..999999999`. `FeelNumber::pow` keeps its signature and has no domain of its own;
    `0 ** 0` stays 1 and an underflow stays 0 (`10 ** -999999999`). Test first:
    `an_exponent_outside_the_range_gives_null` failed on the parent with `1 ** 1e10` = 1; the
    test of `4 ** 1E+40` takes the new reason, and `4 ** 999999999` keeps the overflow reason.
    Identity: 12 registered differences in `registered/exponent-domain.json` (whole responses),
    all on the number oracle's edge vectors; the entry `1.000000001 ** 1000000000` of
    `registered/integer-powers.json` (item 104) is amended in place to the null, its `why`
    naming both rules. The oracle's last two exemptions are deleted: its list is empty, and its
    quick run judges every case right. No API change.
106. **A time offset prints its sign from its value.** DMN 1.6 §10.3.2.3.4: "A time in the
    semantic domain is a value of the XML Schema time datatype"; XML Schema Part 2 §3.2.7.3, the
    lexical representation of a timezone: "'+' indicates a nonnegative duration, '-' indicates a
    nonpositive duration". The text of an offset took its sign from the whole hours (`offset /
    3600`), which are 0 below one hour, so `time("10:00:00-00:30")` printed `10:00:00+00:30` and
    `time(10, 0, 0, duration("-PT30S"))` printed `10:00:00+00:00:30`: a text that names another
    value. An offset now has one type, `FeelOffset` (new file `feel-temporal/src/feel_offset.rs`):
    whole seconds within ±14:00, built by one range check, with one printer that writes `-` for a
    negative count and `+` otherwise (zero is `+00:00`, as before), then `hh:mm` of the
    magnitude and `:ss` when its seconds are not zero. `FeelZone::Offset` holds it, and `impl
    From<FeelOffset> for FeelZone` is the one place where the zero offset becomes UTC; offset
    literals and `TryFrom<i32>` go through it (`TryFrom<i32>` keeps its range in this item).
    Test first: `an_offset_prints_its_own_sign` (feel-evaluator, `bif_time.rs`) failed on the
    parent with `10:00:00+00:30`; `feel_offset.rs` has the printer's and the range's unit tests.
    Identity: 8 registered differences in `registered/offset-sign-from-value.json` (the `string`
    of a time and of a date and time with an offset below one hour, from a literal, `@`, `time(h,
    m, s, offset)` and `date and time(date, time, zone)`), with their requests and a control
    (`time offset`, unchanged) added to `requests/temporal.json`. Rust API break
    (rheon-feel-temporal): `FeelZone::Offset` holds a `FeelOffset` (was `i32`); `FeelOffset` is
    added with `ZERO` and `Display`; the `Display` of `FeelZone::Offset` changes for negative
    offsets below one hour.
107. **`time()` takes every offset within ±14:00 and none with a fraction of a second.** DMN 1.6
    Table 67, `time offset`: "The time offset duration must be in the interval
    [duration("-PT14H")..duration("PT14H")] as per the XML Schema Part 2 dateTime datatype"; XML
    Schema Part 2 §3.2.7.3: "Timezones are durations with (integer-valued) hour and minute
    properties". `time(hour, minute, second, offset)` accepted -11:59..+12:00 only (the range of
    XML Schema's recoverable timezone of a date, not of a time offset), so `time(10, 0, 0,
    duration("PT13H"))` was null, while literals took ±14:00; and it truncated the offset to
    whole seconds, so `duration("PT0.5S")` gave a UTC time and `duration("PT1H0.5S")` +01:00.
    Now `FeelOffset::from_duration` is the offset of a duration: whole seconds, then the one
    range check of `FeelOffset`; `time_4` builds the time in `FeelZone::from` of it, and any
    other duration gives null with the reason `time: offset is out of range: …` as before. Whole
    seconds stay accepted (the TCK's `time(11, 59, 45, duration("PT2H45M55S"))` is
    `11:59:45+02:45:55`). `FeelTime::offset_opt` and `FeelDateTime::offset` go through the same
    check. `TryFrom<i32> for FeelZone` and `err_invalid_time_zone_offset` are removed; the doc of
    `FeelZone::Offset` and the comment on `tzOffsetMinutes` (its ±18:00 is the host offset a
    request accepts, not a FEEL limit) are corrected. Test first:
    `an_offset_is_a_whole_number_of_seconds_within_14_hours` (feel-evaluator, `bif_time.rs`)
    failed on the parent with null for `duration("PT13H")`. Identity: 9 registered differences in
    `registered/time-offset-range.json` (whole responses: six offsets between ±12:00 and ±14:00
    that were null are times, and three with a fraction of a second that were times are null),
    with their requests and the boundary cases that keep their answers (`PT14H1S`, `-PT14H1S`,
    `PT14H0.000000001S`, the TCK's `PT2H45M55S` pair) added to `requests/temporal.json`. Rust API
    break (rheon-feel-temporal): `TryFrom<i32> for FeelZone` removed (`From<FeelOffset>`
    replaces it); `FeelOffset::from_duration` added; `FeelTime::offset_opt` and
    `FeelDateTime::offset` accept offsets within ±14:00 (were -11:59..+12:00).
108. **The minutes of an offset literal are below 60.** DMN 1.6 §10.3.2.3.4: a time is "a value
    of the XML Schema time datatype"; XML Schema Part 2 §3.2.7.3: timezones have "the minute
    magnitude limited to at most 59". The offset of a literal was the sum `3600 h + 60 m`
    unchecked, so `+05:99` was `+06:39`, `+13:60` was `+14:00` and `-00:60` was `-01:00`. Now
    `FeelZone::from_captures` gives `None` for minutes of 60 or more, so a time, date and time
    or `@` literal with such an offset is invalid (null with its literal reason), and so is the
    zone `"+05:99"` of `date and time(date, time, zone)` (`FeelZone::from_designator`). Test
    first: `test_from_captures` and `zone_designators` (feel-temporal) and
    `the_minutes_of_an_offset_literal_are_below_60` (feel-evaluator) failed on the parent with
    `Some(Offset(FeelOffset(23940)))` and `10:00:00+06:39`. Identity: 8 registered differences
    in `registered/offset-literal-minutes.json` (whole responses), four on requests of
    `requests/temporal.json`; the other requests and two controls (`-01:59`, `+05:59`) are added
    to it. Rust behaviour break (rheon-feel-temporal): `FeelTime`, `FeelDateTime` and
    `FeelZone::from_designator` reject an offset with minutes of 60 or more.
109. **An offset literal has no seconds.** DMN 1.6 §10.3.2.3.4: a time is "a value of the XML
    Schema time datatype"; XML Schema Part 2 §3.2.7.3: "The lexical representation of a timezone
    is a string of the form: (('+' | '-') hh ':' mm) | 'Z'". The offset pattern of time, date
    and time and `@` literals and of the zone of `date and time(date, time, zone)` took an
    optional `:ss`, so `time("10:00:00+05:30:15")` was a time with that offset and
    `@"2020-01-01T10:00:00+01:00:00"` one with +01:00. Now `OFFSET_PATTERN` is `±hh:mm` and
    `FeelZone::from_captures` has no seconds branch: such a literal or zone is invalid and gives
    null with its reason. An offset with seconds remains a value: `time(h, m, s, offset)` takes
    whole seconds (item 107), and the TCK requires `string(time(11, 59, 45,
    duration("PT2H45M55S")))` to be `"11:59:45+02:45:55"`; that text does not read back as a
    literal. Test first: `an_offset_literal_has_no_seconds` (feel-temporal and feel-evaluator)
    failed on the parent with `Some(Offset(FeelOffset(18001)))` and the time
    `10:00:00+05:30:15`. Tests changed with it: `test_from_captures`, `zone_designators`, the
    zone table of `date_and_time_from_date_time_and_zone_takes_the_zone_suffix_rules` (`+01:00:30`
    is invalid), `bif_date_time::_0038` (null, and the same value written `+05:01`),
    `bif_is::_0047`-`_0049` (written with `±hh:mm` offsets), and the non-compliant model N_0079,
    `_0013` and `_0016` with their bench copies (null; the model expects the text with offset
    seconds). Identity: 8 registered differences in `registered/offset-literal-seconds.json`
    (whole responses), four on requests of `requests/temporal.json`; the other four are added
    to it. Rust behaviour break (rheon-feel-temporal): `FeelTime`, `FeelDateTime` and
    `FeelZone::from_designator` reject an offset written with seconds.
110. **The offset of a zoned date and time is displayed with its seconds.** DMN 1.6 Table 48: a
    date and time with an IANA zone maps to an ISO 8601 text "suffixed by the IANA timezone id in
    rectangular brackets, e.g. 2007-12-03T10:15:30+01:00[Europe/Paris]". The wasm `display` built
    that offset with its own sign and minute arithmetic and dropped its seconds, so
    `@"1900-01-01T12:00:00@Europe/Paris"` showed `+00:09[Europe/Paris]` for the zone's +00:09:21
    (local mean time before the standard zones): the text of another instant. Now
    `zoned_date_time` (`wasm/feel-wasm/src/display.rs`) writes the offset the zone database
    resolves with `FeelOffset::write_seconds`, the one offset printer (item 106), which takes a
    plain count of seconds, since such an offset can have seconds and can lie beyond ±14:00; its
    own arithmetic is deleted. The seconds depart on purpose from the form Table 48 asks for:
    ISO 8601 writes an offset with hours and minutes only, as XML Schema Part 2 §3.2.7.3 does
    ("The lexical representation of a timezone is a string of the form: (('+' | '-') hh ':' mm)
    | 'Z'", the clause item 109 applies to literals); `±hh:mm:ss` keeps the instant, as
    java.time's zoned text does, where whole minutes would name another one. CONTRACT §3 states
    it. Test first:
    `zoned_date_time_display_keeps_the_seconds_of_the_offset` (`wasm/feel-wasm/tests/contract.rs`)
    failed on the parent with `+00:09[Europe/Paris]`. Identity: 3 registered differences in
    `registered/zoned-display-offset-seconds.json` (Paris 1900 as `date and time()` and `@`, New
    York 1880 `-04:56:02`), with their requests and two controls (Kolkata `+05:30`, the `time
    offset` of Paris 1900) added to `requests/temporal.json`. Rust API (rheon-feel-temporal):
    `FeelOffset::write_seconds` added.
111. **`second` and `seconds` are exact decimals.** DMN 1.6 Table 67: `second` is "The second of
    the minute as a decimal in the interval [0..60)", and `seconds` of a days and time duration
    "must return a decimal in the interval [0..60)". Both read the whole seconds only, so
    `time("10:00:30.5").second` was 30 and `duration("PT1.5S").seconds` 1. Now they come from
    `temporal_value::second` (feel-evaluator, `builders.rs`), which builds the exact decimal of the
    whole seconds and the nanoseconds with the same step as `value`; `seconds` stays a magnitude
    (`duration("-PT1.5S").seconds` is 1.5), like `days`, `hours` and `minutes`. The `Display` of a
    days and time duration takes its nanoseconds from the new `get_nanoseconds`. Test first:
    `second_and_seconds_are_exact_decimals` (feel-evaluator, `properties.rs`) failed on the parent
    with 30 for `time("10:00:30.5").second`; the pinned `seconds` of
    `duration("-PT9223372036.854775807S")` is 16.854775807 (was 16). Identity: 13 registered
    differences in `registered/exact-decimal-seconds.json`, five of them requests of
    `requests/temporal.json` already, with the other eight and three controls (whole seconds,
    `date("2020-01-01").second`) added to it. Rust API (rheon-feel-temporal):
    `FeelDaysAndTimeDuration::get_nanoseconds` and `FeelDateTime::nanos` added.
112. **The years and months of a duration are magnitudes.** DMN 1.6 Table 67: `years` is "The
    normalized years component of a years and months duration value as an integer", and `months`
    "must return an integer in the interval [0.. 11]"; the owner's decision is that every
    duration component is a magnitude. `years` and `months` were signed (`duration("-P1Y2M")` had
    -1 and -2), while `days`, `hours`, `minutes` and `seconds` of a days and time duration are
    magnitudes. Now `FeelYearsAndMonthsDuration::years` and `months` split the magnitude of the
    months count (`-P1Y2M` has 1 and 2), `is_negative` carries the sign, and the `Display` of the
    duration uses them instead of its own split. `value` keeps the signed months count. Test
    first: `the_components_are_magnitudes` (feel-temporal, every count in -30..=30: `months` in
    `0..12`, `12 * years + months` the magnitude) and `years_and_months_are_magnitudes`
    (feel-evaluator, `properties.rs`) failed on the parent with -6 months for -30 and -1 for
    `duration("-P1Y2M").years`. Identity: 7 registered differences in
    `registered/duration-component-magnitudes.json`, with their requests and six controls
    (positive durations, a negative one's zero component, `value` and the text) added to
    `requests/temporal.json`. Rust API break (rheon-feel-temporal):
    `FeelYearsAndMonthsDuration::years` and `months` return `u64` magnitudes (were signed `i64`).
113. **`years and months duration(from, to)` has one rule for both signs.** DMN 1.6 Table 73:
    "return years and months duration between from and to", with `years and months
    duration(date("2011-12-22"), date("2013-08-24")) = duration("P1Y8M")`. `&FeelDate - &FeelDate`
    chose its branch by the year alone, so a negative span inside one year took the day rule of a
    positive one: from 2020-05-15 to 2020-03-01 was `-P3M`, while from 2020-03-01 to 2020-05-15
    is `P2M`. Now it orders the two dates: the difference of their months, one fewer when the day
    of the later date is below the day of the earlier one, negated when `self` is the earlier
    date, so `a - b` is `-(b - a)`. Positive spans and negative spans across years keep their
    answers; date and time arguments still compare their dates alone. Test first:
    `a_negative_span_inside_one_year` and
    `the_months_between_two_dates_have_one_rule_for_both_signs` (feel-temporal: over a grid of two
    years, every month and the days 1, 15, 28, 31, antisymmetry, and for an earlier date whose day
    every month has, `a + (b - a) <= b < a + (b - a) + P1M`) and
    `the_duration_between_two_dates_has_one_rule_for_both_signs` (feel-evaluator) failed on the
    parent with `-P3M`. At a month end the day rule and the clamped addition differ
    (2021-01-31 plus `P1M` is 2021-02-28, while from 2021-01-31 to 2021-02-28 is `P0M`), so the
    grid's addition test leaves out the days 29 to 31 of the earlier date. Identity: 6 registered
    differences in `registered/years-and-months-between.json`, with their requests and six
    controls (the positive twins, a span across years, equal dates, the example of Table 73 and
    its reverse) added to `requests/temporal.json`. Rust behaviour break (rheon-feel-temporal):
    `&FeelDate - &FeelDate` gives new results for negative spans inside one year.
114. **The value of a time in a zone without a known offset is null.** DMN 1.6 §10.3.2.3.4: "If
    no time offset is specified, the time is interpreted as a local time of day", and `valuet` is
    the number of seconds since midnight; a time in a zone is valued in its UTC form. `of_time`
    (feel-evaluator, `temporal_value`) took a zone whose offset it could not resolve as UTC
    (`feel_time_offset().unwrap_or(0)`), so such a time had the value of another instant. Now a
    local time keeps its clock time, a time with an offset or a resolved zone its UTC form, and a
    time in a zone without a known offset gives null with the reason `value: the time zone of the
    time has no known offset`. A time in a zone resolves its offset on the current date, so the
    case is not reproducible through an expression (a zone name is checked when it is read; a
    time in a gap of daylight saving time has no offset only on such a day). Test first:
    `the_value_of_a_time_needs_its_offset`, a unit test inside `temporal_value` (local, UTC,
    `+01:00`, `Etc/GMT-1` and an unknown zone name), failed on the parent with 36000.5 for the
    unknown zone. Identity: no difference; four controls (a local, a UTC, a zoned and an offset
    time with a fraction) are added to `requests/temporal.json`. No API change.
115. **A years and months duration that drops a fraction of a month warns.** DMN 1.6 Table 60:
    a years and months duration times a number is `valueymd⁻¹(valueymd(e1) * value(e2))`, and
    divided by a number "If value(e2)=0, the result is null. Else the result is
    valueymd⁻¹(valueymd(e1) / value(e2))"; §10.3.2.3.8: "The value of a years and months duration
    can be expressed as a number of months", and the text is silent on a product or a quotient
    with a fraction of a month. The DMN TCK (compliance-level-3, 0100-arithmetic) expects it
    truncated toward zero: `@"P10Y11M" / 2.5` is `P4Y4M`, `@"P10Y11M" / -2.5` is `-P4Y4M` and
    `-2.5 * @"P1Y11M"` is `-P4Y9M`. The answers keep that truncation, as 0.4.0 gave
    them, and by the owner's decision the evaluation now says what it dropped: the three
    years and months duration sites of `build_mul` and `build_div` (feel-evaluator,
    `builders.rs`) call `temporal_value::ym_duration`, the one conversion of a number of months
    to a duration, which truncates and records the warning `fractional months of 'P10Y11M / 2.5'
    truncated to whole months; dropped fraction: '0.4'` through the warning channel of the
    truncated fractional seconds. A whole number of months, as FEEL computes it in 34 digits
    (`duration("P1Y") * (1/3)` is `P4M`), does not warn. A warning is no error: `diagnostics`
    and strict mode are unchanged. CONTRACT §3 lists the warning. Test first:
    `a_years_and_months_product_warns_when_it_drops_a_fraction_of_a_month`
    (`multiplication.rs`) and `a_years_and_months_quotient_warns_when_it_drops_a_fraction_of_a_month`
    (`division.rs`) failed on the parent with no warning. Identity: 13 registered differences in
    `registered/whole-months-truncation-warnings.json` (whole responses: the same result with
    the new warning), with their requests and fourteen controls (whole results, zero, a product
    out of range, a division by zero, days and time durations) added to `requests/temporal.json`.
    No API change.
116. **The nulls that the specification defines are pinned as no errors.** DMN 1.6 §7.3.8: the
    evaluation of a semantic error "MUST report or log diagnostic information and SHALL return
    null", while some inputs have null as their defined result: Table 51, before which "Because
    these are total functions, the input can be true, false, or otherwise", gives null for `and`
    and `or` with an operand that is no boolean unless the other operand decides (`false and x`
    is false, `true or x` is true); §10.3.4.4 has `sum([]) = null` and `mean([]) =
    null`; Table 82 has `get value({key1 : "value 1"}, "unexistent-key") = null`. These already
    answer a bare null with no error, and now tests hold them so: `te_errors` (feel-evaluator,
    `tests/mod.rs`, beside `te_warnings`, which shares its evaluation) asserts the value, the
    counted errors and their messages, and the `and`, `or`, `sum`, `mean`, `get value` and
    `[{a: 1}].b` tests assert none; `defined_nulls_are_no_errors`
    (`wasm/feel-wasm/tests/contract.rs`) asserts a count of 0 and the same result in strict mode;
    `a_missing_input_variable_of_a_built_in_type_is_a_null_without_an_error`
    (model-evaluator, `input_data.rs`) asserts a bare null and a count of 0. Tests only: they pass
    on the parent. `requests/nulls.json` is a new committed request source (71 requests) for
    these and the neighbouring nulls and errors (a missing context entry, `stddev` of fewer than
    two numbers, `if` with a condition that is not true, `!=` on kinds that `=` cannot compare, a
    null operand of `+`), lenient and strict, FEEL and B-FEEL. Identity: no difference. No API
    change.
117. **A missing context entry is a defined null with a note.** DMN 1.6 §10.3.2.6: "Selecting a
    value by key from context m in the semantic domain is denoted as m.key1 or get value(m,
    "key1")", and Table 82 defines `get value({key1 : "value 1"}, "unexistent-key") = null`;
    §7.3.8 makes a semantic error "report or log diagnostic information". `{a: 1}.b` gave the
    null with the reason `build_path: no entry b in context: {a: 1}`, counted as an error, so
    strict mode failed on it, while `get value({a: 1}, "b")` is a bare null without one. Now a
    reason is an error or a note: `rheon_feel::diagnostics::Severity` (`Error`, `Note`,
    non-exhaustive) says which, `record(severity, message)` records an error and returns at once
    for a note, and `null_with_reason(severity, reason)` builds the message once for both.
    `value_null!(note: …)` gives the null with a note, every other form an error null, and
    `value_null!()` is unchanged. The path's missing entry (feel-evaluator, `build_path`) is a
    note: the null keeps its text (`result.nullReason`, the `null(…)` of `result.feel` and the
    Rust `Display` are unchanged), it is not counted or listed in `diagnostics`, and strict mode
    does not stop at it. The module docs of `diagnostics` classify every null as a value, a
    defined null (bare or with a note), an error null or an abort. CONTRACT §3 and the
    `Diagnostics` type of `@rheon/feel-runtime` say that a note is no error. Test first:
    `a_missing_context_entry_is_a_null_with_a_note` (feel-evaluator, `path.rs`) failed on the
    parent with the message listed, and `a_missing_context_entry_is_no_error`
    (`wasm/feel-wasm/tests/contract.rs`) with a count of 1; `a_note_is_no_error` and
    `a_note_with_arguments_is_capped_like_any_message` (feel, `diagnostics.rs`) test the
    mechanism. Identity: 6 registered differences in `registered/missing-context-entry.json`
    (lenient, strict and B-FEEL `{a: 1}.b`, a nested path, and a path beside a division by zero,
    whose division is still counted), all requests of `requests/nulls.json`, three of them added
    in this commit; the entry of `{a: for i in 1..1000 return i}.b` in
    `registered/message-previews.json` is amended in place (its count is now 0). Rust API break
    (rheon-feel): `diagnostics::Severity` added; `diagnostics::record` and
    `values::null_with_reason` take the severity first; `value_null!` gains the `note:` forms;
    `Diagnostics::count` and `messages` leave out notes.
118. **`stddev` of fewer than two numbers is a defined null.** DMN 1.6 §10.3.4.4, stddev: "If
    the list is empty or if the list contains only one element, the function returns null", with
    `stddev( [ 47 ] ) = null` and `stddev( 47 ) = null`; its parameter is a "list of number".
    `stddev` (feel-evaluator, `bifs/core.rs`) tested the count first, with a counted error, so
    `stddev([47])` failed strict mode and `stddev(["a"])` was reported as too short; the
    positional form gave a number or a boolean as its one argument the error `[positional::stddev]
    invalid argument type, expected list`. Now the items are checked to be numbers first (an
    error null, `stddev: expected number, actual type is … with value …`), and then fewer than
    two numbers is the null with the note `stddev: minimum two input arguments expected`, not
    counted and not failing strict mode; one positional argument that is no list is a list of
    one item, as for `sum`, `mean` and `median`. Test first:
    `stddev_of_fewer_than_two_numbers_is_a_null_with_a_note` (`bif_stddev.rs`) failed on the
    parent with the message counted, and `stddev_of_fewer_than_two_numbers_is_no_error`
    (`wasm/feel-wasm/tests/contract.rs`) with a count of 1. The pins of `stddev(true)`
    (`bif_stddev.rs`) and of TCK 0063 `stddev(null)` and `stddev(47)` (`dmn_3_0063.rs`, test and
    bench) take the new texts. Identity: 15 registered differences in
    `registered/stddev-fewer-than-two-numbers.json` (lenient, strict and B-FEEL), all requests of
    `requests/nulls.json`, four of them added in this commit. No API change.
119. **A decision table without a matching rule is a defined null.** DMN 1.6 §10.3.2.10: "A
    decision table may have no rule hit for a set of input values. In this case, the result is
    given by the default output value, or null if no default output value is specified." The
    decision table evaluator (model-evaluator, `decision_table.rs`) gave that null with the
    counted reason `no rules matched, no output value defined`, so
    `evaluate_invocable_with_mode(…, ErrorMode::Strict)` turned it into the strict null. Now the
    reason is a note (item 117): the value keeps its text, the count is 0, and strict mode
    returns the same value. Test first: `no_matching_rule_is_a_null_with_a_note`
    (model-evaluator, `no_default_output_values.rs`) failed on the parent with a count of 1.
    Identity: no difference (the wasm module evaluates no decision tables). Rust behaviour break
    (rheon-model-evaluator): `evaluate_invocable_with_mode` counts no error for a decision table
    without a matching rule, and in strict mode returns its value.
120. **A missing input variable is a defined null whatever its type.** An input variable that
    is missing from the input context is missing data, and a null the specification defines for
    its input is no error (DMN 1.6 §7.3.8 covers semantic errors; the owner's list of defined
    nulls names the missing input variable). A variable of a built-in type gave a bare null with
    no error, while one whose type is an item definition (model-evaluator, `variable.rs`) gave
    the counted reason `no name … in context …`, so strict mode failed on it. Now that reason is
    a note (item 117): the value keeps its text, the count is 0. A structured input that lacks a
    component stays a counted error: how structured inputs are bound is not changed here. Test
    first: `a_missing_input_variable_of_an_item_definition_type_is_a_null_with_a_note`
    (model-evaluator, `input_data.rs`, model `tEmploymentStatus` with the context `{}`) failed on
    the parent with a count of 1. The same conversion reads the input decisions of a decision
    service invoked as a whole from the input context, so an input decision of an
    item-definition type that the context does not supply is the same missing data:
    `a_missing_input_decision_of_an_item_definition_type_is_a_null_with_a_note`
    (model-evaluator, `tests/various/dmn_0007`) failed on the parent with a count of 1.
    Identity: no difference (the wasm module has no input data).
    Rust behaviour break (rheon-model-evaluator): `evaluate_invocable_with_mode` counts no error
    for a missing input variable of an item-definition type, nor for such an input decision of
    a decision service that the context does not supply, and in strict mode no longer fails
    on it.
121. **`if` takes the else branch unless the condition is true.** DMN 1.6 Table 50 rule 45:
    `if e1 then e2 else e3` is "if FEEL(e1) is true then FEEL(e2) else FEEL(e3)", and "A
    conditional if a then b else c is equal to b if a is true, and equal to c otherwise." The
    `if` evaluator (feel-evaluator, `build_if`) took the else branch for `false` and null only,
    and gave a condition of another kind the counted error `condition in 'if' expression is not
    a boolean value`, so `if 9 then 1 else 2` was null and failed strict mode. Now every
    condition that is not `true` takes the else branch, with no error: `if 9 then 1 else 2`,
    `if "x" then 1 else 2`, `if [true] then 1 else 2` and `if {} then 1 else 2` are 2. The boxed
    conditional is unchanged: DMN 1.6 §10.2.1.8 says its "if" part "MUST resolve to a boolean",
    and TCK 1150 expects null for one that does not. Test first:
    `a_condition_that_is_not_true_takes_the_else_branch` (feel-evaluator, `if_expression.rs`)
    and `if_takes_the_else_branch_unless_the_condition_is_true`
    (`wasm/feel-wasm/tests/contract.rs`) failed on the parent with the null; the pin of
    `if N then 1 else 2` with `N = 9` (`if_expression.rs`) is now 2. Identity: 8 registered
    differences in `registered/if-else-unless-true.json` (the four conditions, lenient and
    strict), all requests of `requests/nulls.json`. No API change.
122. **`!=` reports like `=`.** DMN 1.6 §10.3.2.15: "By definition, FEEL(e1 != e2) is FEEL(not(e
    1= e2))". `=` on operands it cannot compare (`"a" = 1`) gives null with the counted reason
    `equal err '"a"' =?= '1'`, but `!=` (feel-evaluator, `build_nq`) gave a bare null with no
    reason, so strict mode stopped on `"a" = 1` and not on `"a" != 1`. Now one private function,
    `equal`, holds the body of `=`; `!=` negates its boolean and passes its null on, with the
    same reason, the same count and the same strict failure. In B-FEEL `"a" != 1` is still
    `true`, and the error behind it is listed, as for `=`. Test first:
    `not_equal_on_kinds_that_equal_cannot_compare_reports_like_equal` (feel-evaluator,
    `comparison_ne.rs`, every pair of kinds whose `=` the equality tests pin as `equal err`) and
    `not_equal_reports_like_equal` (`wasm/feel-wasm/tests/contract.rs`, lenient, strict and
    B-FEEL) failed on the parent with the bare null. The BACKLOG entry on `!=` is removed.
    Identity: 5 registered differences in `registered/not-equal-reports-like-equal.json`
    (lenient, strict, B-FEEL, B-FEEL strict, and a date against a number), all requests of
    `requests/nulls.json`. No API change.
123. **A null operand of an arithmetic operator is an error.** DMN 1.6, before Table 57 and
    before Table 59: "if input values are not of the listed types, the result is null"; Table 61
    takes "type(e1) is number. value(e2) is a number"; Table 63 defines `-e` as "e*-1". The text
    gives the value, null, and does not say whether a null operand is an error, so the official
    TCK decides: it marks every arithmetic operation with a null operand `errorResult="true"`
    ("invalid arithmetic combination", compliance-level-3/0100-arithmetic, the
    `error_when_{add,subtract,multiply,divide,exponent}_*` cases with a null on either side,
    `10 + null` and `null + 10` among them), and TCK 0085 decision 002 with a null input ("because
    of the string + null operation"). The operators decided it each in their own way: the
    addition (feel-evaluator, `build_add`) passed a null on when it was the left operand or stood
    beside a number (`1 + null`, `null + "a"`, no error) and otherwise counted an error with one
    of several reasons (`expected string as a second argument in addition`, `[builders::add]
    invalid argument type, …`); the multiplication passed a null left operand on (`null * 1`, no
    error); the subtraction and the division counted `[subtraction] incompatible types: 1 - null`
    and `[division] incompatible types: …`, the exponentiation `exponentiation base is not a
    number`, the negation `unexpected type in arithmetic negation: Null`. Now one private rule,
    `Arithmetic::null_operand`, decides for all six operators: with a null operand on either side
    the value is null with the counted reason `[<operator>] incompatible types: <operation>`, the
    style of the subtraction's and the division's type errors, which `Arithmetic::incompatible`
    writes with the operands as evaluated (`[addition] incompatible types: 1 + null`,
    `[negation] incompatible types: -null`), and strict mode fails on it; a failing operand
    counts its own error before it (`"a" + 1/0` counts the division's and the addition's). The
    binary operators go through one combinator, `arithmetic`, which evaluates both operands and
    hands them to `operate`, and the incompatible-types errors of the subtraction, the
    multiplication and the division come from `Arithmetic::incompatible` too, with their texts
    unchanged. `operate` holds the null rule and the operator and is never inlined: the
    combinator's frame stays on the stack while a nested operand evaluates, and with `operate`
    inlined the stack report's recursion reached 17 levels on Node's main thread with Liftoff,
    one fewer than the parent; out of line it reaches 22 there (parent 18), 46, 75 and 104 in
    workers of 2, 3 and 4 MiB (40, 64, 89), 24, 51, 83 and 114 with TurboFan (13, 29, 47, 65),
    and 289, the parser's limit, in Wasmtime with a 16 MiB stack (120); the smaller Wasmtime
    stacks reach the parent's 10, 26, 57 and 120. B-FEEL converts the null operand (§11.9-§11.11)
    and gives a value (`1 + null` and `2 ** null` are 1, `null - 6` is -6, `null * 1` is 0,
    `"a" + null` is `"a"`), and the FEEL error behind it stays listed, as for its other
    conversions (CONTRACT §2), so B-FEEL in strict mode fails on it, as it did for the
    subtraction (`null - 6`). Test first: `a_null_operand_of_an_addition_is_an_error`,
    `…_of_a_multiplication_…`, `…_of_an_exponentiation_…` and `…_of_a_negation_…`
    (feel-evaluator, one per operator's test file, the binary ones through `te_null_operands`:
    every kind on either side of a null, and a failing operand) and
    `a_null_operand_of_an_arithmetic_operator_is_an_error` (`wasm/feel-wasm/tests/contract.rs`,
    lenient, strict and B-FEEL) failed on the parent; `…_of_a_subtraction_…` and
    `…_of_a_division_…` passed there, as those operators already counted the error with this
    text. Pins that took the old answers: TCK 0100 (72 tests: 25 additions, 22 multiplications,
    25 exponentiations), TCK 0085 decisions 002 with a null input (three), TCK 2-0105 (three,
    which expect null and mark no error) and TCK 2-0003 (the allowed-values null of the input),
    each in `src/tests/compatibility` and in its bench copy; in feel-evaluator `null + @"P1DT2H"`,
    `null * @"P1Y"`, a date plus an out-of-range product and its negation, two function bodies
    that add a missing name, and a missing context entry plus 1 (`path.rs`); in model-evaluator
    the decision of `various/dmn_0007` that doubles a missing input decision, whose note stays
    uncounted while the multiplication counts its error; and `bbt/tests/cli/edm/1006_ERR`. The
    TCK runner of this repository compares values, which stay null, so its results are
    unchanged. The diagnostics module docs no longer count a null operand passed on among the
    defined nulls. Identity: 85 registered differences in `registered/arithmetic-null-operand.json`
    (30 FEEL, 16 FEEL strict, 23 B-FEEL, 16 B-FEEL strict), all requests of `requests/nulls.json`,
    which gains 146 requests (each binary operator with a null on either side of a number, of a
    string, of a null and of a failing operand, and the negation of null, of a failing operand and
    of a missing context entry, in the four modes): the operations whose null operand 0.4.0 passed
    on (`1 + null`, `null + 1`, `null * 1`, …) count an error now, the others keep their count
    with the new reason, and the subtraction and the division answer as in 0.4.0. Five entries of
    `registered/durations-symmetric-range.json` (the negation of an out-of-range product, and a
    date, a date and time or a time plus one) and the entry `({a: {b: 1}}.a.c) + 1` of
    `registered/missing-context-entry.json` are amended in place (the product's or the note's null
    and the operator's error). No API change.
124. **A unary comparison compares with the value of its path endpoint.** DMN 1.6 §10.3.2.7: a
    range is "a comparison operator and a single endpoint (grammar rule 7.a.)", and "Endpoints
    can be expressions (grammar rule 16)"; the grammar parses `< x.p` as the unary comparison of
    the path `x.p`. The path evaluator (feel-evaluator, `build_path`) looked at its parent node:
    under a unary comparison it wrapped a left value that is neither a context nor a list in the
    unary value first, read the property from that, and returned it as `Value::Transparent`,
    which the unary builders unwrapped. So `< x.p` was `(< x).p`: `< d.year` with `d` a date was
    null with `no such property in unary less: year`, and `> r.start` with `r` a range was `r`
    itself, so unary tests such as `< applicant.dob.year` never matched. Now a path is
    evaluated the same way wherever it stands, and the unary builders wrap whatever their
    endpoint gives: `< d.year` is `UnaryLess(2020)`, and in unary tests 2000 satisfies it, 20
    satisfies `> r.start`. A path whose property does not exist is a null endpoint, as `< null`
    is. A property of the unary value itself is read with parentheses, `(< 10).end`, the form
    TCK 0074 uses. The builder no longer keeps a stack of parent nodes, which had no other
    reader, and its lifetime parameter goes with it. Test first:
    `a_unary_comparison_compares_with_the_value_of_its_path_endpoint` (feel-evaluator,
    `properties.rs`, and `wasm/feel-wasm/tests/contract.rs` in the unary-tests dialect) failed
    on the parent with `false`. The 16 property tests of `>1.start` and the like
    (`properties.rs`) take the parenthesised form `(>1).start`, which keeps their answers; the
    three `Value::Transparent` assertions (feel, `tests/values.rs`) are gone. CONTRACT §2 says
    that path endpoints need no parentheses. Identity: 14 registered differences in
    `registered/unary-comparison-path-endpoints.json`, all requests of the new committed source
    `requests/unary-tests.json` (20 requests: path endpoints in unary tests and as expressions,
    and the parenthesised forms, which do not change). Rust API break (rheon-feel):
    `Value::Transparent` is removed.
125. **One collection evaluator for every simple type.** An item definition that is a collection
    of a simple type (model-evaluator, `item_definition.rs`) had ten copies of one closure, one
    per FEEL type, that differed only in the test of an element and the type named in the
    message. They are one closure now, built from the element test and that name, so a rule for
    the elements is written once. The messages stay as they were, including the two that differ
    from the rest: a collection of nulls says `expected null` for a value that is no list, and a
    collection of `Any` takes every element. A test pins them:
    `a_collection_of_a_simple_type_names_what_it_expected` (`item_definition.rs`), which passes on
    the parent as well. The compatibility tests of the model evaluator pass unchanged. Identity:
    no difference (the wasm module has no item definitions). No API change.
126. **An item definition that references another keeps its allowed values.** DMN 1.6 §7.3.3:
    "An ItemDefinition element may restrict the values that are allowed from typeRef, using the
    allowedValues attribute. allowedValues is an instance of UnaryTests that constrains the
    domain of the typeRef." The evaluator of an item definition whose typeRef names another item
    definition (model-evaluator, `item_definition.rs`) built its allowed values and dropped them,
    so `tSmall` (typeRef `tBase`, a number, allowedValues `[0..10]`) took 11. Now the value the
    referenced definition gives is tested against the allowed values; a null it gives keeps its
    reason, as the simple types pass a null through. Test first:
    `an_item_definition_that_references_another_keeps_its_allowed_values` (`item_definition.rs`,
    the new example model `item_definition/0701.dmn`) failed on the parent with 11 for 11. TCK
    0109, 0110, 0113 and 0119 are unchanged. Identity: no difference (the wasm module has no
    item definitions). Rust behaviour break (rheon-model-evaluator): an input whose item
    definition references another with allowed values is null with `value '…' is not allowed`
    when the value is outside them.
127. **Allowed values are tested against each element of a collection.** DMN 1.6 §7.3.3: "In
    cases where the isCollection attribute of an ItemDefinition is true, each element of the
    collection must satisfy the UnaryTests of the allowedValues, i.e. the allowedValues are
    projected onto the collection elements"; Table 26: "In case of a collection, it is projected
    on the collection elements." The three collection evaluators (model-evaluator,
    `item_definition.rs`: of a simple type, of a referenced type, of a type with components)
    tested the whole list, so a collection of strings with allowedValues `"A", "B"` rejected
    `["A"]` and `[]` with `value '["A"]' is not allowed`. Now `check_each_allowed_value` tests
    each element and names the first one that fails (`["A", "C"]` is null with `value '"C"' is
    not allowed`); `[]` is `[]`. An element that the referenced definition of a collection
    rejects (a value that is not null, made null) stays in the list with its null and reason, as
    item 126 does for a single value; every other element is tested, a null element included, so
    `["A", null]` is null with `value 'null' is not allowed`, as it is for a collection of a
    simple type (there with `expected string`). Test first:
    `allowed_values_are_tested_against_each_element_of_a_collection` (`item_definition.rs`, the
    new example model `item_definition/0702.dmn`) failed on the parent with `value '["A"]' is
    not allowed`. TCK 0109, 0110, 0113 and 0119 are unchanged. Identity: no difference (the wasm
    module has no item definitions). Rust behaviour break (rheon-model-evaluator): an input
    typed by a collection with allowed values takes a list whose elements each satisfy them,
    including the empty list, and a rejected list names its first failing element.
128. **An iteration context sees the variables bound before it.** DMN 1.6 §10.3.2.14: "Examples
    of valid iteration contexts are: ... x..x+10", and "A for loop expression will iterate over
    each element in the iteration context, binding the element to the corresponding variable";
    §10.3.2.15 binds the variables of `some` and `every` to the elements of the Cartesian
    product of their contexts, and "When the Cartesian product is empty, the some ... satisfies
    quantifier returns false and the every ... satisfies quantifier returns true". The DMN TCK
    (compliance-level-3, 0084-feel-for-loops, "for loop with second iterator depending on first
    iterator") expects `for x in [[1,2],[3,4]], y in x return y` to be `[1,2,3,4]`. The loops
    (feel-evaluator, `iterations.rs`) evaluated every context once, before the loop, with one
    special case for a context that is the bare name of an earlier variable, which did not skip
    an empty list: `for x in [[1,2],[],[3]], y in x return y` was `[1, 2, 2, 3]`, `for i in 1..3,
    j in 1..i return j` was null, and `y in x.items` and the contexts of `some` and `every` saw
    no earlier variable. Now one walk binds the variables of `for`, `some` and `every` in order:
    the domain of the first is evaluated in the scope of the expression, the domain of each later
    one in every iteration of the earlier ones with them bound, and a domain without items
    contributes no iteration. The walk keeps one open sequence per variable, so it has no
    recursion; a variable whose sequence ended is unbound, and the implicit `partial` of a `for`
    is unbound while a domain is evaluated, so a domain sees the variables before it only.
    Numbers and dates of an interval are still produced one at a time, with the rule that a step
    lost to rounding ends a number interval. The boxed `for`, `some` and `every`
    (model-evaluator, `boxed_expressions.rs`) use the same walk with their one variable and a
    copied iteration context. So a later context is evaluated once per iteration of the earlier
    ones: its errors are counted once per iteration (the message is listed once), and after an
    empty context it is not evaluated at all. A repeated variable is bound by the later context.
    Test first: `an_iteration_context_sees_the_variables_bound_before_it` and
    `a_later_iteration_context_is_evaluated_in_every_iteration_of_the_earlier_ones`
    (`for_expression.rs`) and `a_quantified_context_sees_the_variables_bound_before_it`
    (`some_expression.rs`, `every_expression.rs`) failed on the parent with `[1, 2, 2, 3]`, a
    count of 1, `false` and null. `tests/iterations.rs` is rewritten against `Iteration` and
    `Sequence`; its dependent case with an empty list now has no iteration for it. The BACKLOG
    entry on an empty list in a `for` with a dependent variable is removed. TCK 0003, 0016,
    0033, 0084, 1152, 1153 and 1154 pass as before. Identity: 21 registered differences in
    `registered/iteration-contexts-see-earlier-variables.json`, all requests of the new committed
    source `requests/iteration.json` (47 requests: dependent, independent and single contexts,
    errors in later contexts, empty contexts, `partial`, repeated variables, `some` and `every`).
    Speed (the median of the wasm engine, Apple M2, against the module before this change):
    single-variable loops of 10,000 iterations 0.92 to 1.02, independent two-variable loops of
    10,000 iterations 0.80 to 0.97, the dependent loop `for x in xs, y in x` over 100 lists of
    100 numbers 0.20 (the value of each earlier variable was copied again in every iteration);
    against 0.4.0 0.79 to 0.93 and 0.18. Rust API break (rheon-feel-evaluator):
    `ForExpressionEvaluator`, `SomeExpressionEvaluator` and `EveryExpressionEvaluator` are
    removed; `Iteration` (`boxed`, `collect`, `some`, `every`), `Sequence` (`of`, `empty`,
    `interval`) and `Domain` are added.
129. **A repeated iteration variable gives null.** DMN 1.6 §10.3.2.14 binds each element "to the
    corresponding variable", and §10.3.2.15 binds "keys n1, n2, etc." to the elements of the
    Cartesian product; the text is silent on a variable named twice. With the contexts bound in
    order (item 128) the later binding hides the earlier one, so `for i in [1,2], i in [3] return
    i` was `[3, 3]`, where 0.4.0 kept the earlier binding and gave `[1, 2]`. Keeping either
    binding silently answers a question the expression does not ask, so now the builder checks
    the names of the iteration contexts of `for`, `some` and `every` (feel-evaluator,
    `builders.rs`, `iteration`), and a name given twice builds an evaluator of null with the
    counted reason `duplicate iteration variable 'i'`. The boxed expressions have one variable.
    Test first: `a_repeated_iteration_variable_gives_null` (`for_expression.rs`) failed on the
    parent with `[3, 3]`. Identity: the three requests of a repeated variable in
    `registered/iteration-contexts-see-earlier-variables.json` are amended in place (`for`,
    `some`, `every`: null with the reason). No API change.
130. **`matches` and `replace` read their flags by one rule.** DMN 1.6 Table 75 note 2:
    "pattern, replacement, and flags SHALL conform to the syntax and constraints specified in
    clause 7.6 of XQuery 1.0 and XPath 2.0 Functions and Operators. Note that where XPath
    specifies an error result, FEEL specifies a null result." That edition's §7.6.1.1: "If there
    are characters present that are not defined here as flags, then an error is raised
    [err:FORX0001]"; "Letters may appear in any order and may be repeated"; and `x`: "whitespace
    characters (#x9, #xA, #xD and #x20) in the regular expression are removed prior to matching
    with one exception: whitespace characters within character class expressions (charClassExpr)
    are not removed". `matches` trimmed its flags and rejected `""` (`empty regex flags`), passed
    `x` to the engine, where `#` starts a comment and whitespace inside a class is dropped too
    (three string rewrites patched part of that), and applied `q` after the block-name rewrite;
    `replace` ignored letters that are no flags, dropped `q` beside `s`, `m` or `x`, escaped every
    character under `q` (so `b` became the word boundary `\b`), ignored flags that are not a
    string, and compiled the pattern without its flags when the flagged pattern was invalid. Now
    one private `Flags` (feel-regex, `flags.rs`) reads the flags of both: every character must be
    `s`, `m`, `i`, `x` or `q`, `""` is no flags, nothing is trimmed, and there is no fallback.
    `q` is not defined by the edition DMN 1.6 cites; it is the flag of XPath and XQuery F&O 3.1
    §5.6.2, kept as an extension: "all characters in the regular expression are treated as
    representing themselves, not as metacharacters", and "If it is used together with the m, s,
    or x flag, that flag has no effect". So under `q` the pattern is quoted (only the engine's
    special characters are escaped) and only `i` applies. `x` is applied by a scanner, never by
    the engine: whitespace outside character classes is removed, also after a backslash, so
    `"hello\ sworld"` reads `hello\sworld` as the clause's example expects, and `#` is an ordinary
    character. An invalid pattern's reason quotes the pattern as written, not the engine's form
    with its `(?i)` group. In the evaluator one `matches` replaces `matches_2` and `matches_3`;
    null flags are no flags for both built-ins (TCK 1111 `fn-null-flags`), other flags that are
    not a string are null (`replace: flags must be a string`), and one helper turns a
    `RegexError` into the result of all three regex built-ins. Test first: the flag tests of
    `bif_matches.rs` and `bif_replace.rs` failed on the parent (`matches("input", "pattern", "")`
    was null with `empty regex flags`; `replace("abc","[A-Z]","#","all unknown but i")` was
    `"###"`). TCK 1111 `K-MatchesFunc-5` (flags `" "`) stays null, now with `invalid regex flag:
    ' '`. 25 registered differences (`registered/regex-flags.json`), among them
    `replace("abc","b","x","q")`, which was `"xabcx"`, and `replace("a#b","#","-","x")`, which was
    `"-a-#-b-"`; `requests/regex.json` gains the 23 requests of the new tests that it did not
    hold. API breaks (rheon-feel-regex): `is_match` takes the flags (`""` for none) and
    `is_match_with_flags` is removed; `replace` takes `flags: &str` instead of `Option<&str>`;
    `RegexError::Invalid` is renamed `InvalidPattern`, and `RegexError::InvalidArgument` is added.
131. **A replacement string follows the F&O rules.** XQuery 1.0 and XPath 2.0 F&O §7.6.3, where
    S is the number of parenthesized sub-expressions and "N is the decimal number formed by taking
    all the digits that consecutively follow the $ character": N=0 is the whole match, 1<=N<=S
    the group (empty when it did not match), "If S<N<=9, then the variable is replaced by the
    zero-length string", and "Otherwise (if N>S and N>9), the last digit of N is taken to be a
    literal character ... and the rules are reapplied"; "A literal "$" symbol must be written as
    "\$"", "A literal "\" symbol must be written as "\\"", and FORX0004 (a null in FEEL) is raised
    for a `$` "that is not immediately followed by a digit 0-9 and not immediately preceded by a
    "\"", and for a `\` "that is not part of a "\\" pair, unless it is immediately followed by a
    "$" character". `replace` rewrote `$N` to the engine's `${N}` with a process-wide compiled
    regex and handed the rest to the engine's own syntax: `$0` named no group (`[$0x]` gave
    `[]`), `$10` with one group named group 10, `$`, `$$` and `$a` were accepted (`$$` as one
    `$`), `${1}` was the engine's named-group form, and `\$` and `\\` stayed two characters.
    Now a private `Replacement` (feel-regex, `replacement.rs`) reads the string once, after the
    pattern is compiled so that S is known, into literal text and group numbers, and is the
    engine's replacer for every match; an invalid string is null whether or not the pattern
    matches, with `invalid replacement string: a '$' that is not escaped must be followed by a
    digit` or `... a '\' must be followed by '\' or '$'`. The string is read from left to right,
    so in `\\$1` the pair comes first and `$1` is a group, and `\\$` ends in a lone `$`. The
    group digits are read with saturating arithmetic, so a long run of digits cannot overflow.
    With `q`, the F&O 3.1 extension of item 130, "the replacement string is used as is" (§5.6.4).
    The process-wide `REPLACEMENT_GROUP` is deleted. The forms of the TCK (`[1=$1][2=$2]`,
    `a$1$1`, `$1c$2`, `($1) $2-$3`, `[$0]`) keep their answers; TCK 1109 passes 28 of 28. Test
    first: the replacement tests of `bif_replace.rs` failed on the parent (`replace("abc","b","$")`
    was `"a$c"`, `replace("abc","b","\\$")` was `"a\$c"`, `replace("abc","b","[$0x]")` was
    `"a[]c"`). 15 registered differences (`registered/regex-replacement.json`), among them
    `replace("abcde","(a)(b)(c)(d)(e)","$23")`, the clause's own example, which was `""` and is
    `"b3"`; the entry `replace("abc","b","$1","q")` of item 130 is amended in place from `"ac"`
    to `"a$1c"`; `requests/regex.json` gains the 9 requests of the new tests that it did not hold.
    API break (behaviour, rheon-feel-regex): `replace` reads the replacement by these rules and
    returns `RegexError::InvalidArgument` for an invalid one.
132. **`replace` and `split` reject a pattern that matches the zero-length string.** XQuery 1.0
    and XPath 2.0 F&O §7.6.3: "An error is raised [err:FORX0003] if the pattern matches a
    zero-length string, that is, if the expression fn:matches("", $pattern, $flags) returns true.
    It is not an error, however, if a captured substring is zero-length." §7.6.4 (`fn:tokenize`,
    which DMN 1.6's `split` is without flags) raises the same error, and "If $input is the empty
    sequence, or if $input is the zero-length string, the result is the empty sequence"; a
    separator at the start or the end, and two adjacent separators, give zero-length strings.
    `replace` and `split` iterated over the engine's empty matches: `replace("abc","x*","-")` was
    `"-a-b-c-"`, `replace("abc","^","-")` was `"-abc"`, `split("abc","")` was
    `["", "a", "b", "c", ""]`, `split("a,b",",*")` was `["", "a", "b", ""]`, and `split("", ";")`
    was `[""]`. Now the one compile of the two built-ins (feel-regex, `impl_regex.rs`, `compile`)
    tests the compiled pattern, flags included, against `""` once and returns
    `RegexError::InvalidArgument` with `the pattern matches the zero-length string`, a null in
    FEEL; `split` of `""` is `[]` (owner decision), and every other input keeps the engine's
    split, which already gives tokenize's zero-length strings (`split("1,15,,24,50,",",")` is
    `["1", "15", "", "24", "50", ""]`, the clause's example). The pattern is checked before the
    input, as an invalid pattern is, so `split("","x*")` is null, not `[]`; an engine limit and an
    invalid pattern come before the zero-length test. Test first: the zero-length tests of
    `bif_replace.rs` and `bif_split.rs` failed on the parent (`"-a-b-c-"`, `[""]`, `["", "a",
    "b", "c", ""]`). TCK 0067 and 1109 use no such pattern and pass. Every entry of
    `registered/regex-engine-limits.json` still reproduces (`split("a", <100 nested groups>)` is
    `["", ""]`). 11 registered differences (`registered/regex-zero-length-matches.json`);
    `requests/regex.json` gains the 13 requests of the new tests that it did not hold. API break
    (behaviour, rheon-feel-regex): `replace` and `split` return `RegexError::InvalidArgument` for
    a pattern that matches the zero-length string, and `split` returns no item for an empty
    input.
133. **CI documents the published crates with warnings denied.** Job `engine` runs `cargo doc
    --no-deps` on the ten published crates with `RUSTDOCFLAGS=-D warnings`, so an intra-doc link
    that resolves to nothing, or any other rustdoc warning, fails the run; the README's script
    table names the step. The module docs of rheon-feel-wasm linked its three ABI exports, which
    exist only on wasm32, and now write them as code, so that crate documents without a warning
    too. The step was checked against a tree whose rheon-feel-number docs still quoted DMN 1.6's
    "pairs [that] have the same value" with bare brackets (item 97 escapes them): it failed there
    with `unresolved link to 'that'` (exit 101), and it passes here. No API change.
134. **CI runs on Node 24.** The identity check answers every registered request with the
    previous release's module. One of them,
    `{big: for i in 1..100000 return i, r: for j in 1..1000 return big = j}.r`, makes the 0.4.0
    module exhaust its memory while it builds its null reasons (item 68 bounds them). Node 24 and
    Node 26 report that as a trap of the module, which the ledger registers. Node 22.23 crashes
    itself with a segmentation fault instead (6 of 6 runs on macOS arm64, and on the ubuntu
    runner), so the check ended with status 139. The `ci` and `release` workflows now set up
    Node 24, and `tools/identity/README.md` states the requirement. A request that exhausts the
    memory of this module (a list doubled 30 times) ends as a trap on Node 22 as well. For
    hosts: a 0.4.0 module on Node 22 can take the whole process down on such a request, not only
    its worker. No API change.
135. **The benchmarks that need a nightly toolchain are removed.** Five crates had a `benches/`
    directory (model-evaluator 156 files, feel-number 11, feel-parser 6, feel-evaluator 4, feel 3;
    36,510 lines), and every file needed `#![feature(test)]`. `rust-toolchain.toml` pins a stable
    compiler, so no gate and no CI step compiled them, and `cargo check --workspace --all-targets`
    failed on the first of them (E0554). The model-evaluator ones were a second copy of the TCK
    tests in `src/tests/compatibility`: 3,591 `#[bench]` against 3,594 `#[test]`, kept in step by
    hand, and 16 of the 151 copied files already held other string literals (expected answers and
    input contexts) than their tests, 177 in all. The answers stay pinned by the tests, and the
    speed of the engine is measured on the wasm module. Removed with them: the Taskfile tasks
    `bench`, `benchmarks-dry` and `benchmarks`, the `benchmarks.txt` ignore line, and the two
    model-evaluator tests that only printed the number of tests beside the number of benches (so
    the workspace runs 2 tests fewer), with the `antex` and `walkdir` dev-dependencies they alone
    used; the table at the top of the compatibility tests loses its benches column. A benchmark
    of the TCK models, when one is built, runs on the stable toolchain and states no expected
    answer: it takes the models and inputs from the pinned TCK checkout of `scripts/tck.sh` and
    times the model build (parsing and `ModelEvaluator::new`) and each test case's evaluation with
    its own timing loop, one row per model, so nothing in it can drift from the tests. No API
    change.
136. **A ledger entry of the identity check can state a rule.** A change that removes the same
    kind of diagnostic from many requests needed one ledger entry per request. A ledger file may
    now also hold rule entries (`tools/identity/README.md`, "Rule entries"): a name, the basis, an
    optional pattern the request's expression must match, the paths where the two responses may
    differ, each with its relation (`gains` or `loses` elements that match a pattern, `decreases`
    or `increases` as exact integers, `rewrite` by a pattern and its replacement), and examples in
    the whole-response form. A difference of a request with no entry of its own is explained by
    the first rule, in ledger order, whose relations hold at its paths and outside of which the
    two responses are identical; a trap is never explained. Each example is run like an entry
    (the reference answers `old`, the candidate `new`) and must be an instance of its rule, so a
    rule cannot go stale, and an example's request is registered at most once like any other.
    The summary counts the explained differences per rule, `--explained <out.json>` lists them
    for review, and `--record` drafts only what nothing explains. The relations are pure
    functions in `rules.mjs`, tested on literal responses (`npm run test:identity`, 18 tests, the
    6 that need two modules that differ skipped while the ledger is empty), and the lossless JSON
    helpers move to `json.mjs`; the cache format is 2 and its key covers the three files. Checked
    by hand with the 0.4.0 module as the reference and 0.5.0 as the candidate on
    `requests/nulls.json`: a rule for a dropped `no entry` error (count down, messages lose
    `no entry`) with one example explained 3 of the 120 differences, the other 116 stayed
    differences, and the same rule stated as an increase was refused (exit 2, `diagnostics.count:
    1 to 0 is not an increase`). No API change.
137. **The identity check has request sources and a ledger for the release tier.** Some requests
    cost the 0.5.0 module seconds each before it exhausts its memory. The two large request sets
    kept outside the repository were run once against 0.5.0 (7,845 requests with the committed
    ones, 480 s, peak 3.5 GB): 22 trap both sides, 19 of them on memory exhaustion after more than
    a second each and 3 at once. The 17 of those 19 that are short texts are committed as
    `tools/identity/release/requests/memory-exhausting.json`: 15 that double a list or a context
    30 times through `partial` and then count, compare, search, render or walk it (1.2 to 1.7 s
    each), and 2 that nest a list or a context 100,000 levels deep by a loop (158 s and 35 s); the
    other 2 are 2 MB texts (100,000 context entries that each nest the one before, 1.2 s each)
    and stay in the large set. `tools/identity/run.sh` reads `release/requests/` and the ledger
    `release/registered/` only with `--release`; otherwise `equal.mjs --exclude` (new) leaves
    those requests, with their parse twins, out of every other source and counts them in the
    summary, so no run after a build evaluates them, while a ledger entry is always run.
    `run.sh --large` adds the sets that `IDENTITY_LARGE_SETS` lists (colon-separated), and the
    npm scripts `identity` and `identity:large` run the check with and without them. A change
    that alters the answer of a release-tier request registers it in `release/registered/`; at a
    release that directory is emptied with `registered/`. `run.sh --release` against 0.5.0: 7,534
    requests, the 17 trap both modules, 0 differences, 427 s, peak 3.7 GB; the run without it is
    unchanged (7,502 requests, 0 left out). No API change.
138. **The TCK run uses pinned inputs and fails on a changed result.** `scripts/tck.sh` cloned the
    official TCK, DecisionToolkit's patches and its runner at the head of their default branches,
    and printed the comparison with `docs/tck-baseline.txt` but always exited 0, so it could not
    serve as a gate and its inputs could move under it. Now `scripts/tck-pins.txt` names the
    commit of each (tck `5cd912f`, dsntk-tck-patches `8dfc54e`, dsntk-test-runner `3276110`, the
    inputs of the recorded baseline); a clone already at its pin is used offline, any other is
    fetched at the pin, and `--refresh` is gone (moving a pin is a commit with the baseline it
    gives). `scripts/tck-summary.mjs` sets the exit status: 1 for a newly failing or newly passing
    test, a baseline test with no row, or a result file without rows, 2 for a usage error; with
    `--only` or `--addendum-only` the script passes `--partial`, which compares only the tests that
    ran. `--baseline <file>` names another baseline, and its checks are
    `npm run test:tck-summary` (7 tests, failing on the previous summary, which took no options
    and exited 0). The runner is built into its own target directory and the server is found
    through `cargo metadata`, so the run works with any `CARGO_TARGET_DIR`. Run here: official
    3,392 of 3,407, addendum 312 of 312, 0 newly failing, 0 newly passing, 0 not run, exit 0, 86 s
    with the server's release build. No API change.
139. **The three test tiers are commands with measured times and budgets.** `scripts/tier.mjs`
    (`npm run tier:fix`, `tier:group`, `tier:release`; the README's "Test tiers") runs each tier as
    a list of steps and ends with a table of every step (build or run, seconds, result), the build
    and run totals, the budget and the verdict. The fix tier runs format, compilation of every
    target, the fix's own test, the tests of the touched crates (named with `-p` or derived from
    the files that differ from the merge base with `main` through `cargo metadata`), the wasm build
    and the identity check, adds the Rust copies of the TCK tests and each release-tier ledger
    file when a ledger changes and a tool's own tests when the tool changes, and stops at the
    first failure. The group tier runs the whole workspace and every check of the module and the
    official TCK; the release tier adds the large and slow checks, and requires the inputs kept
    outside the repository unless a `--without` flag lists the step as NOT RUN. Test steps are
    split into their `--no-run` build and their run, and the budget (fix 2 min, group 6 min,
    release 30 min) judges the run total only; with `CI` set the times are reported without a
    budget. `npm run test:tier` checks the runner on step tables of its own (a failing step,
    stopping, a skipped dependant, the budget, a missing outside input, usage errors). The
    toolchain gains the `clippy` component, so clippy is the pinned one everywhere.
    `scripts/build-wasm.sh` and `scripts/tck.sh` find cargo's target directory through the new
    `scripts/cargo-target-dir.sh` instead of assuming `./target`, and `RHEON_WASM_TARGET_DIR` gives
    the tiers' wasm steps a target directory of their own. The fix tier of this change: 3.0 s of
    run time, 25.9 s of builds. No API change.
140. **CI runs the group tier on pull requests, and the release tier before a release.** The two
    jobs `engine` and `wasm` of `ci.yml` become one job `group` that sets up the toolchain, Node
    24, the wasm tools and the oracle's Python environment and runs `npm run tier:group`, then
    uploads the tier's logs, `feel.wasm` and the playground; the Pages deploy needs it. The group
    tier adds clippy with warnings denied, the tools' own tests and the official TCK at its pinned
    commits to what CI ran. `release.yml` gains a job `tier` that runs `npm run tier:release --
    --without-large-sets --without-speed-suite` on a tag and on a manual dispatch, and the
    publishing job needs it, so a failing tier builds and publishes nothing; the large request sets
    and the speed suite stay outside the repository, so that run lists them as NOT RUN and a
    release is fully checked only by the release tier on the reference machine. `cargo doc` of the
    published crates (item 133) moves from every push to the release tier. On CI the tiers report
    their times without a budget until five runs give one (1.5 times the slowest, the job timeout
    twice that); until then the jobs time out at 60 and 120 minutes. The documents that named the
    CI jobs name the tiers. The workflows were not run here: they are seen on the first push. No
    API change.
141. **The identity check's Node requirement and times are stated against 0.5.0.** The identity
    tool's README said to use Node 24 or later because one registered request made the 0.4.0
    module crash Node 22; with 0.5.0 as the reference no committed request does, so it now says
    that Node 21 or later is enough against 0.5.0 and that Node 24 or later is still needed for a
    reference of 0.4.0 or older (a bisect of older commits, a comparison with an old release).
    `engines.node` stays `>=21`, and CI stays on Node 24, the version its times are measured on, as
    the workflow comments now say. The table of times is measured again against 0.5.0 (Apple M2,
    Node 26): the committed sources 7,502 requests, 1.0 s uncached and 0.6 s cached (8.4 s and 7.5 s
    against 0.4.0); with the large sets 7,813 requests, 38 s uncached and 22 s cached; the release
    tier's `--release --verify-cache` 7,534 requests in 427 s and `--large --verify-cache` in 39 s,
    0 cache mismatches, peaks 4.1 GB and 2.2 GB. The README gives the times of the tiers from their
    first runs: the group tier 62.1 s of run time and 15.4 s of warm builds, every step passing;
    the release tier 1,029 s of run time and 51 s of builds, every step passing except the speed
    suite, whose `bench.mjs` (outside the repository) loaded a comparison engine that is not
    installed here; with that load made optional it ran in 54 s, a geomean of 1.00x between the
    release module and the build of the same engine. Running the committed sets on Node 22
    against 0.5.0 was not checked here (no Node 22 is installed). No API change.
142. **Requests and tests on values that are used again after they were passed on.** The value
    rework changes how lists, contexts and frames are stored and shared, and its risk is a write
    through one holder of a value that shows through another. `tools/identity/requests/values.json`
    (105 requests) uses a list, a context or a string again after a built-in, an operator, a
    filter, a loop, a call or a closure derived a new value from it (`append`, `context put` with
    a key and a key list, `sort`, `reverse`, `list replace`, `insert before`, `remove`, `distinct
    values`, `union`, `concatenate`, `flatten`, `sublist`, `context merge`, `get entries`, `+` on
    strings), with items with and without an entry `item` and the input context aliased in both
    context formats; it nests a value 100 and 499 levels deep through each channel that builds one
    (`partial`, a recursion around list literals, a chain of context literals, ranges, closures,
    `context put` with a key list) and walks it with `count`, `=`, `instance of`, `string`,
    `flatten` and `...`; and it doubles a list and a context 10 times through `partial`, so their
    items share the value before them. 0.5.0 answers every one of them (0.8 s for the set). A list
    and a context nested 499 levels deep through `partial`, and their walks, cost 0.5.0 0.7 s and
    5.6 s each, because it copies `partial` in every iteration, so these 12 requests are
    `tools/identity/release/requests/values-deep.json` (44 s on each side). The same expressions,
    except those 12, are tests in `feel-evaluator/src/tests/values_used_again.rs` with the answers
    they have (the value text and the number of errors); the deep values are compared with their
    text built by the test. Identity against 0.5.0: 7,710 requests in 2.7 s uncached and 1.4 s
    cached; with the large sets 7,905; with `--release` 7,766 in 513 s (peak 3.5 GB); 0
    differences. No API change.
143. **What a boxed context writes into an iteration or an item frame is pinned.** A boxed context
    writes each named entry into the context on the top of the scope. In the body of a boxed
    `for` that is the iteration context, as the match of a boxed filter the context of the item;
    no FEEL expression reaches this path, so the identity check cannot see it. The model of
    `model-evaluator/src/tests/various/dmn_0008` pins today's answers: an entry written in one
    iteration or for one item is not seen by the next (`[1, 2, 3]`; `[{a: 2}, {a: 3}]`), nor by
    the entries before it (`[[0, 10], [0, 20], [0, 30]]`; all three items); an entry named
    `partial` replaces the results of the iterations before it, by the list it holds (`[3, 1]`
    for `[x]` over `[1, 2, 3]`) or by no results when it is not a list (`[3]`); an entry named
    `item` is read by the entries after it and leaves the items as they were (`[2, 3]`); and an
    entry named as an entry of a context item is not read by the entries after it (`[]` for
    `a: a * 10` then `a > 15`). A change to how frames are stored or shared that alters one of
    these answers states it here. Tests only; no API change.
144. **The ordering of `sort` takes a predicate that can stop.** `order_by`
    (`feel-evaluator/src/bifs/sorting.rs`) took a predicate that always answered, so a sort ran to
    its end whatever the predicate's calls did. The predicate now answers `Some(precedes)` or
    `None`, and at the first `None` the ordering returns `None` without calling it again, from the
    insertion of a short list, the runs of a long one and its merges alike (the internal helpers
    pass the stop on with `?`). `sort` answers `Some` for every call, so nothing it returns changes;
    a `None` would give null. The unit test stops the predicate at its first call, at a call during
    the runs and at a call during the merges, for 2 to 5,000 items, and checks that the predicate
    was called exactly that many times; on the parent it does not compile, because the predicate
    cannot stop there. Identity step; no API change (the function is private to the crate).
145. **The identity tool's cache test holds when the recorded differences trap the reference.**
    `npm run test:identity` checks that a run with a warm cache answers as one without, on
    requests of the committed ledger that the two modules answer differently. It asserted that
    some of those answers came from the cache, but a trap is never cached (it is evaluated again
    on every run), so a ledger whose entries all trap the reference failed the test although the
    cache worked as specified. The test now asserts the exact counts: every answer of the cold
    run comes from the cache except the traps, which are evaluated again. With the ledger empty
    the test is skipped as before. No API change.
146. **An abort stops every loop.** An evaluation stopped by a limit (the call depth, a regex
    engine limit) refused every further call, but a loop went on through its items: a `for` over
    10,000,000 items held a null for each and exhausted the module's memory (a trap where the
    evaluation error was due), an `every` over 10,000,000,000 combinations ran for minutes, and
    the work after the abort also recorded errors no one reads. `rheon_feel::diagnostics::stopped()` (new, public; its callers are in
    rheon-feel-evaluator) tells whether the running evaluation was stopped, from the same state
    the call guard reads, and is `false` outside an evaluation. The walk of `for`, `some` and
    `every` (`Iteration::walk`, also the boxed forms) reads it before each further item, a filter
    before each item and before it evaluates its expression once more for an index, `sort`
    before each comparison (its ordering then ends, item 144) and `list replace` after each call
    of its function; each then ends with a null, which is never observed, because the abort is
    the outcome. An evaluation that ends answers as before. Two answers change, registered in
    `tools/identity/registered/abort-stops-loops.json` with their requests in
    `wasm/feel-wasm/tests/corpus/no-trap.json`: `for i in 1..10000000 return f(i)` and
    `for i in 1..100, j in 1..100000 return f(j)` with a runaway `f` trapped 0.5.0 after 0.4 s
    and are now the call-depth `evaluation` error (DMN 1.6 §7.3.8: the evaluation "MUST report or
    log diagnostic information"; CONTRACT §2). The corpus also gains the same loops, filters, `sort`
    and `list replace` over 100,000 items, which 0.5.0 already answered with that error, and the
    contract test asserts the error for each kind of loop. Natively, `diagnostics::collect` around
    an aborted evaluation now sees the errors recorded up to the abort only: in the tests, a loop
    over 1,000 items records as many as the same loop over its first item (2 against 1,001 for
    `for i in 1..1000 return [f(i), 1 + null]`), and `list replace` no longer records "matching
    function must return boolean value" for the refused call. Measured (wasm, Node 26): the
    traced `for i in 1..1000000 return f(i)` 107.5 ms on 0.5.0 and 0.4 ms now (a second call on
    one instance); `every i in l, j in l satisfies f(j) = null` over two lists of 100,000 items
    397 s on 0.5.0 and 20 ms now (most of it building the list); `sort` and `list replace` over
    100,000 items 37 ms and 40 ms on 0.5.0, 11 ms each now. The speed suite against 0.5.0 in two
    alternating rounds: geomean 1.005x and 1.003x over 39 workloads, 1.001x and 1.002x over the 86
    parse workloads, no workload below 0.97x in both rounds (`loop-count-10k` 1.012x and 1.003x,
    `list-filter-count` 1.009x and 0.995x, `sort-10k` 1.004x and 1.006x, `list-replace-10k`
    1.006x and 1.001x, `fib15` 1.051x and 1.002x). Allocations, reallocations and peak live bytes
    per call are those of 0.5.0 for every workload (native counting harness); the stack report
    and the string doubling's peak memory (n = 17 to 26) are identical. Rust API: the added
    function `rheon_feel::diagnostics::stopped`.
147. **A context has its own read API and no longer derefs to its map.** `FeelContext`
    dereferenced to the `BTreeMap` that holds its entries, so every caller could read the map
    directly and the way a context stores its entries was part of the API. The context now reads
    through its own methods: `iter` (an `ExactSizeIterator`, in the order of the names),
    `keys_from(name)` (the names from the first one not less than `name` on, which
    the parser's bound on the keys whose first word is a given word reads), and `contains_entry`,
    which takes a `Name` or its text. The parser, `context` and `=` read the context through
    them. Identity step: no answer changes. Rust API (`rheon-feel`): `impl Deref for FeelContext`
    is removed (use the methods above, `get_entry`, `len` and `is_empty`); `FeelContext::iter`
    returns `impl ExactSizeIterator<Item = (&Name, &Value)>` instead of
    `std::collections::btree_map::Iter`; `contains_entry` is generic over `Q` with `Name: Borrow<Q>`
    (a `&Name` argument compiles as before); `keys_from` is added.
148. **The named-parameter and positional-parameters values that nothing builds are removed.**
    `Value::NamedParameter` and `Value::PositionalParameters` came from upstream, but no code
    builds either value (the evaluator binds an invocation's arguments from the syntax tree), so
    they were matched only to display them and to give their type `Any`. Identity step: no answer changes. Rust API (`rheon-feel`): the variants
    `Value::NamedParameter` and `Value::PositionalParameters` are removed (`Value::NamedParameters`
    stays).
149. **`assign_entry` returns the value it replaces.** `FeelContext::get_entry_mut` handed out a
    `&mut Value` into a context, a write that the context cannot see; the next commits keep the
    nesting depth of a context, which such a write would bypass. `FeelContext::assign_entry` now
    returns the value it replaces (`None` for a new entry), and the two callers of
    `get_entry_mut` take their value back through it: `for` takes `partial` out of the iteration
    context, and a filter takes the item out of its `{item: …}` context, by assigning a null
    placeholder, as they did by hand. When the item context no longer has `item` the filter gives
    the lost-context null as before. Identity step: no answer changes. Rust API (`rheon-feel`):
    `FeelContext::get_entry_mut` is removed; `FeelContext::assign_entry` returns `Option<Value>`.
150. **A list value is a type of its own.** `Value::List` held a `Vec<Value>`, so every holder of a
    list could change its items in place, and how a list stores its items was part of the API. It
    now holds `FeelList` (`rheon_feel::values::FeelList`), which reads its items as a slice
    (`Deref<Target = [Value]>`), changes only through its own `push`, and gives its
    items up with `into_vec`; it is built from a `Vec<Value>` without a copy (`From`), from an
    iterator (`FromIterator`), iterates by value and by reference, compares its items and prints as
    the slice of items in its debug form, so `Value::List` debug-prints as before. There is no
    `&mut` access to the items. The built-ins that edit a list (`insert before`, `insert after`,
    `remove`, `reverse`, `list replace`) take a copy of the items with `to_vec`, as they cloned the
    list before, and build the new list from it; `for` keeps its results as a `FeelList`; a filter
    and a loop over a list take its items out with `into_vec`, as they moved the `Vec` out before.
    Identity step: no answer changes. Rust API (`rheon-feel`): `Value::List` holds `FeelList`
    instead of `Vec<Value>` (build one with `vec.into()` or `FeelList::from(vec)`, read it as a
    slice, edit it through `into_vec`); `FeelList` is added; `values_to_string`,
    `values_to_feel_string` and `values_to_jsonify` take `&[Value]` instead of `&Vec<Value>` (a
    `&Vec<Value>` argument compiles as before). `Values` stays `Vec<Value>`, the type of the other
    variants that hold several values.
151. **Lists and contexts keep their nesting depth.** A limit on how deeply values nest needs the
    depth of a value where it is produced, and a walk over the whole value there would make every
    loop quadratic. A list now keeps the nesting depth of its deepest item: a push raises it in
    constant time and a list built from its items computes it in the same pass (a list has no
    other way to change). A context remembers its depth once it was read, until an entry changes:
    every method that changes an entry (`set_entry`, `insert`, `assign_entry`, `remove_entry`,
    `set_null`, `zip`, `overwrite`, `move_entry`, `create_entries`, `create_entry`,
    `apply_entries`) goes through one accessor that forgets it, so a write costs one store, and a
    context that is only written, such as the iteration context of a loop, never computes it; the
    next read looks at each entry once. The remembered depth is no cache: it is derived from the
    entries alone, it is exact whenever it is set (it goes from not known to the depth on a read,
    and back on a change), a clone copies it with the identical entries, and two reads of one
    context, through any handles and on any threads, store the same number. A context kept exact on every write instead, as a list is,
    made `count(for i in 1..10000 return i)` 0.70x as fast as 0.5.0 (three writes of the iteration
    context per item), and 0.88x with the depth of a scalar read inline. The depth of a scalar is 0;
    a list or a context is one level deeper than the deepest value it holds, so an empty one is
    1 level deep; a function is one level deeper than its closure context, so 2 when it captures
    nothing; a range, an interval end, a unary test and the other values that hold values are one
    level deeper than what they hold. Nothing reads it yet. The
    tests in `feel/src/tests/depth.rs` change random contexts 60,000 times through every method
    that writes an entry, build random lists by push, from a vector and from an iterator, and
    compare each depth with a walk of the whole value, the changes that make a value shallower
    included. Equality, cloning and the debug forms are unchanged; `Value` stays `Send + Sync`
    (the remembered depth is an atomic). Identity step: no answer changes; no API change (the
    depth is read inside `rheon-feel` only).
152. **Values nest at most 500 levels within an evaluation.** Nothing bounded how deeply values
    nest, and every consumer of a value (its drop, display, `string`, `=`, the response JSON, the
    parser's index of an input context) walks it recursively, so a value deep enough ran out of
    the stack: a recursion or a loop that wraps a value 9,500 times, and `context put` with a list
    of 100,000 keys, trapped 0.5.0. A value 501 levels deep now stops the evaluation with
    `Abort::ValueDepth` ("values are nested too deeply (more than 500 levels); evaluation
    stopped"), an `evaluation` error, or a `context` error for a FEEL-format input context
    (CONTRACT §1 to §3). Every consumer on every host walks at least 1,193 levels (measured on
    0.5.0), twice the limit and more. The depth is that of item 151, and the limit is read where an
    expression produces a value, by `rheon_feel::diagnostics::bound_depth`: the result of every
    invocation (`guarded`, so every built-in and user function, FEEL and boxed), a list, range and
    context literal (each entry, and the result), the results of a `for` after each item (their
    growth through `partial`; `bound_list_depth` reads the depth the list keeps), a function definition (its closure context), a filter of a value
    that is no list (the list that holds it is a level deeper), and in `rheon-model-evaluator` the
    result of every boxed expression (`build_coerced_result_evaluator`, which a boxed list, context
    or relation passes) and each entry of a boxed context after its coercion (a collection type
    puts a value that is no list into one). It is not read where a value is bound: a scope frame
    is a context, and a frame that binds a value 500 levels deep to a parameter, a loop variable,
    `item` or `partial` is 501 levels deep, which is allowed. `context put` with a key list
    refuses a list of more keys than the value may still nest (`FeelContext::apply_entries`)
    before it builds a context, so 100,000 keys stop at once; a key list whose path is blocked by
    an entry that is not a context builds nothing and gives null whatever its length, as on 0.5.0
    (the existing part of the path is read first, without recursion). Inside an evaluation the null that
    takes the place of the deeper value is never observed, because the abort is the outcome;
    outside one it carries the abort's text. Test first: `feel-evaluator/tests/value_depth.rs`
    checks each channel at 500 levels (the depth of the answer) and at 501 (the abort, and a clean
    next evaluation), a 500-level value bound and returned through each kind of frame, and loops
    and a key list that would nest 100,000 levels; on the parent the 501 cases answer values 501
    and 502 levels deep. `model-evaluator/src/tests/various/dmn_0009` does the same for a boxed list,
    a boxed context and a boxed context entry of a collection type, and fails without the boxed
    checks; the contract test asserts both error kinds with their texts. Answers change, each
    registered with both answers: `tools/identity/requests/value-depth.json` (46 requests, every
    channel at 500 and 501 levels, the 500 ones and two blocked key paths identical) and the two `context put` requests with
    100,000 keys in `no-trap.json` in `tools/identity/registered/value-depth.json` (19 entries);
    the four requests of `tools/identity/release/requests/value-depth-deep.json` (also in
    `no-trap.json`), which trapped 0.5.0 after 0.6 to 1.3 s, and the two loops of
    `memory-exhausting.json` that nest a list and a context 100,000 levels deep, which trapped it
    after 158 s and 35 s, in `tools/identity/release/registered/value-depth.json`. Basis: an
    implementation limit, reported under DMN 1.6 §7.3.8 (the evaluation "MUST report or log
    diagnostic information"), at the limit the owner approved. No committed request of an earlier
    source and no official TCK test nests a value beyond 500 levels: their answers are unchanged.
    Measured (wasm, Node 26, Apple M2) on the module of this commit and the next against 0.5.0, two
    alternating rounds: geomean 0.996x and 0.988x over the 39 workloads of the speed suite, 0.997x
    and 1.000x over the 86 parse workloads; below 0.97x in both rounds `loop-count-1k` (0.964x,
    0.959x), `bif-loop-10k` (0.963x, 0.952x) and `fib15` (0.955x, 0.939x), the cost of items 149
    to 152 together (about 1 to 2 % each on a tight loop, measured per item: moving `partial`
    out through `assign_entry`, the list type, the depth bookkeeping, and the check of every call's
    result); reading the depth of a loop's results by value instead of from the list cost
    `count(for i in 1..10000 return i)` 8 %, and this commit reads the field the list keeps.
    Allocations and reallocations per call are those of 0.5.0 for all 39 suite and 86 parse
    workloads; the peak of live bytes grows by 8 to 2,048 B (a context is 8 bytes larger natively
    for its remembered depth; `list-filter-count` 22,459,853 to 22,459,885 B). The string doubling's
    peak memory (n = 17 to 26), 2,000 calls (0 B) and the stack report are those of 0.5.0. The
    walkers of a value built natively (drop, display, `string`, JSON, the response's type,
    `=`, `instance of`, `flatten`, `...`, the parser's index) reach at least 1,265 levels on every
    host, as on 0.5.0.
    Rust API: the added variant `rheon_feel::diagnostics::Abort::ValueDepth` and the added functions
    `rheon_feel::diagnostics::bound_depth` and `bound_list_depth`; `FeelContext::apply_entries` returns an error (and stops
    a running evaluation) for a path that would nest the value more than 500 levels deep.
153. **The stack check nests a value to the depth limit.** `npm run stack` evaluated five
    constructs nested to the AST depth limit, but no value nested to the value depth limit of item
    152, so a change that makes the frames of a walk over a value larger could take a host's stack
    below 500 levels unseen. `scripts/stack-headroom.mjs` now also evaluates on every host it
    checks a value nested 500 levels deep: a `for` builds it through `partial` (its results 500
    levels deep, `partial` held in a scope frame one level deeper), and it is passed to a function,
    compared with itself, returned, rendered and dropped. On this module it passes on Node's main
    thread and in a 4 MiB worker with Liftoff and with TurboFan and in Wasmtime with a 4 MiB wasm
    stack (7.0 s for the whole check), as on 0.5.0; with the depth set to 501 every host reports the
    construct as failed, where the limit stops the evaluation. Tooling; no API change.
154. **Lists share their items.** A list held its items in a vector of its own, so every copy of
    a list (a name read, an argument passed, an item taken out of a list, `partial` in a loop)
    copied all its items and theirs: `count(items)` over 10,000 contexts read by name copied the
    10,000 contexts, and a list that a loop doubles 30 times through `partial`
    (`if i = 1 then [1] else [partial[-1], partial[-1]]`) held 2^29 copies of its first item and
    exhausted the module's memory. `FeelList` now holds its items behind a reference count: a copy
    shares them, the empty list allocates nothing, and the items are never changed while they are
    shared (DMN 1.6 §10.3.2.5: "Lists are immutable and may be nested"). `push` appends in place
    when the list holds its items alone and to a copy of them otherwise; `into_vec` moves the
    items out of a list that holds them alone and clones them otherwise (a shallow copy: each
    item clone shares what the item holds); taking the items by value moves them out or clones
    them one at a time as they are taken, so a loop over a list read by name copies no vector of
    items first, and a loop domain that is a single value is held as such. The depth of item 151
    is kept as before. Tests first: `feel/src/tests/lists_shared.rs` (a copy shares the items; a
    push, `into_vec` and taking the items of a copy leave the other copy unchanged; the items of a
    list that holds them alone are moved out; the empty list; a list of contexts of strings copied
    into 8 threads and changed in each leaves the original unchanged), failing on the parent where
    a copy does not share; and in `feel-evaluator/tests/allocations.rs`, `count(items)` and
    `{f: function(l) count(l), r: f(items)}.r` over 10,000 numbers and 10,000 contexts read by
    name cost at most 8 allocations and under 4 KiB more than over one item (0 allocations and
    0 B more; on the parent 639,936 B more for `count(items)` over numbers, the copy of the
    items). Answers change: 7 requests of
    `tools/identity/release/requests/memory-exhausting.json`, which exhausted the memory of 0.5.0
    after 1.2 to 1.7 s, now answer (`tools/identity/release/registered/shared-lists.json`):
    `count` of the doubled loop (30, in 8 ms), `=` of two doubled lists and of one with itself
    (true), `instance of list<Any>` (true), `distinct values` (1 item), `index of` ([1, 2]) and
    `union` (1 item). On this module the last six take 5.1 to 21.7 s each, because equality and
    the type of a list walk a shared item again on every path that reaches it (2^28 to 2^29
    visits). Rendering the doubled list, `string` and `flatten` of it, whose results are
    exponential in size, and the 5 requests that double a context still exhaust the memory. No
    other committed request and no official TCK test changes its answer. Measured (wasm, Node
    26, Apple M2) against the module of item 153, 2 rounds: `count(items)` over 10,000 contexts
    1.197x (8.1 to 6.8 ms), over 10,000 numbers 1.082x, `{f: function(l) count(l), r: f(items)}.r`
    1.430x, `count(for x in items return x.amount)` 1.015x, `count(for i in 1..10000 return i)`
    1.019x, `sum(items[amount > 100].amount)` 1.036x, `count(items[status = "open"])` 1.023x.
    Rust API (`rheon-feel`): `FeelList`'s `IntoIterator` by value yields the added
    `rheon_feel::values::ListItems` instead of `std::vec::IntoIter<Value>`; `FeelList` and
    `ListItems` are `Send + Sync`.
155. **Equality compares each pair of shared lists once.** FEEL equality (`=`, `!=`, `in`, and the
    list built-ins that compare items) walked both values along every path, so after item 154 a
    value that holds the same list on many paths took time exponential in its size: `x = x` for
    a list doubled 30 times through `partial` took 10.9 s, 2^29 comparisons of its first item,
    and doubled 60 times it would not end. The walk (`feel-evaluator/src/equality.rs`, moved out
    of `builders.rs` unchanged otherwise) now remembers, for one comparison, the answer for each
    pair of lists it compared where either list shares its items (`FeelList::is_shared`), keyed
    by the addresses of their items (`FeelList::address`), and answers such a pair again from
    memory. The memory is created when the first shared pair is met and dropped with the
    comparison; it is never iterated. It is exact: a pair is reached again only through two paths
    that meet in a list held twice, which is shared, and the borrowed values keep every address
    in use for the whole comparison; the two values compared are never looked up, and a list
    that holds its items alone costs one read of its reference count. There is no shortcut for a
    list compared with itself, because equality is not reflexive (a list of functions is not
    equal to itself). Tests first, in `feel-evaluator/src/tests/shared_values.rs`: lists doubled
    60 times compared with themselves, with a list doubled alike, and with one that differs in
    its last item, and lists of functions doubled 60 times (false, as before) answer at once,
    while on the parent the test was still running when it was stopped after 180 s; and equality
    on 200 random sets of values that share their lists answers as on the same values with
    nothing shared. Identity step: no answer changes. On this module the requests of
    `release/registered/shared-lists.json` with `=`, `distinct values`, `index of` and `union`
    take 7 to 9 ms (5.1 to 14.4 s before); `instance of` still takes 21.8 s. Rust API
    (`rheon-feel`): the added `FeelList::address` and `FeelList::is_shared`.
156. **The list built-ins that compare items share one equality walk per call.** `distinct
    values`, `index of`, `list contains`, `union`, `intersection` and `in` with a list compare
    many pairs in one call, each with an equality walk of its own (item 155), so a pair of shared
    lists below the items was compared again in every comparison that reached it: over 2,000
    copies of a list doubled 60 times, each comparison walked the 59 levels again. Each call now
    passes one walk through all its comparisons, so such a pair is compared once per call. The
    items and the match themselves are compared as the two values of a comparison are, without
    a lookup, so a scan over lists that are not shared below their items creates no memory. Test:
    in `feel-evaluator/src/tests/shared_values.rs`, these built-ins and `in` answer on 100 random
    sets of values that share their lists as on the same values with nothing shared. Identity
    step: no answer changes. Measured (wasm, Node 26, Apple M2, 2 rounds) against the module of
    item 155, with `d` a list doubled 60 times: `count(distinct values(for i in 1..2000 return d))`
    11.1 to 0.53 ms, `count(index of(l, d))` over 2,000 copies of `d` 11.1 to 0.54 ms,
    `count(union(l, l))` 21.7 to 0.60 ms; scans over lists that share nothing below their items
    unchanged (`count(distinct values(for i in 1..2000 return [i, i]))` 26.5 and 25.9 ms,
    `index of` and `in` over 2,000 one-item lists 0.34 and 0.31 ms, `list contains` over 5,000
    numbers 0.56 ms on both). No API change.
157. **`instance of` is decided on the value, without building its type.** `instance of` built the
    type of a list, a context, a range or a function (`Value::type_of`) and related it to the
    type asked for. The type of a list is a list of the type of its first item when every other
    item's type conforms to it, so building it typed each item, and a value that holds the same
    list on many paths was typed once per path: `instance of list<Any>` over a list doubled 30
    times through `partial` took 21.8 s after item 154, and doubled 60 times it would not end.
    `Value::has_type`, `Value::has_type_of` and `Value::type_instance_of`
    (`feel/src/values/type_relations.rs`) answer what `type_of() == t`,
    `type_of() == other.type_of()` and `type_of().instance_of(t)` answer, reading the value only
    as far as the relation needs (a list against `list<Any>` is not read at all, a context only
    in the entries of the type asked for). Within one relation, whether the items of a list
    conform to its first is decided once for a list that nests more than two levels (a shallower
    list costs a pass over its items when it is reached again), and the answer for a pair of
    nodes is remembered when either is a list that shares its items, keyed by the addresses of
    the borrowed nodes; the memory lives for the relation, is created on first use and is never
    iterated. `instance of` uses them for lists, contexts, ranges and functions and for a right
    operand that is not a type; Table 62 is not changed (the answers are those of the built
    types). Tests first: `feel/src/tests/type_relations.rs` compares the three relations with
    the built types for 300 random sets of values of every kind that share their lists (scalars,
    lists, contexts, ranges, functions, types, interval ends, unary tests) against the types of
    those values and random types, and relates lists doubled 60 times, one of them with a
    different last item, to their types at once; in `feel-evaluator/src/tests/shared_values.rs`,
    `instance of` over a list doubled 60 times answers at once, while on the parent that test was
    still running when it was stopped after 180 s. Identity step: no answer changes. The request
    `instance of list<Any>` of `release/registered/shared-lists.json` takes 7.6 ms (21.8 s
    before). Measured (wasm, Node 26, Apple M2, 2 rounds) against the module of item 156:
    `instance of` over lists that share nothing is slower, `l instance of list<list<number>>` for
    100 lists of two numbers 3.38 to 6.44 ms per 1,000 (1.9x the time; 11.0 ms when the items of
    every list were decided once through the memory, which is now kept for deeper lists only),
    with items of two kinds 3.78 to 5.89 ms; `instance of` over a context 4.83 to 4.79 ms per
    10,000, a range 2.22 to 2.28 ms, a number 1.24 ms on both. Rust API (`rheon-feel`): the added
    `Value::has_type`, `Value::has_type_of` and `Value::type_instance_of`.
    Measured on the module of items 154 to 157 together (wasm, Node 26, Apple M2, two
    alternating rounds): against the module of item 153, geomean 1.012x and 1.016x over the 39
    workloads of the speed suite, none below 0.97x in either round; `count(items)` over 10,000
    contexts 1.194x and 1.188x, over 10,000 numbers 1.062x and 1.070x,
    `{f: function(l) count(l), r: f(items)}.r` 1.434x; against 0.5.0, geomean 1.009x and 1.006x
    over the suite and 0.998x and 0.992x over the 86 parse workloads; below 0.97x in both rounds
    `count(for i in 1..10000 return abs(i))` (0.952x, 0.965x) and the 100 by 100 nested loop
    (0.965x, 0.947x), against item 153's module at least 0.990x and 0.977x, and of the parse set a
    list literal nested 297 levels (0.924x; 0.955x of item 153's module, one allocation more per
    list) and a flat context literal of 5,000 entries (0.964x, 0.970x; 1.00x of item 153's
    module). Allocations per call (native): one more per non-empty list built (the box that holds
    its items), fewer where a list was copied (`count(items)` over 10,000 contexts 210,105 to
    160,105, `{f: function(l) count(l), r: f(items)}.r` 260,207 to 160,206); the native peak of
    `count(items[status = "open"])` 22,459,853 B on 0.5.0, 22,459,925 B here (one box more than
    item 153's module); `count(items)` over 10,000 contexts 22,447,220 to 12,191,876 B. The string
    doubling's peak memory (n = 17 to 26, 267.38 MiB at 26) and the stack report are those of
    0.5.0. The walkers of a value built natively reach at least 1,265 levels on every host, as
    on 0.5.0 (equality at least 2,140 levels, 3,750 before; dropping a list at least 4,609, 6,656
    before); `context put` refuses a key list beyond 500 levels by design. `run.sh --release`:
    7,886 requests, 34 registered differences, 0 others, 529 s, peak RSS 3.1 GB.
158. **The parameters and the result type of a function definition are read through accessors.**
    `FunctionDefinition` exposed its formal parameters and its return type as public fields, so any
    holder of a function definition could change what an invocation binds and coerces to. They are
    now private and read through `parameters()` (the names with their types, in order) and
    `result_type()`; the invocation, `sort`, the callback check of `list replace`, the business
    knowledge model's inputs, `instance of` and the display read them there. `closure` and
    `closure_ctx` stay public: a function returned by a body is closed over by writing them.
    Identity step: no answer changes. Rust API (`rheon-feel`): the fields
    `FunctionDefinition::parameters` and `FunctionDefinition::result_type` are private; read them
    with the added `FunctionDefinition::parameters()` (`&[(Name, FeelType)]`) and
    `FunctionDefinition::result_type()` (`&FeelType`); build a definition with
    `FunctionDefinition::new` as before.

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

Commits 85-133 (2026-11-15), on the tree of commit 133: workspace 8,724 passed, 0 failed, 24
ignored (CI's skips), the same with `RHEON_FULL_TESTS=1`, and the README's full-size run
(`RHEON_FULL_TESTS=1`, release, the four crates) 989 passed; clippy on the workspace without
warnings; the API documentation of the ten published crates builds with rustdoc's warnings denied
(item 133); feel.wasm tests 79 native (8 unit, 71 contract) and 80 under Wasmtime; runtime 33;
smoke passes, with its two memory checks growing by 16.6 MiB and 26.5 MiB (bound 32 MiB); the stack
check passes, and its report's recursion depth is at least that of the start of these commits on
every host: Node's main thread 22 with Liftoff and 24 with TurboFan (start 18 and 14, 0.4.0 17 and
14), workers of 2, 3 and 4 MiB 46, 75 and 104 with Liftoff (40, 64, 89) and 51, 83 and 114 with
TurboFan (31, 50, 69), Wasmtime with 512 KiB, 1, 2, 4 and 16 MiB 10, 26, 57, 120 and 289 (10, 25,
57, 98, 98); the playground builds. The number oracle (item 90) judges every case right with no
exemption: 35,939 cases quick and 2,896,591 at full size, in 188 families. DMN TCK via
`scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test result identical
to 0.4.0's. Full responses against the 0.4.0 release module, each request sent as written: 194,079
requests (the committed sources, each evaluate request also as parse, the ledger, and the request
sets kept outside the repository), 2,789 registered differences reproduced (1,144 of them from
commits 85-133, counted per change in their items), none unregistered, and every cached reference
answer equal to a live one (`--verify-cache`). 52 of the requests trap 0.4.0: 30 of them answer now
and are registered; the other 22 trap the build as well, as they did at the start of these commits:
loops that double a shared list 30 times (`for i in 1..30 return ... [partial[-1], partial[-1]]`),
which exhaust the module's memory. Against the module of commit 129 the same requests differ only
in the 51 registered answers of items 130-132. Speed against 0.4.0 (engine benchmarks, 24
workloads): geomean 1.02x, every result equal, from 0.92x (`fib15`, 572 to 619 µs) to 1.10x
(`loop-count-1k`); against the start of these commits 1.03x, from 0.98x to 1.08x; 11 regex
workloads against 0.4.0: geomean 1.03x, from 1.00x to 1.23x, every result equal except the one
whose replacement string the F&O rules read differently. The publish dry run needs the crates.io
index and was not run.

Commits 55-84 (2026-11-14), on the tree of commit 84: workspace 8,639 passed, 0 failed, 24 ignored
(CI's skips); clippy without warnings; feel.wasm tests 69 native (8 unit, 61 contract) and 70 under
Wasmtime (8, 62); runtime 33; smoke passes, with its two memory checks growing by 22.1 MiB and 26.5
MiB (bound 32 MiB); the stack check passes, and its report reaches at least 0.4.0's depth on every
host (item 67); the playground builds; the randomized and exhaustive tests at full size
(`RHEON_FULL_TESTS=1`, release, with the parser corpus, now with `rheon-feel-wasm`) 971 passed. DMN
TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test result
identical to 0.4.0's. Full responses against the 0.4.0 release module, each request sent as
written: 187,677 requests (the committed sources, each evaluate request also as parse, the ledger,
and request sets kept outside the repository: the fraction sweep, the duration, message and regex
probes and the performance workloads), 1,645 registered differences reproduced (counted per change
in items 59-83), none unregistered; 52 of the requests trap 0.4.0: 30 of them answer in the build,
and the other 22, loops that double a shared list 30 times, exhaust the module's memory there as
well. The full-size edge-operand product of item 84 (28,110 requests with its parse twins), which
is not an identity source, differs in 568 answers: its 24 answer changes that are not message
changes are among the registered ones, and its other 544 answers differ only in null reasons that
quote a number in scientific notation, instances of the registered message rule (item 68) that are
not listed one by one. Speed against 0.4.0 (engine benchmarks, 24 workloads): geomean 1.01x, every
result equal, from 0.94x to 1.11x.

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

The API changes of commits 85-133 (the item of each change is in parentheses):

- Changed:
  - Every crate that forwarded a number backend: the Cargo features `intel` and `dashu` (and
    `default`) of `rheon-feel-number`, `rheon-feel`, `rheon-feel-temporal`, `rheon-feel-parser`,
    `rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator` are gone, so a dependency
    written with `features = ["dashu"]` or `features = ["intel"]` no longer resolves;
    `default-features = false` is harmless (95).
  - `rheon-feel-number`: `FeelNumber`'s `Display` (and so `to_string()` and `Jsonify`) prints the
    canonical text and ignores a precision, and `FeelNumber` is 32 bytes (97). Behaviour: a zero
    has no sign (96); `Debug` of a NaN is `NaN` (98); `new(n, s)` with `s` beyond the exponent's
    range is the rounded value or infinite (102); `pow` with an integer
    exponent is correctly rounded, and `Debug` of an integer power shows the cohort of the exact
    power (104).
  - `rheon-feel-temporal`: `FeelZone::Offset` holds a `FeelOffset` (was `i32`), and its `Display`
    changes for negative offsets below one hour (106); `FeelTime::offset_opt` and
    `FeelDateTime::offset` accept offsets within ±14:00 (were -11:59..+12:00) (107);
    `FeelYearsAndMonthsDuration::years` and `months` return `u64` magnitudes (were signed `i64`)
    (112). Behaviour: `FeelTime`, `FeelDateTime` and `FeelZone::from_designator` reject an offset
    with minutes of 60 or more (108) or written with seconds (109); `&FeelDate - &FeelDate` gives
    new results for negative spans inside one year (113).
  - `rheon-feel`: the module form of `value_null!` takes the function name as any expression,
    not only as a literal (101); `diagnostics::record` and `values::null_with_reason` take a
    `Severity` first, and `Diagnostics::count` and `messages` leave out notes (117).
  - `rheon-feel-regex`: `is_match` takes the flags (`""` for none) and `replace` takes
    `flags: &str` instead of `Option<&str>`; `RegexError::Invalid` is renamed `InvalidPattern`
    (130). Behaviour: invalid flags (130), an invalid replacement string (131) and a pattern of
    `replace` or `split` that matches the zero-length string (132) are
    `RegexError::InvalidArgument`; `split` of `""` has no item (132).
  - `rheon-model-evaluator`, behaviour: `evaluate_invocable_with_mode` counts no error for a
    decision table without a matching rule (119) or a missing input variable of an
    item-definition type (120), and in strict mode returns their value; an input whose item
    definition references another with allowed values is checked against them (126), and an
    input typed by a collection with allowed values is checked per element (127).
- Removed:
  - `rheon-feel-number`: `FeelNumber::ceiling`, `floor`, `round_up`, `round_down`,
    `round_half_up`, `round_half_down` and `round` (use `round_to`) (101); `impl Rem` and
    `impl RemAssign` for `FeelNumber` (use `FeelNumber::modulo`) (103).
  - `rheon-feel-temporal`: `TryFrom<i32> for FeelZone` (`From<FeelOffset>` replaces it) (107).
  - `rheon-feel`: `Value::Transparent` (124).
  - `rheon-feel-evaluator`: `ForExpressionEvaluator`, `SomeExpressionEvaluator` and
    `EveryExpressionEvaluator` (128).
  - `rheon-feel-regex`: `is_match_with_flags` (130).
- Added:
  - `rheon-feel-number`: `FeelNumber::round_to` and `RoundingMode` (100, public with their callers
    in 101), `FeelNumber::modulo` (103).
  - `rheon-feel-temporal`: `FeelOffset` with `ZERO` and `Display` (106), `FeelOffset::from_duration`
    and `From<FeelOffset> for FeelZone` (107), `FeelOffset::write_seconds` (110);
    `FeelDaysAndTimeDuration::get_nanoseconds` and `FeelDateTime::nanos` (111).
  - `rheon-feel`: `diagnostics::Severity` and the `note:` forms of `value_null!` (117).
  - `rheon-feel-evaluator`: `Iteration` (`boxed`, `collect`, `some`, `every`), `Sequence` (`of`,
    `empty`, `interval`) and `Domain` (128).
  - `rheon-feel-regex`: `RegexError::InvalidArgument` (130).
- Kept: `rheon-feel-grammar` and `rheon-feel-wasm` have no API change, and `rheon-feel-parser`
  and `rheon-model` change only by the removed features (95). The wasm module's ABI and its
  request and response JSON are unchanged; what changes there are answers, registered with both
  answers as "Verification" states.

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
