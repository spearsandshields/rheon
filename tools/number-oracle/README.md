# The number oracle

Every number result of a `feel.wasm` is judged against its truth: the value DMN 1.6 defines,
computed independently of the engine in Python (`decimal`, exact rationals and mpmath), and the
one canonical text of that value. The oracle sends real requests to the module, so it judges what
a consumer sees, through the same JSON and FEEL paths.

```sh
bash tools/number-oracle/run.sh [quick|full] [module.wasm]     # default: quick, dist/feel.wasm
```

`PYTHON` names the interpreter (default `python3`). It must have the C `decimal` module and
mpmath 1.3.0; with neither at hand, a virtual environment does:

```sh
python3 -m venv .tools/py
.tools/py/bin/pip install --require-hashes -r tools/number-oracle/requirements.txt
PYTHON=.tools/py/bin/python bash tools/number-oracle/run.sh quick
```

CI runs the same quick run after the identity check (`.github/workflows/ci.yml`, job `wasm`), in a
virtual environment of the runner's `python3`. `ORACLE_SEED` (digits, default 1) changes the
random parts. Exit status: 0 every case right or
exempt, 1 a wrong answer or a stale exemption, 2 usage, environment or input.

| File | What it is |
|---|---|
| `oracle.py` | The truths: pure functions over `decimal.Decimal`, no I/O. `--self-test` checks them against the examples of the specification and against second formulations, without an engine |
| `cases.py` | The families of requests: fixed edge vectors and a random part per family |
| `judge.py` | Draws the same cases again, reads the module's answers in order, gives each a verdict, applies the exemption list and reports |
| `known-wrong.json` | The exemption list: the classes of wrong answers the engine is known to give |
| `requirements.txt` | mpmath 1.3.0 with the sha256 of its wheel |
| `run.sh` | self-test, then `cases.py` writes the requests, `scripts/feel-answers.mjs` answers them with the module, and `judge.py` judges the answers as they arrive |

## The truths

Basis for all, DMN 1.6 §10.3.2.3.1: "FEEL Numbers are based on IEEE 754-2008 Decimal128 format,
with 34 decimal digits of precision and rounding toward the nearest neighbor with ties favoring the
even neighbor", "There is no distinction between -0 and 0", and "There is no value for
notANumber, positiveInfinity, or negativeInfinity. Use null instead." Every operation is applied to
the exact values of its operands and rounded once to 34 digits, half even, with gradual underflow
(subnormal results down to `1E-6176`); a result that overflows is null.

| Result | Truth | Clause |
|---|---|---|
| `+ - * /`, unary `-`, `abs` | the exact result rounded once; division by zero is null | Tables 58 and 60 |
| `= < <= > >=` | exact comparison of the values: `-0 = 0`, `1 = 1.000` | §10.3.2.3.1; Table 40 |
| `decimal(n, s)` | n rounded to a multiple of 10^-s, half even | Table 77: "return n with given scale"; `decimal(2.5, 0) = 2` |
| `floor`, `ceiling`, `round up`, `round down`, `round half up`, `round half down`, with and without a scale | n rounded to a multiple of 10^-s by the function's mode; a number with no digit below the scale is itself | Table 77: "Return n with given scale and rounding mode"; `round half down(-1.126, 2) = -1.13` |
| a scale outside -6111..6176 | null | Table 77, footnote 1: "Scale is in the range [−6111..6176]" |
| a scale with a fraction | the scale without its fractional part: `round up(1.121, 2.7)` is 1.13 | Table 77, footnote 2: "If scale is decimal numbers, the implicit conversion from decimal to integer is applied"; §10.3.2.9.4: "any fractional part of this number will be discarded" |
| `modulo(a, b)` | a - b·floor(a/b) over the exact values, rounded once; b = 0 is null | Table 77: "modulo (dividend, divisor) = dividend - divisor*floor (dividend/divisor)"; owner decision: the formula over exact values, rounded once |
| `odd(n)`, `even(n)` | the parity of an integer; a number that is not an integer is neither, so both are false; an argument that is not a number is null | Table 77: "Returns true if number is odd, false if it is even"; `odd(5) = true`, `even(2) = true` |
| `sqrt(x)` | correctly rounded; x < 0 is null | Table 77: "If number is negative it returns null." |
| `exp(x)`, `log(x)` | correctly rounded (the interval rule below), and Python's correctly rounded `exp` and `ln` must agree; `log` of x <= 0 is null | Table 77 |
| `a ** b` | an exponent outside -999,999,999..999,999,999 is null; an integer exponent: the exact rational power rounded once while it has at most about 20,000 digits, else the interval rule; a fractional exponent: the interval rule; a negative base with a fractional exponent and 0 to a negative power are null; `0 ** 0` is 1 | Table 61: "value(e2) is a number in the range [-999,999,999..999,999,999]", "p is limited to 34 digits", "ε is rounding error"; IEEE 754-2008 `pow(x, ±0)` is 1; owner decision: correctly rounded |
| `number(from)`, `number(from, grouping, decimal)` | the separators of footnote 1 (grouping: space, comma, period or null; decimal: period, comma or null; not the same unless both are null); the text after removing the grouping separator and changing the decimal separator to a period must be a numeric literal (grammar rule 35), else null; its value rounded once; `number(5)` is 5 | Table 73 and its footnote 1. The footnote names "grammar rule 37", which is `digits`; `number("1.1") = 1.1` needs rule 35, the numeric literal |
| a FEEL literal, a JSON number | its value rounded once; beyond the range of decimal128 a literal is null, and a JSON number is a `context` error (CONTRACT §2), which no case sends | grammar rule 35: "1.2e3, which is equivalent to 1.2*10**3" |
| the text of a number: `display`, `feel`, `typed`, `string(n)` | plain digits of the value: no exponent, no trailing zeros after the point, zero is `0`, no `-0` | Table 73: `string(1.1) = "1.1"`; owner decision: canonical values in every text form. Table 40 shows texts such as `1.00` for `decimal(1, 2)`; the oracle judges their values |

