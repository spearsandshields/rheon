"""The truths of FEEL number results (DMN 1.6), as pure functions over decimal.Decimal.

An expression is a tree: a number leaf is its literal text ("1.50", "-0", "1E+40"), a string leaf is
["str", text], the null leaf is ["null"], and an operation is [op, argument…] with an op of TRUTHS.
evaluate(tree) answers its truth: a Decimal (a finite FEEL number), a bool, a str, or None (null).

Basis: DMN 1.6 §10.3.2.3.1, "FEEL Numbers are based on IEEE 754-2008 Decimal128 format, with 34
decimal digits of precision and rounding toward the nearest neighbor with ties favoring the even
neighbor", and "There is no value for notANumber, positiveInfinity, or negativeInfinity. Use null
instead." Every operation is applied to the exact values of its operands and rounded once into
D34. The clause of each operation is next to its function.

`python3 -I oracle.py --self-test` checks the truths against the examples of the specification and
against second formulations; it needs no engine.
"""
import decimal
import math
import re
import sys
from decimal import (
    Context, Decimal, InvalidOperation, ROUND_CEILING, ROUND_DOWN, ROUND_FLOOR, ROUND_HALF_DOWN,
    ROUND_HALF_EVEN, ROUND_HALF_UP, ROUND_UP,
)
from fractions import Fraction

import mpmath

if hasattr(sys, 'set_int_max_str_digits'):
    sys.set_int_max_str_digits(0)
# Arithmetic on Decimals outside the contexts below is exact or fails: an implicit rounding in the
# default context would silently change an operand.
decimal.setcontext(Context(prec=1_000_000, Emax=decimal.MAX_EMAX, Emin=decimal.MIN_EMIN,
                           traps=[InvalidOperation, decimal.Inexact, decimal.Overflow]))

# IEEE 754-2008 decimal128: 34 digits, exponents of the coefficient -6176..6111, gradual underflow.
D34 = Context(prec=34, rounding=ROUND_HALF_EVEN, Emax=6144, Emin=-6143, clamp=1, traps=[])
# Exact work on results that always fit (the scale functions): any inexactness raises.
WIDE = Context(prec=200, rounding=ROUND_HALF_EVEN, Emax=10**6, Emin=-10**6, traps=[InvalidOperation])
MAX = Decimal('9.999999999999999999999999999999999E+6144')
EXPONENT_LIMIT = 999_999_999  # Table 61
SCALE_MIN, SCALE_MAX = -6111, 6176  # Table 77, footnote 1
FIRST_PRECISION, LAST_PRECISION = 60, 4000
EXACT_POWER_BITS = 66_500  # about 20,000 digits: beyond, a power is decided by the interval rule


class Undecided(Exception):
    """The interval rule found no truth up to its last precision."""


class Disagreement(Exception):
    """Two libraries give two truths for one result."""


def finite(d):
    """A D34 result as a FEEL value: a non-finite result is null."""
    return d if d is not None and d.is_finite() else None


def number(text):
    """A FEEL literal or a JSON number (grammar rule 35): its exact value rounded once."""
    return finite(D34.create_decimal(text))


def round_exact(fraction):
    """An exact rational rounded once into D34 (the division of two exact integers is correctly
    rounded, subnormal results included). Infinite on overflow."""
    return D34.divide(Decimal(fraction.numerator), Decimal(fraction.denominator))


def canonical(d):
    """The one text of a number (owner decision: canonical values in every text form): plain
    digits of the value, no exponent, no trailing zeros of a fraction, zero is "0"."""
    if d == 0:
        return '0'
    text = format(d, 'f')
    if '.' in text:
        text = text.rstrip('0').rstrip('.')
    return text


def is_integer(d):
    return d == d.to_integral_value()


# --- the interval rule -----------------------------------------------------------------------------

