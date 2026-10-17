# The `wasm` branch: a thin fork of dsntk-rs for dsntk-wasm

This repository is a **local-only** fork of <https://github.com/dsntk/dsntk-rs>
(the DecisionToolkit FEEL/DMN engine). It is the engine behind
`dsntk-wasm` (`~/dev/dsntk-wasm`), which compiles the FEEL crates
to WebAssembly for a browser playground and other hosts.

Rules for this clone:

- `main` tracks `origin/main` and is never modified here.
- `wasm` is the working branch. All fork changes live on it, as small,
  upstream-shaped commits.
- `fastnum-backend` is a side branch off `wasm` with the experimental fastnum
  number backend (see below); it is not needed for wasm.
- Never push, never create anything on GitHub, never `cargo publish`.

## Base

- Upstream commit: `c76aee02187183b3659a9051f3ebb283630066f7`
  ("Prepared next development version.", workspace version `0.3.1-dev`,
  `git describe`: `v0.1.0-47-gc76aee0`).
- This is the commit recorded in `dsntk-wasm/patches/dsntk/UPSTREAM_COMMIT`.
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
| 5 | `8c56f2c83c5fb28d3cdd5d8faa10444ffa4d4996` | Add WASM-FORK.md | fork only | no |
| 6 | `ab6339c` | feel-number: shared decimal128 core for pure-Rust backends | dashu spike | maybe (see below) |
| 7 | `e0df83b` | feel-number: dashu-float backend, feature `dashu` (default) \| `intel` | dashu spike | maybe |
| 8 | `64fca91` | Forward the number backend features through the whole workspace | fork | with 6-7 |
| 9 | `c64e330` | Tests: per-backend expectations where Intel's rounding error is encoded, dashu regressions | fork | with 6-7 |
| 10 | `786f584` | feel-number: track the sign of NaN like Intel (Debug-visible only) | fork | with 6-7 |
| 11 | `2271732` | tools: differential fuzz and FEEL corpus harnesses (Intel vs dashu) | dashu spike | no |
| 12 | `4ea7544` | feel-number: `NUMBER_BACKEND` constant naming the compiled-in backend | fork | with 6-7 |

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
12. **`NUMBER_BACKEND`.** `dsntk_feel_number::NUMBER_BACKEND` is `"dashu"` or
    `"intel"`, selected by the same feature as the backend itself; dsntk-wasm
    reports it as `numberBackend` in its `info` op.

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

Default features (`dashu`), at commit 10:

```sh
cd ~/dev/dsntk-rs
cargo build --workspace                       # no dfp-number-sys in `cargo tree`
cargo test -p dsntk-feel-number               # 21 unit + 207 integration tests
cargo test -p dsntk-feel-temporal -p dsntk-feel-parser -p dsntk-model
cargo test -p dsntk-feel-evaluator --lib -- --skip external_functions
cargo test -p dsntk-model-evaluator
cargo clippy -p dsntk-feel-number --lib --tests
```

| Crate | dashu (default) | intel (`--no-default-features --features intel`) |
|-------|-----------------|-----------------------------------------------|
| dsntk-feel-number | 228 passed, 0 failed (21 unit, 207 integration) | 207 passed, 0 failed |
| dsntk-feel-temporal | 79 passed | 79 passed |
| dsntk-feel-parser | 461 passed | 461 passed |
| dsntk-model | 45 passed | 45 passed |
| dsntk-feel-evaluator | 3371 passed, 13 failed, 3 ignored | 3370 passed, 14 failed, 3 ignored |
| dsntk-model-evaluator | 3650 passed, 19 failed, 20 ignored | 3650 passed, 19 failed, 20 ignored |

The 13 feel-evaluator failures are `tests::external_functions::*` (Java RPC
server at `127.0.0.1:22023`, same on both); the 14th on intel was
`tests::bifs::bif_now::_0001` (`now()` crossing a second boundary, flaky).
The 19 model-evaluator failures are `dmn_3_0076::*` (18, Java external
functions) and `dmn_3_0103::_0017` (`time_006`, depends on the current
Australia/Melbourne DST offset), identical on both backends; before commit 9
dashu also failed `dmn_3_0100::_0278` (`5 ** 2.55`, Intel's 1-ulp value).

The Intel runs need the patched `dfp-number-sys` from dsntk-wasm (the
crates.io version has FFI signature bugs), injected without committing a
machine-specific path:

```sh
cargo test --no-default-features --features intel \
  --config 'patch.crates-io.dfp-number-sys.path="~/dev/dsntk-wasm/third_party/dfp-number-sys"' \
  -p dsntk-feel-number
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
`cargo test -p dsntk-feel-number --no-default-features --features fastnum`
228 passed, clippy clean, `cargo check --workspace --no-default-features
--features fastnum` succeeds. `dashu` stays the default.

## Rebasing onto upstream

```sh
cd ~/dev/dsntk-rs
git fetch origin
git checkout main && git merge --ff-only origin/main     # keep main a mirror
git checkout wasm
git rebase main                                         # replay the commits
git rebase wasm fastnum-backend                         # optional side branch
```

Resolve conflicts per commit (each one is small and self-contained; commit 3
touches the most files, commit 8 touches every manifest). Then:

1. Rerun the verification above (suites, `tools/difftest/run.sh`,
   `tools/evaluator-corpus/run.sh`).
2. Point dsntk-wasm at the new head: it depends on this clone as a git
   dependency (`git = "file://$HOME/dev/dsntk-rs"`, pinned with
   `rev = "<sha>"` in `[workspace.dependencies]` of `dsntk-wasm/Cargo.toml`,
   plus `[workspace.metadata.dsntk] upstream-commit`). Replace the SHA (and
   the upstream base), then `cargo update` there and run its `npm test`,
   `npm run build:wasm` and `npm run smoke`. `git log main..wasm` is the
   source of truth; dsntk-wasm no longer carries patches.
3. In dsntk-wasm run `./scripts/bootstrap.sh --force` (or point the path
   dependencies here) and `npm test`.

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