**The interval rule.** mpmath computes the result at P digits plus guard digits that cover the
condition of the function (the digits of the argument of `exp`, of the exponent of `**`, those lost
by `log` near 1), so the computed v is within 10^-P of the result, relatively. Both ends of
v·(1 ± 10^(8-P)) are rounded to 34 digits; when they agree, that is the truth. When they are
neighbours and the result is exactly the rounding boundary between them (a tie, or the overflow or
the underflow threshold, which only a rational result can be: `(s^2) ** 1.5` with a 35-digit `s^3`
ending in 5), the boundary is rounded exactly. Otherwise P doubles from 60 up to 4,000 digits; a
result still undecided there is the verdict `undecided`. A result far outside the range is an
overflow or zero without computing it exactly.

The self-test (`python3 -I oracle.py --self-test`, about 3 s) checks every example of Table 77,
the conversion examples of Table 73 and §10.3.2.3.1, the rows of Table 40 (their values), the
domain rules above, `modulo` against the remainder of a wide context on 10,000 random pairs,
integer powers against repeated exact multiplication, the interval rule on integer powers against
the exact path, and `sqrt` against `math.isqrt` on perfect squares and their neighbours.

Any arithmetic on `Decimal` outside the oracle's own contexts is exact or raises, so an operand
cannot be rounded by Python's default context unnoticed.

## Verdicts and the exemption list

| Verdict | Meaning |
|---|---|
| `ok` | the truth, in its canonical text in every text form |
| `value` | a number (or boolean, or string) that is not the truth; a number reports its distance in units of the last place |
| `null-for-number` | null where the truth is a value |
| `number-for-null` | a value where the truth is null |
| `negative-zero` | the right value, a zero written with a minus sign |
| `text` | the right value, a text form that is not the canonical text |
| `error` | the response is not `ok` |
| `trap` | the instance trapped |
| `undecided`, `oracle-disagreement` | the oracle has no truth: the interval rule ran out of precision, or two libraries disagree |

A number result's value is read from `result.display`; `result.feel` and `result.typed`'s text are
its other text forms. A string result is its typed text.

`known-wrong.json` lists `{"family": <glob>, "verdict": <verdict>, "why": <text>}`. A case that
fails with a listed verdict in a matching family is exempt; any other wrong answer fails the run.
An entry that no failing edge vector matches is stale and fails the run, so the change that
repairs a class deletes its entry in the same change, and the list cannot outlive the defects it
names. Only edge vectors keep an entry alive, because they run in every size: the quick and the
full run agree on staleness. `trap`, `undecided` and `oracle-disagreement` cannot be listed.

The report prints, per family, the cases, the right, the exempt and the failing ones, then every
class (family and verdict) that is not right with its count, and the first three failing requests
of each failing class as complete request lines that can be sent to any host.

## The families

Each family has fixed edge vectors, sent in every run in literal form (every operand written in the
expression), and a random part from `random.Random(f"{seed}/{family}")`, 200 cases in a quick run
and 20,000 in a full one, whose operands travel in the request context as JSON numbers. A request
is sent once: the edge vectors first, in the family that lists one first, then the random parts
(a random case equal to an edge vector is left out).

