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

- Engine changes are small, self-contained commits. The "PR candidate" column
  below records which ones would be upstreamable; offering them is not a
  priority (decision of 2026-10-28).
- `fastnum-backend` is a side branch with the experimental fastnum number
  backend (see below); it is not needed for wasm.
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

## Commits on `wasm` (oldest first)

| # | SHA | Title | Origin | PR candidate |
|---|-----|-------|--------|--------------|
| 1 | `e1fc50dc2a3fd9b941712ee0e76c01a51bf74b99` | Build on wasm32: gate Java external functions, 32-bit isize/usize conversions | `0001-wasm32-support.patch` | yes |
| 2 | `b1a04ca6aaec7a93d57ca4141587e7f2c0b5eb4f` | feel-temporal: process-wide local UTC offset override for hosts without a time zone | `0002-local-offset-override.patch` | yes |
| 3 | `ec2ced32fff2b0fdc72a8173385774cbb654a567` | Robustness: no user-reachable panics, non-finite numbers are null, call depth limit, iterative AST drop | `0003-robustness.patch` | yes (split, see below) |
| 4 | `55948221d3e11df370366696d3d52431820d7c9f` | Linear `for` loops and list literal parsing (were quadratic) | `0004-linear-for-and-list-parsing.patch` | yes (overlaps upstream PR #112) |
| 5 | `8c56f2c83c5fb28d3cdd5d8faa10444ffa4d4996` | Add docs/FORK.md | fork only | no |
| 6 | `ab6339c` | feel-number: shared decimal128 core for pure-Rust backends | dashu spike | maybe (see below) |
| 7 | `e0df83b` | feel-number: dashu-float backend, feature `dashu` (default) \| `intel` | dashu spike | maybe |
| 8 | `64fca91` | Forward the number backend features through the whole workspace | fork | with 6-7 |
| 9 | `c64e330` | Tests: per-backend expectations where Intel's rounding error is encoded, dashu regressions | fork | with 6-7 |
| 10 | `786f584` | feel-number: track the sign of NaN like Intel (Debug-visible only) | fork | with 6-7 |
| 11 | `2271732` | tools: differential fuzz and FEEL corpus harnesses (Intel vs dashu) | dashu spike | no |
| 12 | `4ea7544` | feel-number: `NUMBER_BACKEND` constant naming the compiled-in backend | fork | with 6-7 |
| 13 | `ba39cb1` | tools: vendor the patched dfp-number-sys used as the Intel reference | fork only | no |
| 14 | `c760d4d` | feel-number: scale rounding keeps the scale when the rounded coefficient is ±1 | upstream bug | yes |
| 15 | `55c7406` | feel-evaluator: stop a numeric `for` range when the step is lost to rounding | upstream bug | yes |
| 16 | `4d61eb1` | feel-temporal: time ± duration wraps at midnight; order times and date-times across offsets as instants | upstream bug | yes |
| 17 | `f308a34` | feel-parser: bound the name-prefix search in the lexer (path chains were cubic) | upstream bug | yes |
| 18 | `5ecdede` | feel-evaluator: readable null reasons for time/exp/log/duration; huge numbers in messages in scientific notation | fork | maybe |
| 19 | `cf21d52` | feel-temporal: `is()` on times stays zone-strict now that `=` compares instants | with 16 | yes |
| 20 | `7de788f` | model-evaluator: expectations for the new null reasons of time(), exp(), log() and duration() | with 18 | with 18 |
| 21 | `c324518` | feel-number: scale-based rounding accepts the full FEEL scale range -6111..=6176 | TCK 1141-1144 | yes |
| 22 | `44049be` | feel-regex: back-references in `matches` patterns (fancy-regex) | TCK 1111 | yes |
| 23 | `14afccf` | model: accept the DMN 1.6 namespace | DMN 1.6 | no |
| 24 | `1503770` | feel: single-argument rounding and number(from); trim, intersection, insert after (1.7 beta) | DMN 1.6 | no |
| 25 | `1c11b1e` | feel: `value` property of temporal values; filters on a non-list; number() text must be a numeric literal; tck-addendum | DMN 1.6 | no |
| 26 | `dac2c47` | feel: truncate fractional list positions before the sign check (remove panicked) | upstream bug | yes |
| 27 | `a46200f` | feel: date and time(date, time, timezone) | DMN 1.6 | no |
| 28 | `39d6ed9` | model: load ONNX functions; compatibility tests follow the 1.6 changes | DMN 1.6 | no |
| 29 | `2606bc5` | feel-grammar back in the tree as rheon-feel-grammar (tables reproduce byte for byte) | fork | no |
| 30 | `9605874` | feel: descendant operator `e...name` (grammar, lexer, AST, evaluator) | DMN 1.6 | no |
| 31 | `f0487fd` | model: implicit conversions, TCK 1157 12/12 | TCK 1157 | no |
| 32 | `b911ac4` | model: multiple imports (per-model import names), TCK 1160 4/4; boxed for over a single value | TCK 1160 | no |
| 33 | `9ac3594` | feel: B-FEEL; all()/any() follow the deciding item | DMN 1.6 | no |
| 34 | `624c414` | feel: diagnostics and lenient/strict error modes; wasm `language`, `errorMode`, `diagnostics` | DMN 1.6 | no |

(Full SHAs: `git log --format='%H %s' main..wasm`.)

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
    than the longest key in scope (or built-in property name) are skipped;
    the fallback name is unchanged.
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

`fastnum-backend` = the head of `wasm` + one commit adding
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

## Publishing to crates.io

The ten `rheon-*` crates share the workspace version (`[workspace.package] version`,
rheon's own line; `0.1.0` was the first release, `0.2.0` added `rheon-feel-grammar`) and pin each other with `=<version>` in
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

## PR candidates for DecisionToolkit/dsntk

Commits 1-4 are written to be upstreamable (no dsntk-wasm specifics,
no behaviour change on native targets unless a panic is turned into a null):

- **Commit 1 (wasm32 support)** - send as is. Small, cfg-gated, no native
  change.
- **Commit 2 (local offset override)** - send as is. Opt-in, additive API in
  `feel-temporal`; when unset the code path is the original one.
- **Commit 3 (robustness)** - ideally split into one PR per crate
  (feel-number Display/is_finite; feel-parser lexer surrogates + iterative
  AstNode drop; feel-temporal checked durations; feel-evaluator non-finite
  -> null, call depth, 64-bit positions; feel type_of). The call depth limit
  (`MAX_CALL_DEPTH`) is the only part where upstream might prefer a
  configurable value.
- **Commit 4 (linear for/list)** - same root-cause fixes as the open upstream
  PR DecisionToolkit/dsntk#112 but without its parallel evaluator; offer it
  as the minimal alternative or as a comment on #112.

- **Commits 21-22 (scale range, back-references)** - send as is; each
  makes an unpatched DMN TCK case pass and is covered by the TCK and crate
  tests. Commit 22 adds the `fancy-regex` dependency to `feel-regex`.

Commits 6-10 (the pure-Rust number backend) are a larger proposal: they
would have to be offered with `intel` kept as upstream's default (one-line
change in `feel-number/Cargo.toml` and the forwarding manifests) and with the
per-backend test expectations explained by the oracle results above.
Commit 11 (tools) and this file are fork-only.

## Layout note

This is the full upstream workspace (CLI, server, model evaluator, ...) so
that `cargo test` of any crate works unchanged. dsntk-wasm only needs
`common`, `macros`, `feel`, `feel-number`, `feel-temporal`, `feel-regex`,
`feel-grammar`, `feel-parser` and `feel-evaluator`; with the default
features none of them needs a C toolchain any more.