def _fraction(v):
    """The exact value of an mpmath number."""
    sign, man, exp, _ = v._mpf_
    if man == 0:
        if v != 0:
            raise Undecided('mpmath gave a non-finite value')
        return Fraction(0)
    f = Fraction(man << exp) if exp >= 0 else Fraction(man, 1 << -exp)
    return -f if sign else f


# log2 of 10^6145 and of 10^-6177: beyond them a result is an overflow or rounds to zero.
_LOG2_ABOVE, _LOG2_BELOW = 20414, -20520


def _beyond_range(v):
    """Infinity or zero (signed) for an mpmath result far outside D34's range, else None. The
    exact value of such a result is not needed and could be too large to hold."""
    sign, man, exp, bc = v._mpf_
    if man == 0:
        return None
    magnitude = exp + bc  # 2^(magnitude-1) <= |v| < 2^magnitude
    if magnitude > _LOG2_ABOVE:
        return Decimal('-Infinity') if sign else Decimal('Infinity')
    if magnitude < _LOG2_BELOW:
        return Decimal('-0') if sign else Decimal(0)
    return None


def _half_ulp_beyond(d):
    """Half the distance from a finite D34 value to the next one away from zero, in its binade."""
    up = D34.next_plus(d.copy_abs())
    if up.is_infinite():
        return Fraction(10 ** 6111, 2)
    return (Fraction(up) - Fraction(d.copy_abs())) / 2


def _boundary(lo, hi):
    """The rounding boundary between two neighbouring D34 results, or None."""
    if lo.is_infinite() and hi.is_infinite():
        return None
    if hi.is_infinite():
        return Fraction(lo) + _half_ulp_beyond(lo) if D34.next_plus(lo) == hi else None
    if lo.is_infinite():
        return Fraction(hi) - _half_ulp_beyond(hi) if D34.next_minus(hi) == lo else None
    if D34.next_plus(lo) != hi:
        return None
    return (Fraction(lo) + Fraction(hi)) / 2


def decide(compute, guard, exact=None):
    """The correctly rounded D34 value of a result that mpmath computes.

    compute() runs at the working precision set here: P + guard digits, where guard covers the
    condition of the function, so the computed value is within 10^-P of the result, relatively.
    Both ends of the enclosure v(1 ± 10^(8-P)) are rounded once; when they agree, that is the
    truth. Otherwise, when the ends are neighbours and exact(boundary) says the result is the
    rounding boundary itself (a tie, or the overflow or underflow threshold), the boundary is
    rounded exactly. Else the precision doubles, up to LAST_PRECISION.
    """
    p = FIRST_PRECISION
    while True:
        with mpmath.workdps(p + guard):
            m = compute()
        beyond = _beyond_range(m)
        if beyond is not None:
            return beyond
        v = _fraction(m)
        margin = abs(v) / 10 ** (p - 8)
        lo, hi = round_exact(v - margin), round_exact(v + margin)
        if lo == hi:
            return lo
        boundary = _boundary(lo, hi)
        if boundary is not None and exact is not None and exact(boundary):
            return round_exact(boundary)
        if p >= LAST_PRECISION:
            raise Undecided(f'undecided at {p} digits')
        p *= 2


# |ln| of a result beyond which it certainly overflows (ln MAX is about 14149.4) or rounds to zero
# (ln of half the smallest subnormal is about -14221.4).
_LN_BEYOND = 14300


def _ln_estimate(d):
    """ln |d| to about 15 digits, for a finite non-zero d of any exponent."""
    adjusted = d.adjusted()
    return adjusted * math.log(10) + math.log(float(d.copy_abs().scaleb(-adjusted)))


def _digits(d):
    """The number of digits before the point of |d| (0 for |d| < 1)."""
    return max(0, d.adjusted() + 1) if d != 0 else 0


def _magnitude_of_log(d):
    """An upper bound of the digits of 1/|ln d| that are lost near d = 1, as a guard."""
    gap = (d - 1).copy_abs()
    if gap == 0 or gap >= Decimal('0.5'):
        return 0
    return -gap.adjusted() + 1


