# FEEL engine edge-case comparison: feelin vs feel-scala vs dsntk

Measured on 2026-10-16. Purpose: judge whether the crash/odd behaviours found in dsntk during
our adversarial review are unusual, by running the same edge cases through the two other
widely used FEEL engines. (The raw run lived in a scratch directory that did not survive a
reboot; the numbers below are the recorded results.)

| Engine | Version | How it was run |
|---|---|---|
| feelin | 8.2.0 (JavaScript; plain JS `Number`, no decimal library) | Node 26, one child process per case, 20 s timeout |
| feel-scala | 1.22.1 (Scala/JVM, Camunda) | Camunda's public playground API (`feel-playground.camunda.com`), server-side `evaluationTimeInMillis` |
| dsntk | upstream commit c76aee0, native, Intel BID128 numbers | release binary, 30 s timeout |

`null` = FEEL null returned; THROWS = exception surfaced, process alive; PANIC = process abort;
TIMEOUT = killed at the cap.

## Results

| Case | Expression | feelin | feel-scala | dsntk (upstream, native) |
|---|---|---|---|---|
| A1 | `0.1 + 0.2` | `0.30000000000000004` | `0.3` | `0.3` |
| A2 | `1/3` | 16 digits | 34 digits | 34 digits |
| A3 | `decimal(1/3, 30)` | 16 digits (double) | 30 digits | 30 digits |
| A4 | `9999999999999999999999999999999999 + 1` | `1e+34` | `1.000000000000000000000000000000000E+34` | `10000000000000000000000000000000000` |
| A5 | `12.2384283 ** 4.25` (true `41959.85737359436186095331070746801`) | `41959.85737359437` | `41959.85737359437` (via `Math.pow`) | `41959.857373594361860953310707468` (1 ulp off) |
| A6 | `exp(1)` | 16 digits | 16 digits (double) | `2.718281828459045235360287471352662` |
| A7 | `log(10)` | 16 digits | 16 digits (double) | `2.302585092994045684017991454684364` |
| B1 | `9e6144 + 9e6144` | `18` (exponent literal silently read as `9`) | parse error | **PANIC** (`dfp_number.rs:440` unwrap) |
| B2 | `10 ** 400` | `Infinity` | `1.000…E+400` (JSON shows `"Infinity"`) | exact 401-digit integer |
| B3 | `1e400 * 1e400` | `1` (literals read as `1`) | parse error | exact 801-digit integer |
| B4 | `decimal(1e33, 1)` | `1` | parse error | **PANIC** (same site) |
| B5 | `1/0` | `null`, no warning | `null` + warning | `null` + reason |
| C1 | `time(23, 59, 59, duration("PT20H"))` | `"23:59:59+20:00"` (accepts) | `null` + warning | **PANIC** (`bifs/core.rs:2758`) |
| C2 | `time(1, 2, 3, duration("P1D"))` | `null` + warning | `null` + warning | **PANIC** (same site) |
| D1 | `"\uD800"` | lone surrogate kept | `\u` never interpreted (6-char literal) | **PANIC** (`lexer.rs:810`) |
| D2 | `string length("\uD800")` | `1` | `6` | lexer error |
| D3 | `"a" + "\uD800" + "b"` | `"a\ud800b"` | 8-char literal | lexer error |
| E1/E2 | 26-deep nested list, `count(flatten(…))` | ok | ok | ok |
| E3 | 5000 nested parentheses | THROWS (fails above ~2,700) | THROWS `StackOverflowError` (fails above ~450) | `1` in 5 ms |
| E4 | `1 + 1 + … + 1`, 5000 terms | THROWS (fails above ~650) | THROWS `StackOverflowError` (fails above ~650) | `5000` in 8 ms |
| F1 | recursive context function, `f(10)` | `0` | `null` (recursive self-reference in a context literal unsupported) | `0` |
| F2 | same, `f(100000)` | THROWS `RangeError` (~1,000–1,500 frames) | `null` (never recurses) | **PANIC** (native stack overflow) |
| G1 | `count(for i in 1..10000 return i)` | 7 ms | 79 ms | 1,034 ms |
| G2 | `sum(for i in 1..100000 return i)` | 23 ms | 326 ms | TIMEOUT (> 30 s) |
| G3 | `count(for i in 1..1000000 return i)` | 146 ms | 2,472 ms | TIMEOUT (> 60 s) |
| H1 | `sublist([1,2,3], 5000000000)` | `[]` | `[]` | `null` + reason |
| H2 | `substring("abc", 5000000000)` | `""` | THROWS `StringIndexOutOfBoundsException` (32-bit truncation) | `null` + reason |
| H3 | `substring("abc", 2, 4294967298)` | `"bc"` | `"bc"` | `null` + reason |
| I1 | unary `> 5`, input 7 | `true` | `true` | `true` |
| I2 | unary `? > 5`, input 7 | `true` | `true` | `false` (compares the boolean with the input) |
| I3 | unary `[18..65)`, input 42 | `true` | `true` | `true` |
| J1 | `date("2026-13-45")` | `null` + warning | `null` | `null` + reason |
| J2 | `duration("P1Y") + duration("P1D")` | `null` + warning | `null` + warning | `null` + reason |
| J3 | `now() - date and time("2026-01-01T00:00:00Z")` | works | works | `null` (dsntk's `now()` is zone-less) |

## Time zones (2026-10-16, dsntk run with the host offset -04:00)

| Case | feelin | feel-scala | dsntk |
|---|---|---|---|
| `now()` | `…@America/New_York` | `…@Etc/UTC` (server zone) | zone-less local date-time |
| `now().timezone` / `.time offset` | zone / `-PT4H` | zone / `P0D` | `null` / `null` |
| `@Europe/Berlin` with DST-correct offsets | yes | yes | yes (chrono-tz) |
| `10:00+02:00 − 08:00Z` (same instant) | `PT2H` (wrong) | `P0D` | `PT0S` |
| `10:00@Europe/Berlin − 08:00Z` (summer) | `PT2H` (wrong) | `P0D` | `PT0S` |
| zone-less − zoned | treats local as UTC | `null` + warning | `null` + reason |
| zone-less `<` / `=` zoned | compares as instants | `null` + warning | `false` |
| DST gap `2026-03-29T02:30:00@Europe/Berlin` | kept | normalised to `03:30` | kept |

## Patterns

- **feelin** uses binary doubles (`0.1+0.2 ≠ 0.3`), silently drops exponent literals, ignores
  offsets when subtracting date-times. Nothing hangs; parser fails at a few hundred nesting levels.
- **feel-scala** uses `BigDecimal` DECIMAL128 for arithmetic but `double` for `exp`/`log`/`**`;
  stack-overflows at ~450 nested parentheses; one 32-bit truncation exception; strict (null) on
  mixed-zone comparisons.
- **dsntk** has the best numerics (only engine with 34-digit transcendentals) and the most robust
  parser (5,000 nesting levels), and the most informative null reasons. Its unusual traits were
  (1) panicking where the other engines fail catchably, (2) quadratic `for` loops, (3) `?` in a
  unary test not binding, (4) a zone-less `now()`.

## What changed in our fork (dsntk-rs `wasm` branch)

- All panic sites above return `null` with a reason or a syntax error (robustness commit).
- `for` loops and list literal parsing are linear (`count(for i in 1..1000000 return i)` ≈ 0.11 s
  in wasm, measured 2026-11-13).
- Numbers come from the dashu-float backend: bit-identical to Intel on exact operations,
  correctly rounded on `exp`/`ln`/`log`/fractional `pow` (so A5 gives `…46801`).
- Verification round of 2026-10-17 fixed four more engine bugs that exist upstream too:
  `round up(0.1, 1)` returned `1` (any scale rounding whose coefficient is ±1); `for i in 1e34..1e34`
  never terminated (step lost to 34-digit rounding); `23:59:59 + PT1S` gave `24:00:00`; `<`/`>` on
  times and date-times with different offsets were always `false` (now compared as instants, so
  the "zone-less `<` zoned" row above stays `false` but offset-vs-offset ordering works). The lexer's
  name-prefix search was cubic on long `.a.a.a` chains; now linear.
- 2026-10-18: the scale-based rounding functions accept the full FEEL scale range (-6111..=6176):
  `round up(5.5, 6176)` is `5.5` and `decimal(1e33, 1)` (B4) is `1e33` instead of null, since a
  value with no digit below the requested scale is returned unchanged. `matches` supports
  back-references (`(a)\1`, fancy-regex), with a backtracking limit instead of a hang.
- Open: `? > 5` semantics (upstream PR #112 has a fix), arithmetic in unary-test endpoints needs
  parentheses (`> (0.1 + 0.2)`, upstream grammar precedence), and a zoned `now()` (proposed).