The edge vectors are also a committed source of the identity check:
`tools/identity/requests/numbers.json` is the output of `python3 -I -B cases.py --edges`, and
`run.sh` fails when it is not, so the two cannot drift. Every change of an answer on an edge vector
is therefore registered in `tools/identity/registered/` by the change that makes it, and each
registered new answer is one the oracle has judged. After a change of `cases.py` the file is
written again with that command.

- **Operand pool**: 1 to 34 digits, exponents near 0 and over the whole range with weight on its
  ends; zeros of every sign and exponent; cohorts of one value (`1`, `1.0`, `1.00E+2`, `100`); the
  largest and smallest normal and subnormal numbers; 34 nines.
- `spec/examples`: the examples of Table 77 and the rows of Table 40.
- `add`, `sub`, `mul`, `div`: `random`, `tie` (the exact result halfway at digit 35, both
  neighbours, both signs), `cancellation` (add and sub), `cohort`, `zero`, `overflow`, `underflow`
  (subnormal results and ties at the subnormal quantum), `scale-limit` (the exponent folded down at
  6111, the quantum `1E-6176`); `div/zero-divisor`; `arith/compound` (nested operations).
- `neg`, `abs`: `zero`, `cohort`, `random`; `compare`: `zero`, `cohort`, `random`.
- `decimal`, `floor`, `ceiling`, `round-up`, `round-down`, `round-half-up`, `round-half-down`: `tie`,
  `near-tie` (…4999…9 and …5000…1 out to 34 digits), `zero` (every zero at scales -5..5),
  `negative-to-zero`, `carry`, `scale-fine` (a scale beyond the digits: the number itself),
  `scale-coarse` (negative scales), `scale-limit` (the ends of the domain, one beyond, ±2^31,
  ±2^63, `1E+10`), `subnormal`, `scale-fraction` (a scale with a fractional part, within and at the
  ends of the domain), `random`, and `one-argument` for all but `decimal`.
- `modulo`: `sign`, `quotient-crosses-integer` (a/b within one unit of the 34th digit of an
  integer), `huge-quotient` (`modulo(1e40, 3)`), `underflow`, `zero-divisor`, `zero-dividend`,
  `random`.
- `pow-int`: `exact`, `inexact` (`1.0001 ** 1000000`, `(1 + a·10^-k) ** n`), `negative`,
  `domain` (±999,999,999 and one beyond, bases 1, -1, 1.000000001), `zero-base`,
  `negative-base`, `overflow`, `underflow`.
- `pow-frac`: `random`, `exact-root` (`4 ** 0.5`, `1024 ** 0.1`, the ties `(s^2) ** 1.5`),
  `negative-base`, `zero-base`, `overflow`, `underflow`, `domain`.
- `odd`, `even`: `integer` (cohorts such as `2.0` and `1E+1`, zeros, 34 nines, the largest and the
  smallest numbers, near-integers), `not-a-number` (null, a string), `random`.
- `sqrt`: `perfect`, `neighbour`, `random`, `negative`, `zero`, `subnormal`, `largest`.
- `exp`: `random` (in -14250..14150), `threshold` (the last finite result, the first overflow, the
  first subnormal, the first zero), `zero`, `tiny`; `log`: `random`, `near-one`, `zero`,
  `negative`, `one`, `largest`, `smallest-subnormal`.
- `number`: `literal-forms`, `long` (beyond 34 digits, ties), `overflow`, `underflow`,
  `separators`, `invalid`, `number-argument`.
- `string`: `integers`, `fractions`, `trailing-zeros`, `zero`, `exponents`, `extremes`,
  `round-trip` (`number(string(x)) = x`), `length`.
- `literal/digits` and `json-number/digits`: 1 to 60 digits written in the expression and sent in
  the context: the rounding of an input beyond 34 digits, and the ends of the range.

Not judged here, because no FEEL expression reaches them alone: the conversions to machine
integers, `trunc` and `frac` of the number core, and its assign operators. They keep their unit
tests in `feel-number`.

## Cost

Measured on a MacBook (Apple silicon, Node 26, Python 3.12 with the C `decimal`): 188 families,
2,535 edge vectors. A quick run is 35,937 requests and takes 6.8 s, the self-test 3 s of it; a full
run is 2,896,589 requests and takes 5.3 min, with a peak of 1.3 GB. The module answers about
130,000 requests a second; the judge's Python is most of the rest.