# --- arithmetic: Tables 58 and 60 ------------------------------------------------------------------

def _null_in(*args):
    return any(a is None for a in args)


def add(a, b):
    return None if _null_in(a, b) else finite(D34.add(a, b))


def sub(a, b):
    return None if _null_in(a, b) else finite(D34.subtract(a, b))


def mul(a, b):
    return None if _null_in(a, b) else finite(D34.multiply(a, b))


def div(a, b):
    if _null_in(a, b) or b == 0:
        return None
    return finite(D34.divide(a, b))


def neg(a):
    return None if a is None else D34.minus(a)


def abs_(a):
    return None if a is None else D34.abs(a)


# --- comparisons: "There is no distinction between -0 and 0" ---------------------------------------

def _compare(test):
    def compare(a, b):
        if _null_in(a, b) or type(a) is not type(b):
            return None
        return test(a, b)
    return compare


def eq(a, b):
    if _null_in(a, b) or (type(a) is not type(b)):
        return None
    return a == b


# --- scale functions: Table 77 ---------------------------------------------------------------------

def _quantize(n, scale, mode):
    """n with the given scale and rounding mode (Table 77): the multiple of 10^-scale that the mode
    picks. A number with no digit below the scale is itself. The result always fits: at most the
    digits of n, plus a carry. Footnote 2: "If scale is decimal numbers, the implicit conversion
    from decimal to integer is applied", which (§10.3.2.9.4, "from decimal to integer") means "any
    fractional part of this number will be discarded"."""
    if _null_in(n, scale):
        return None
    scale = scale.to_integral_value(rounding=ROUND_DOWN)
    if not SCALE_MIN <= scale <= SCALE_MAX:
        return None
    s = int(scale)
    if n.as_tuple().exponent >= -s:
        return n
    return finite(D34.create_decimal(n.quantize(Decimal((0, (1,), -s)), rounding=mode, context=WIDE)))


def _scaled(mode):
    def scaled(n, scale=Decimal(0)):
        return _quantize(n, scale, mode)
    return scaled


def decimal_(n, scale):
    """Table 77: "decimal(n, scale) … return n with given scale"; decimal(2.5, 0) = 2: half even."""
    return _quantize(n, scale, ROUND_HALF_EVEN)


def modulo(a, b):
    """Table 77: "modulo (dividend, divisor) = dividend - divisor*floor (dividend/divisor)";
    "divisor must not be 0 (zero)". Owner decision: the formula over exact values, rounded once."""
    if _null_in(a, b) or b == 0:
        return None
    fa, fb = Fraction(a), Fraction(b)
    return finite(round_exact(fa - fb * math.floor(fa / fb)))


def _parity(remainder):
    """Table 77: odd(number) "Returns true if number is odd, false if it is even", and even the
    reverse; `odd(5) = true`, `even(2) = true`. A number that is not an integer is neither odd nor
    even, so both are false for it; an argument that is not a number gives null."""
    def parity(a):
        if type(a) is not Decimal:
            return None
        return is_integer(a) and int(a) % 2 == remainder
    return parity


def sqrt(a):
    """Table 77: "If number is negative it returns null." The square root is correctly rounded."""
    if a is None or a < 0:
        return None
    return finite(D34.sqrt(a))


def exp(a):
    """Table 77: e raised to the power of the number. Two libraries must agree."""
    if a is None:
        return None
    if a == 0:
        return Decimal(1)
    if a.copy_abs() > _LN_BEYOND:
        return None if a > 0 else Decimal(0)
    truth = decide(lambda: mpmath.exp(mpmath.mpf(str(a))), 10 + _digits(a))
    second = D34.exp(a)
    if truth != second:
        raise Disagreement(f'exp({a}): mpmath {truth}, decimal {second}')
    return finite(truth)


