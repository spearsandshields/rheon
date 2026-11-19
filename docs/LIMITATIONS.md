# Known limitations

What the engine does not do, where it differs from other engines, and the limits it enforces.
Known bugs are in [BACKLOG.md](BACKLOG.md).

- **Java external functions are not supported.** They need a JVM, and on wasm32 they evaluate to
  null with a reason.
- **Rounding of `exp`, `ln`, `log` and `**`.** These are correctly rounded to 34
  digits (the exact result, rounded once, half-even). Upstream dsntk with the Intel library is
  1 ulp off for roughly 11–16 % of inputs and occasionally more, so the last digit of such results
  can differ from upstream dsntk; e.g. `12.2384283 ** 4.25` is `41959.85737359436186095331070746801`
  here (the exact value continues …7468005517…, so …801 is the correctly rounded 34-digit
  result), while Intel gives …800. The number oracle (`tools/number-oracle/`) judges every
  number result against DMN 1.6; the classes that still deviate are listed in its
  `known-wrong.json`.
- **Integer-exponent `**`.** `x ** n` with an integer `n` is the exact power rounded once while
  it has at most about 4,900 digits (its reciprocal one correctly rounded division), and the
  correctly rounded `exp(y ln x)` beyond: `1.05 ** 360` is
  `42476396.40868002039560415895318542`, and `2 ** -20000 = 0.5 ** 20000`. Upstream dsntk
  multiplies with a rounding after every product, so its last digits differ, by 294 ulp for
  `1.1 ** 100000` and about 89,000 ulp for `1.0001 ** 1000000`. A large exponent beyond the
  exact power costs about 0.4 ms per evaluation.
- **Size.** `feel.wasm` is about 4.0 MiB raw (4.2 MB), about 1.1 MiB gzip and 760 KiB brotli
  (`npm run build:wasm` prints the exact figures). The `wasm` profile builds at opt-level 3; at
  opt-level z the same code is about 3.0 MiB raw and 540 KiB brotli and runs at about half the
  speed. With the Intel C library (and opt-level z) the module was 3.7 MB / 1.6 MB / 1.3 MB.
  The build is reproducible: `scripts/build-wasm.sh` remaps `CARGO_HOME` and the workspace
  path out of the module (the only machine-specific bytes were panic-location strings), so the
  same toolchain, lock file and commit give a byte-identical `feel.wasm`
  from any directory (verified by rebuilding from a copy at another path). A plain
  `cargo build` without the script still works but embeds the builder's paths. The JS bundle is
  mostly CodeMirror.
- **Long loops** were quadratic in upstream dsntk (`for i in 1..100000` took tens of seconds);
  this repository makes them linear (`count(for i in 1..1000000 return i)` takes about 0.11 s
  in wasm). Expressions that are still too slow hit the 2 s timeout; the worker is terminated
  and restarted. In the playground a superseded evaluation is cancelled (its worker is replaced
  after a 100 ms grace period), so typing never queues up behind a runaway one.
- **Nesting and recursion limits.** Expressions nested deeper than 300 AST levels are a `syntax`
  error, and more than 200 nested function calls (e.g. runaway recursion) stop the evaluation with
  an `evaluation` error (CONTRACT §2). FEEL calls take no native stack, so 200 nested calls run
  with a body nested to the AST limit on Node's main thread, in a worker and with Wasmtime's
  default 512 KiB wasm stack; a recursion through a callback of the engine (the comparison of
  `sort`) takes native stack per level, and a host that runs out of it reports `crash` and the
  worker is replaced. Measured (`npm run stack -- --report`): 66 levels through a `sort`
  comparison with a body nested to the AST limit run on every host measured (CONTRACT §1).
- **Memory.** Linear memory is capped at 512 MiB, so a runaway allocation traps early (`crash`).
  A list element costs about 130 B while the list grows (a 64-byte value plus the spare
  capacity of the list; `count(for i in 1..1000000 return i)` grows memory by 128 MiB) and a
  one-entry context about 1 KB, so `for i in 1..2500000 return i` (after about 0.3 s) or a
  million one-entry contexts (after about 0.9 s) hit the cap before the 2 s budget;
  the runtime recognises Rust's "memory allocation of … failed" in that trap and the message
  starts with `out of memory:` (the playground shows "Out of memory — engine restarted" instead
  of a crash). Wasm memory never shrinks, so the runtime replaces a worker whose instance grew
  past 256 MiB. `instantiateFeel` (and any host without fuel or epoch interruption) cannot stop
  a long evaluation such as `for i in 1..1e30 return i`; use `FeelEngine` or a Wasmtime
  deadline for untrusted input.
- **Panics.** Any remaining panic inside dsntk traps the instance (`panic = "abort"`; integer
  overflow checks are on in release builds, so an overflow is a panic, never a wrong answer). The
  runtime recovers and adds the panic message to the error.
- **Time zones.** `tzOffsetMinutes` is one fixed offset per request, the browser's current
  offset. There are no DST rules for other dates. Without it, local time is UTC on every target.
- **Temporal comparisons.** Date-times and times with offsets or zones compare as instants
  (`@"…T10:00:00+01:00" < @"…T10:00:00Z"` is true, `@"10:00:00+02:00" = @"08:00:00Z"` is true;
  `is()` stays zone-strict). A zone-less (local) value is only ordered against another local
  value: `<`, `>`, `<=`, `>=` between a local and a zoned date-time are false, like dsntk's
  subtraction of the two (null), although `=` applies the fixed offset. `max`/`min` accept only
  numbers and strings in dsntk (`max(dt1, dt2)` is null); use `sort(…)[1]` or `if a < b then …`.
- **Unary-test semantics.** The engine evaluates `? in (<tests>)`. A boolean
  expression test such as `? > 5` is satisfied when it is true. The usual forms `> 5`,
  `[1..10]` and `"a", "b"` work; a tested value of another kind than the tests gives null. The endpoint of a comparison
  takes the arithmetic after it: `> 0.1 + 0.2` tests `? > (0.1 + 0.2)`. See CONTRACT §2.
- **Parsing needs context.** Names with spaces or operators (`Grace's age`, `a+b`) are only split
  correctly when they are in the context, so the playground sends the context to `parse` too.
- **The editor reads a text as the engine does.** The playground's FEEL editor takes its
  colours, its error marks (squiggle, gutter marker, "Jump to error"), its completions and the
  tree with positions from the engine's own parser (`syntax` and `complete`, docs/CONTRACT.md
  §3), through `@rheon/feel-codemirror`. A syntax error of `evaluate` carries the same
  diagnostics with their places (`error.diagnostics`).
- **Context entries are sorted.** dsntk keeps contexts in a `BTreeMap`, so results list keys in
  name order.
- **Only Chromium was tested** in a real browser.
