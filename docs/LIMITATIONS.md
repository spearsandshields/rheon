# Known limitations

What the engine does not do, where it differs from other engines, and the limits it enforces.
Known bugs are in [BACKLOG.md](BACKLOG.md).

- **Java external functions are not supported.** They need a JVM. On every target they evaluate to
  null with a reason, and an evaluation makes no network call: native builds up to 0.9 posted the
  call to a Java service on `127.0.0.1:22023`.
- **PMML and ONNX external functions are not supported.** In FEEL, a function with a `pmml` or
  `onnx` mapping (DMN 1.7 Beta 1, 10.3.2.13.3) is a function value, and invoking it gives null with
  a reason; so does invoking a boxed PMML function definition of a DMN model. Up to 0.9 a PMML call
  returned a made-up string naming the document and the model. The three kinds are optional for
  conformance (2.1). The DMN model layer still differs in three places (BACKLOG.md, *Java and PMML
  BKMs are not external*): a BKM of kind Java or PMML returns its mapping context, a boxed ONNX
  function definition is null rather than a function, and a boxed PMML mapping needs its `model`.
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
  `is()` compares the value itself, zone included). A zone-less (local) value takes the offset of
  the other operand (DMN 1.7 Beta 1, Table 55, which the owner applies to `=`, ranges, unary
  comparisons, `between`, `min`, `max` and sort too): `time("10:00:00") = time("10:00:00Z")` is true
  and `time("10:00:00") < time("11:00:00+01:00")` is true. So `=` is not transitive across kinds
  (`10:00:00 = 10:00:00Z` and `10:00:00 = 10:00:00+01:00`, but not `10:00:00Z = 10:00:00+01:00`).
  `=` compares whole seconds (TCK 0068 time_005); `<` keeps the fraction of the second. Two values
  have no order only where a zone gives one of them no single offset (a clock change hour) and they
  are not written in the same zone.
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
- **Context entries are shown in the order they were written.** A context keeps its entries by
  name and the written order beside it; results, `string()` and the JSON list the entries in the
  written order, and equality does not depend on it. The entries of a context literal are evaluated
  in the order they depend on each other, not as written.
- **Readings of rules the spec leaves unclear or contradicts.** Each follows the official TCK
  where it decides, otherwise the reading given here:
  - *Unknown string escapes* (DMN 1.7 Beta 1, grammar rules 33 and 64). An escape outside rule 64,
    such as `"\q"` or `"\d"`, is kept as the backslash and the character, so `string length("\s")`
    is 2, while `"\n"` is a newline. TCK 0067, 1109 and 1111 write regex patterns with one backslash
    and need it.
  - *`<=` and `>=` on values that have no order* (Table 51 against Table 55). They are null for
    kinds without an order, as `<` and `>` are: `null <= null` and `[1] <= [1]` are null, although
    "`<` or `=`" would give true.
  - *Equality of the two duration kinds* (9.4 against Table 53). `@"P1D" = @"P1M"` and
    `@"P0D" = @"P0Y"` are null: Table 53 requires one kind, and TCK 0068 (`P1Y = P365D` is null)
    reads it so; 9.4 says the kinds compare.
  - *B-FEEL results that stay null* (11.1 against 11.2-11.11). 11.1 says a B-FEEL null only means
    missing data, but the sections that follow give results only for boolean operators and
    functions, functions by their result type, and the arithmetic operators. Where none applies,
    B-FEEL keeps FEEL's null: unary negation (`-"a"`), paths and filters on null, and the functions
    whose result has no default: `context`,
    `context merge`, `context put`, `get value`, `max`, `min` (a context or any type). So does an
    arithmetic conversion whose FEEL operation on the converted values still fails (11.9-11.10).
    Their FEEL errors stay errors. DMN 1.7 Beta 1 adds to 11.7 that `for x in null return x` is `[]`
    in B-FEEL; rheon answers `[null]`, as DMN 1.6 has it, until the 1.7 answers that change 1.6
    ones are gated (owner decision 3, with the FEEL rule of DT-017).
  - *Range literals out of order or of two types* (10.3.2.7 against Tables 56 and 79). 10.3.2.7 says
    the endpoints of `[a..b]` "must be of equivalent types" and ordered, but gives no answer for a
    range that is not; the tests on ranges are defined by comparisons of the endpoints (Table 56),
    and the range functions by formulas (Table 79), which answer for every range. So `[10..1]` is a
    range value: `5 in [10..1]` is false, `2 in [1.."z"]` is null (2 <= "z" is null), and
    `before([1..2], [4..3])` is true. Making such a literal null would make `2 in [1.."z"]` the test
    `2 in null`, which is false.
  - *The result type of a function literal* (10.3.2.9, 10.3.2.9.2). It is not inferred: the type of
    `function(x: number) x` is `function<number>->Any`, so it is no instance of
    `function<number>->number`. Bound to a parameter or a variable of a function type, a function
    whose result type is Any (not declared) is not held against the target's result type, so
    `f(function(x) x * 2)` works where f takes a `function<number>->number`; its parameters are
    checked (contravariant).
  - *Equality of functions* (Table 54: "the same parameters, body, and scope"). The body is the
    function literal where it is written: two literals are two functions even when their text is
    alike (`{f: function(x) x}.f = {f: function(x) x}.f` is false), and one literal evaluated twice
    in equal scopes gives equal functions. An external function is equal to one with the same
    parameters and the same Java class and method signature, or PMML document and model; an ONNX
    function, whose mapping rheon does not keep, only to itself. A function of a DMN model (a boxed
    expression body) is the same when it is the same evaluator. A built-in function equals only
    itself. The values a function closes over are compared by `=`: functions that close over
    `@"2020-01-01T10:00:00Z"` and `@"2020-01-01T11:00:00+01:00"` are equal, although `string(x)` in
    their bodies differs.
  - *The endpoints of `(!= 10)`* (10.3.2.7). Its `start`, `end` and inclusion properties are those
    of `(= 10)`; the spec's rule (the other endpoint undefined, not included) fits neither form.
  - *`count(null)`* (10.3.4 against the conversion to a singleton list, 10.3.2.9.4). It is null,
    as an argument outside a built-in's domain is; a value that is no list and not null counts as
    a list of one item. Upstream dsntk read null as `[null]` and answered 1.
  - *The components of a negative duration* (10.3.2.15, Table 63). `duration("-P2Y3M").years` is 2
    and `duration("-P2D").days` is 2: the properties are magnitudes, and the sign is the duration's
    (owner Q-T6); XPath's `years-from-duration` would give -2.
  - *Adding months to a date at the end of a month* (Table 58). `date("2020-03-31") - duration("P1M")`
    is 2020-02-29 and `date and time("2021-01-31T10:00:00") + duration("P1M")` is
    2021-02-28T10:00:00: the day is clamped to the end of the shorter month, as 9.4 and the XPath
    functions do (FORK item 61); the literal Table 58 formula gives date(2020, 2, 31), which is null.
  - *`number(from, grouping separator, decimal separator)`* (10.3.4.1, footnote). The cleaned
    string is checked against grammar rule 35 (a numeric literal); the footnote cites rule 37
    (digits), which would reject the table's own example `number("1.1")`.
- **Only Chromium was tested** in a real browser.