def log(a):
    """Table 77: the natural logarithm; null for a number that is not positive. Two libraries
    must agree."""
    if a is None or a <= 0:
        return None
    if a == 1:
        return Decimal(0)
    truth = decide(lambda: mpmath.log(mpmath.mpf(str(a))), 10 + _magnitude_of_log(a))
    second = D34.ln(a)
    if truth != second:
        raise Disagreement(f'log({a}): mpmath {truth}, decimal {second}')
    return finite(truth)


def pow_(a, b):
    """Table 61: "value(e2) is a number in the range [-999,999,999..999,999,999]"; the result is
    (p,s) with "value(p,s)= value(e1)value(e2) + ε", "p is limited to 34 digits", "ε is rounding
    error". Owner decision: correctly rounded. 0 ** 0 is 1 (IEEE 754-2008 pow(x, ±0) is 1); 0 to a
    negative power and a negative base to a non-integer power have no value: null."""
    if _null_in(a, b) or b.copy_abs() > EXPONENT_LIMIT:
        return None
    if b == 0:
        return Decimal(1)
    if a == 0:
        return Decimal(0) if b > 0 else None
    integral = is_integer(b)
    if a < 0 and not integral:
        return None
    sign = -1 if a < 0 and int(b) % 2 else 1
    base = a.copy_abs()
    fa, fb = Fraction(base), Fraction(b)
    if integral:
        n = int(b)
        if (fa.numerator.bit_length() + fa.denominator.bit_length()) * abs(n) <= EXACT_POWER_BITS:
            return finite(round_exact(sign * fa ** n))

    def exact(boundary):
        # boundary = base^(p/q) exactly when boundary^q = base^p, decided while the numbers are small
        p, q = fb.numerator, fb.denominator
        size = (boundary.numerator.bit_length() + boundary.denominator.bit_length()) * q
        size += (fa.numerator.bit_length() + fa.denominator.bit_length()) * abs(p)
        return size <= 4 * EXACT_POWER_BITS and boundary > 0 and boundary ** q == fa ** p

    size = float(b) * _ln_estimate(base)
    if abs(size) > _LN_BEYOND:
        return None if size > 0 else Decimal(0)
    guard = 10 + _digits(b.copy_abs()) + _digits(b.copy_abs() * (abs(base.adjusted()) + 1) * 3)
    truth = decide(lambda: mpmath.power(mpmath.mpf(str(base)), mpmath.mpf(str(b))), guard, exact)
    return finite(D34.minus(truth) if sign < 0 else truth)


# --- conversions: Table 73 -------------------------------------------------------------------------

RULE_35 = re.compile(r'-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?')
GROUPING = {' ', ',', '.', None}
DECIMAL = {'.', ',', None}


def number_(text, grouping=None, decimal=None, *more):
    """Table 73: number(from) and number(from, grouping separator, decimal separator), footnote 1:
    "grouping separator SHALL be one of space (' '), comma (','), period ('.'), or null. Decimal
    separator SHALL be one of period, comma, or null, but SHALL NOT be the same as the grouping
    separator unless both are null. from SHALL conform to grammar rule 37, after removing all
    occurrences of the grouping separator, if any, and after changing the decimal separator, if
    present, to a period." The rule meant is the numeric literal, rule 35 (rule 37 is "digits",
    which `number("1.1") = 1.1` rules out); number(5) = 5."""
    if more:
        return None
    if isinstance(text, Decimal):
        return text if grouping is None and decimal is None else None
    if not isinstance(text, str) or grouping not in GROUPING or decimal not in DECIMAL:
        return None
    if grouping is not None and grouping == decimal:
        return None
    if grouping is not None:
        text = text.replace(grouping, '')
    if decimal is not None:
        text = text.replace(decimal, '.')
    return number(text) if RULE_35.fullmatch(text) else None


