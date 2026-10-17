# The `wasm` branch: a thin fork of dsntk-rs for dsntk-wasm

This repository is a **local-only** fork of <https://github.com/dsntk/dsntk-rs>
(the DecisionToolkit FEEL/DMN engine). It is the engine behind
`dsntk-wasm` (`~/dev/dsntk-wasm`), which compiles the FEEL crates
to WebAssembly for a browser playground and other hosts.

Rules for this clone:

- `main` tracks `origin/main` and is never modified here.
- `wasm` is the working branch. All fork changes live on it, as small,
  upstream-shaped commits.
- Never push, never create anything on GitHub, never `cargo publish`.

## Base

- Upstream commit: `c76aee02187183b3659a9051f3ebb283630066f7`
  ("Prepared next development version.", workspace version `0.3.1-dev`,
  `git describe`: `v0.1.0-47-gc76aee0`).
- This is the commit recorded in `dsntk-wasm/patches/dsntk/UPSTREAM_COMMIT`.
  The four commits below are the four patches `dsntk-wasm/patches/dsntk/0001..0004`
  applied in order, one commit each. After the last commit the tree is
  byte-identical to `dsntk-wasm/third_party/dsntk` (verified with
  `diff -r`, excluding `target/`, `.git/` and `.bootstrap-stamp`).

## Commits on `wasm` (oldest first)

| # | SHA | Title | Origin | PR candidate |
|---|-----|-------|--------|--------------|
| 1 | `e1fc50dc2a3fd9b941712ee0e76c01a51bf74b99` | Build on wasm32: gate Java external functions, 32-bit isize/usize conversions | `0001-wasm32-support.patch` | yes |
| 2 | `b1a04ca6aaec7a93d57ca4141587e7f2c0b5eb4f` | feel-temporal: process-wide local UTC offset override for hosts without a time zone | `0002-local-offset-override.patch` | yes |
| 3 | `ec2ced32fff2b0fdc72a8173385774cbb654a567` | Robustness: no user-reachable panics, non-finite numbers are null, call depth limit, iterative AST drop | `0003-robustness.patch` | yes (split, see below) |
| 4 | `55948221d3e11df370366696d3d52431820d7c9f` | Linear `for` loops and list literal parsing (were quadratic) | `0004-linear-for-and-list-parsing.patch` | yes (overlaps upstream PR #112) |

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

## Verification (native, Intel backend)

The number backend at this point is still `dfp-number-sys` (Intel BID128).
The crates.io version has FFI signature bugs that dsntk-wasm patches in
`third_party/dfp-number-sys`; to get correct numbers natively the tests were
run with that patched copy injected through `--config` (nothing is committed
for it, so the fork carries no machine-specific path):

```sh
cd ~/dev/dsntk-rs
cargo test --no-fail-fast \
  --config 'patch.crates-io.dfp-number-sys.path="~/dev/dsntk-wasm/third_party/dfp-number-sys"' \
  -p dsntk-feel-number -p dsntk-feel-temporal -p dsntk-feel-parser -p dsntk-feel-evaluator --lib
cargo test --config 'patch.crates-io.dfp-number-sys.path="~/dev/dsntk-wasm/third_party/dfp-number-sys"' \
  -p dsntk-feel-number          # its tests are integration tests, not --lib
git checkout Cargo.lock          # --config patch.crates-io rewrites the lock file
```

Results with cargo 1.95.0 / rustc 1.95.0:

| Crate | Result |
|-------|--------|
| dsntk-feel-parser (`--lib`) | 453 passed, 0 failed |
| dsntk-feel-temporal (`--lib`) | 57 passed, 0 failed |
| dsntk-feel-evaluator (`--lib`) | 3357 passed, 13 failed, 3 ignored: the 13 failures are all `tests::external_functions::*`, which call a Java RPC server at `http://127.0.0.1:22023` and fail on any machine without it. With `-- --skip external_functions`: 3345 passed, 0 failed, 3 ignored, 25 filtered out. |
| dsntk-feel-number (all targets) | 200 passed, 0 failed (34 integration test binaries; 0 `--lib` tests by design) |

The same tree, as `dsntk-wasm/third_party/dsntk`, also passes dsntk-wasm's
`cargo test --target wasm32-wasip1` under wasmtime (see dsntk-wasm `npm test`).

## Rebasing onto upstream

```sh
cd ~/dev/dsntk-rs
git fetch origin
git checkout main && git merge --ff-only origin/main     # keep main a mirror
git checkout wasm
git rebase main                                         # replay the four commits
```

Resolve conflicts per commit (each one is small and self-contained; commit 3
touches the most files). Then:

1. Rerun the verification above.
2. Update `dsntk-wasm/patches/dsntk/UPSTREAM_COMMIT` to the new base and
   regenerate the patches if dsntk-wasm still consumes them:
   `git format-patch --no-stat --no-numbered -o <dir> main..wasm`, or keep
   them as `git diff <prev>..<commit>` per step as before. Once dsntk-wasm
   depends on this fork by path, the patches directory becomes a mirror of
   this branch and `git log main..wasm` is the source of truth.
3. In dsntk-wasm run `./scripts/bootstrap.sh --force` (or point the path
   dependencies here) and `npm test`.

If a commit was merged upstream, `git rebase` drops it automatically when
the patch is identical; otherwise drop it by hand with `git rebase -i`.

## PR candidates for DecisionToolkit/dsntk

All four commits are written to be upstreamable (no dsntk-wasm specifics,
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

Not PR material: nothing yet. Later fork-only work (number backend trait,
dashu-float backend, Cargo features such as `intel | fastnum | dashu`) should
go in separate commits after these four so the rebase set stays clean and
the upstream-shaped commits stay at the bottom of the branch.

## Layout note

This is the full upstream workspace (CLI, server, model evaluator, ...) so
that `cargo test` of any crate works unchanged. dsntk-wasm only needs
`common`, `macros`, `feel`, `feel-number`, `feel-temporal`, `feel-regex`,
`feel-grammar`, `feel-parser` and `feel-evaluator`.
