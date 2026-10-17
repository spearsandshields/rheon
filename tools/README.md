# Differential test tools for the number backends

Two standalone Cargo projects (each has its own `[workspace]`, so they are not members of the
dsntk workspace) that compare the `intel` (dfp-number-sys, Intel BID128) and `dashu`
(pure Rust, the default) implementations of `dsntk_feel_number::FeelNumber` against each other
and against an independent oracle. They were used to validate the dashu backend; run them again
after touching `feel-number/src/decimal128.rs` or `feel-number/src/backend_dashu.rs`.

Both need the patched `dfp-number-sys` from dsntk-wasm for the Intel side (the crates.io
version has FFI signature bugs on some targets); the run scripts inject it with
`--config patch.crates-io.dfp-number-sys.path=...` and take its location from
`DFP_NUMBER_SYS` (default `../../../dsntk-wasm/third_party/dfp-number-sys`, i.e. a sibling
checkout of dsntk-wasm). The oracles need `python3` with `mpmath`.

## `difftest/`: op-by-op fuzz of `FeelNumber`

`crates/fn-intel` and `crates/fn-dashu` compile `feel-number/src/lib.rs` twice (feature `intel`
and feature `dashu`) so that one binary holds both backends. For every operation of the
`FeelNumber` API (parse, `+ - * / %`, assign ops, neg, abs, trunc, frac, sqrt, square, exp, ln,
FEEL `log`, pow in four flavours, FEEL modulo, quantize, FEEL `decimal`, ceiling, floor,
round down/up/half up/half down with scales, comparisons, predicates, formatting, the ten
integer conversions, `new`, `From<int>`, constants, and random op chains) it generates
`iters_per_op` random cases from a fixed seed (`DIFFTEST_SEED` changes it), feeds each to both
backends and compares `Display` (and `Debug`, i.e. the cohort, as a secondary signal).

```sh
tools/difftest/run.sh [iters_per_op] [only_op,...]       # default 200000 per op
tools/difftest/target/release/difftest eval pow 12.2384283 4.25   # one case, both backends
```

`run.sh` writes `runs/<timestamp>/mismatches.tsv`, `cohort.tsv`, `run.txt` and `oracle.txt`.
`judge4.py` / `oracle.py` judge every `Display` mismatch: Python `decimal` with an IEEE
decimal128 context for arithmetic, sqrt, exp, ln and quantize, mpmath (150 digits, rounded once)
for pow, and dsntk's own scale algorithm in exact arithmetic for the scale-based rounding
functions; chains are judged at their first divergent step. A healthy run has no
`INTEL correct, DASHU WRONG` bucket. Expected classes of mismatch:

- `dashu correct, intel wrong (<=1 ulp | 2-9 ulp | >=10 ulp)` on exp, ln, FEEL log, pow with
  a non-integer exponent or an exponent outside `i32`, and chains through them: Intel
  evaluates these in binary128 and is not correctly rounded (11-16% of random inputs).
- `dashu correct, intel wrong (special: Inf vs truth finite)` on scale-based rounding and
  `new` near the overflow edge: Intel's `bid128_scalbn` pads the coefficient only to about
  33 digits before declaring overflow; the core pads to 34.
- `both correct value (cohort/format only)` on the scale-based rounding functions when
  `ilogb + scale` leaves `i32` (a zero with a negative scale, the `i32` scale extremes): Intel
  panics on the overflow (overflow checks are on in the tool), the core reports the invalid
  scale; in FEEL both are null, the reason text differs in the quoted number.
- `no oracle` for chain steps whose operands were not recorded, and for `round_quantize`
  with a quantum outside the exponent range (Intel panics in `bid128_scalbn`, the core gives
  NaN, i.e. null).
- `cohort.tsv` (Debug-only differences, Display identical) on exp/ln results that are
  integers or short dyadic fractions within a binary128 rounding error of the threshold
  (about 1.5e-5 of exp/ln evaluations).

## `evaluator-corpus/`: FEEL expressions through the evaluator

`feel-eval` reads JSON lines `{"src","ctx","expr"}`, evaluates each expression with
`dsntk-feel-evaluator` built with the selected backend, and writes `{"display","debug","ns"}`.
`run.sh` builds it for `intel` and `dashu`, runs the four corpora in `corpora/` through both,
and compares (`compare.py`: Display and Debug mismatches, panics, time) and judges
(`oracle_feel.py`: mpmath with every intermediate rounded to decimal128):

| corpus | expressions | origin |
|---|---|---|
| `corpus.jsonl` | 3313 | harvested from the feel-evaluator unit tests and `bbt/` (`harvest.py <dsntk checkout>`) |
| `gen.jsonl` | 8636 | generated numeric edge cases (`gen.py`) |
| `extra.jsonl` | 1090 | hand-written numeric expressions around exp/log/sqrt/`**`/`/` |
| `dmn.jsonl` | 4178 | literal expressions of the DMN example models (`harvest_dmn.py <dsntk checkout>`) |

```sh
tools/evaluator-corpus/run.sh            # all four corpora
tools/evaluator-corpus/run.sh gen dmn    # a subset
```

Expected: the only Display mismatches are exp/log/`**` results where dashu is correctly rounded
and Intel is 1 ulp off (plus `now()`/`today()`), identical panic counts (`Display` of an
infinite intermediate value in `string(...)` on both), and a couple of Debug-only cohort
differences.