def string(a):
    """Table 73: string(1.1) = "1.1"; the text of a number is canonical()."""
    if a is None:
        return None
    if isinstance(a, Decimal):
        return canonical(a)
    if isinstance(a, bool):
        return 'true' if a else 'false'
    return a


def string_length(s):
    return Decimal(len(s)) if isinstance(s, str) else None


def identity(a):
    return a


TRUTHS = {
    'add': add, 'sub': sub, 'mul': mul, 'div': div, 'pow': pow_, 'neg': neg, 'abs': abs_,
    'lt': _compare(lambda a, b: a < b), 'le': _compare(lambda a, b: a <= b),
    'gt': _compare(lambda a, b: a > b), 'ge': _compare(lambda a, b: a >= b), 'eq': eq,
    'decimal': decimal_,
    'floor': _scaled(ROUND_FLOOR), 'ceiling': _scaled(ROUND_CEILING),
    'round up': _scaled(ROUND_UP), 'round down': _scaled(ROUND_DOWN),
    'round half up': _scaled(ROUND_HALF_UP), 'round half down': _scaled(ROUND_HALF_DOWN),
    'modulo': modulo, 'odd': _parity(1), 'even': _parity(0), 'sqrt': sqrt, 'exp': exp, 'log': log,
    'number': number_, 'string': string, 'string length': string_length, 'id': identity,
}


def evaluate(tree):
    """The truth of an expression tree. Raises Undecided or Disagreement."""
    if isinstance(tree, str):
        return number(tree)
    op, *args = tree
    if op == 'str':
        return args[0]
    if op == 'null':
        return None
    return TRUTHS[op](*[evaluate(a) for a in args])


# --- self-test -------------------------------------------------------------------------------------

def _self_test():
    import random
    failures = []

    def check(label, got, want):
        if got != want or type(got) is not type(want):
            failures.append(f'{label}: got {got!r}, want {want!r}')

    # Table 77 and Table 73 examples, Table 40 and §10.3.2.3.1, as written in the specification.
    examples = [
        (['decimal', ['div', '1', '3'], '2'], '0.33'), (['decimal', '1.5', '0'], '2'),
        (['decimal', '2.5', '0'], '2'),
        (['floor', '1.5'], '1'), (['floor', '-1.56', '1'], '-1.6'),
        (['ceiling', '1.5'], '2'), (['ceiling', '-1.56', '1'], '-1.5'),
        (['round up', '5.5'], '6'), (['round up', '-5.5', '0'], '-6'),
        (['round up', '1.121', '2'], '1.13'), (['round up', '-1.126', '2'], '-1.13'),
        (['round down', '5.5'], '5'), (['round down', '-5.5', '0'], '-5'),
        (['round down', '1.121', '2'], '1.12'), (['round down', '-1.126', '2'], '-1.12'),
        (['round half up', '5.5'], '6'), (['round half up', '-5.5', '0'], '-6'),
        (['round half up', '1.121', '2'], '1.12'), (['round half up', '-1.126', '2'], '-1.13'),
        (['round half down', '5.5'], '5'), (['round half down', '-5.5', '0'], '-5'),
        (['round half down', '1.121', '2'], '1.12'), (['round half down', '-1.126', '2'], '-1.13'),
        (['abs', '10'], '10'), (['abs', '-10'], '10'),
        (['modulo', '12', '5'], '2'), (['modulo', '-12', '5'], '3'), (['modulo', '12', '-5'], '-3'),
        (['modulo', '-12', '-5'], '-2'), (['modulo', '10.1', '4.5'], '1.1'),
        (['modulo', '-10.1', '4.5'], '3.4'), (['modulo', '10.1', '-4.5'], '-3.4'),
        (['modulo', '-10.1', '-4.5'], '-1.1'),
        (['sqrt', '16'], '4'),
        (['number', ['str', '1 000,0'], ['str', ' '], ['str', ',']], '1000.0'),
        (['number', ['str', '1.1']], '1.1'), (['number', ['str', '1.1'], ['null'], ['null']], '1.1'),
        (['number', '5'], '5'),
        (['number', ['str', '1.000.000,01'], ['str', '.'], ['str', ',']], '1000000.01'),
        (['decimal', '1', '2'], '1.00'), (['add', '.25', '.2'], '0.45'),
        (['mul', '.10', '30.00'], '3.0000'),
        (['sub', ['add', '1', ['mul', ['div', '3', '2'], '2']], ['pow', '2', '3']], '-4.0'),
        (['div', '1', '3'], '0.3333333333333333333333333333333333'),
        (['div', '1.01', '2'], '0.505'), (['decimal', '0.505', '2'], '0.50'),
        (['decimal', '0.515', '2'], '0.52'), (['mul', '1.0', ['pow', '10', '3']], '1000.0'),
        (['id', '1.2e3'], '1200'),
    ]
    for tree, want in examples:
        check(str(tree), evaluate(tree), Decimal(want))
    check('1 = 1.000', evaluate(['eq', '1', '1.000']), True)
    check('string(1.1)', evaluate(['string', '1.1']), '1.1')
    # the two examples are shown rounded to their last digit
    check('log(10)', evaluate(['log', '10']).quantize(Decimal('1E-11'), context=WIDE), Decimal('2.30258509299'))
    check('exp(5)', evaluate(['exp', '5']).quantize(Decimal('1E-12'), context=WIDE), Decimal('148.413159102577'))
    check('-0 = 0', evaluate(['eq', '-0', '0']), True)
    check('string(-0)', evaluate(['string', '-0']), '0')
    check('string(1.50)', evaluate(['string', '1.50']), '1.5')
    check('string(1E+3)', evaluate(['string', '1E+3']), '1000')
    check('0 ** 0', evaluate(['pow', '0', '0']), Decimal(1))
    check('0 ** -1', evaluate(['pow', '0', '-1']), None)
    check('(-8) ** (1/3)', evaluate(['pow', '-8', ['div', '1', '3']]), None)
    check('2 ** 1000000000', evaluate(['pow', '2', '1000000000']), None)
    check('1 ** 999999999', evaluate(['pow', '1', '999999999']), Decimal(1))
    check('1 / 0', evaluate(['div', '1', '0']), None)
    check('9E+6144 * 10', evaluate(['mul', '9E+6144', '10']), None)
    check('scale 6177', evaluate(['floor', '1', '6177']), None)
    check('scale -6112', evaluate(['floor', '1', '-6112']), None)
    check('round up(1E-6000, -6000)', evaluate(['round up', '1E-6000', '-6000']), Decimal('1E+6000'))
    check('4 ** 0.5', evaluate(['pow', '4', '0.5']), Decimal(2))
    check('1024 ** 0.1', evaluate(['pow', '1024', '0.1']), Decimal(2))
    check('number("1.2.3")', evaluate(['number', ['str', '1.2.3']]), None)
    check('number(".5")', evaluate(['number', ['str', '.5']]), Decimal('0.5'))
    check('number same separators', evaluate(['number', ['str', '1'], ['str', '.'], ['str', '.']]), None)
    check('floor(MAX, 6176)', evaluate(['floor', str(MAX), '6176']), MAX)
    check('round up(1E-6176, 6176)', evaluate(['round up', '1E-6176', '6176']), Decimal('1E-6176'))
    # Table 77: odd(5) = true, odd(2) = false, even(5) = false, even(2) = true
    for op, a, want in [('odd', '5', True), ('odd', '2', False), ('even', '5', False), ('even', '2', True),
                        ('odd', '-3', True), ('even', '1E+6144', True), ('odd', '2.0', False), ('even', '20E-1', True),
                        ('odd', '1.5', False), ('even', '1.5', False), ('even', '-0', True), ('odd', '1E-6176', False)]:
        check(f'{op}({a})', evaluate([op, a]), want)
    check('odd(null)', evaluate(['odd', ['null']]), None)
    check('even("2")', evaluate(['even', ['str', '2']]), None)
    # a decimal scale loses its fractional part (Table 77, footnote 2)
    check('round up(1.121, 2.7)', evaluate(['round up', '1.121', '2.7']), Decimal('1.13'))
    check('decimal(1.125, 2.9)', evaluate(['decimal', '1.125', '2.9']), Decimal('1.12'))
    check('floor(1.5, -0.9)', evaluate(['floor', '1.5', '-0.9']), Decimal(1))
    check('floor(1.5, 6176.9)', evaluate(['floor', '1.5', '6176.9']), Decimal('1.5'))
    check('floor(1.5, -6111.9)', evaluate(['floor', '1.5', '-6111.9']), Decimal(0))
    check('floor(1.5, 6177.1)', evaluate(['floor', '1.5', '6177.1']), None)
    # Ties of a fractional power: s^2 ** 1.5 = s^3, 35 digits ending in 5, round half even, which
    # the interval rule decides only through the exact boundary.
    for s in (215443469005, 215443469015, 300000000005, 300000000015, 464158883345, 464158883355):
        check(f'tie power {s}', len(str(s ** 3)), 35)
        check(f'tie power {s}', evaluate(['pow', str(s * s), '1.5']), round_exact(Fraction(s) ** 3))
        check(f'tie power {s}E-3', evaluate(['pow', f'{s * s}E-6', '1.5']), round_exact(Fraction(s, 1000) ** 3))

    rng = random.Random(7)

    def operand(digits_max=34, exponent=40):
        c = rng.randrange(1, 10 ** rng.randint(1, digits_max))
        return Decimal((rng.randint(0, 1), tuple(map(int, str(c))), rng.randint(-exponent, exponent)))

    # modulo against the remainder of a wide context, then the divisor added when the signs differ
    wide = Context(prec=20000, Emax=10**6, Emin=-10**6, traps=[InvalidOperation])
    for i in range(10_000):
        a, b = operand(), operand()
        r = wide.remainder(a, b)
        if r != 0 and (r < 0) != (b < 0):
            r = wide.add(r, b)
        check(f'modulo({a}, {b})', modulo(a, b), finite(D34.plus(r)))
    # integer powers against repeated exact multiplication
    for i in range(300):
        a, n = operand(12, 5), rng.randint(-40, 40)
        if a == 0:
            continue
        f = Fraction(1)
        for _ in range(abs(n)):
            f *= Fraction(a)
        if n < 0:
            f = 1 / f
        check(f'{a} ** {n}', pow_(a, Decimal(n)), finite(round_exact(f)))
    # the interval rule on integer powers beyond the exact path agrees with the exact path
    for i in range(20):
        a, n = Decimal(1) + operand(12, 0) / 10 ** 14, rng.randint(2000, 5000)
        exact_value = finite(round_exact(Fraction(a) ** n))
        guard = 10 + _digits(Decimal(n)) * 2 + 10
        check(f'interval {a} ** {n}',
              finite(decide(lambda: mpmath.power(mpmath.mpf(str(a)), n), guard)), exact_value)
    # sqrt against isqrt on perfect squares and their neighbours
    for i in range(2000):
        k = rng.randrange(1, 10 ** 17)
        check(f'sqrt({k * k})', sqrt(Decimal(k * k)), Decimal(k))
        for m in (k * k - 1, k * k + 1):
            r = sqrt(Decimal(m))
            lower = math.isqrt(m)
            check(f'sqrt({m}) bracket', lower < r < lower + 1, True)

    if failures:
        print('\n'.join(failures[:40]))
        print(f'self-test: {len(failures)} failures')
        return 1
    print('self-test: all truths agree')
    return 0


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        sys.exit(_self_test())
    print('usage: oracle.py --self-test', file=sys.stderr)
    sys.exit(2)
