"""Judges every Display mismatch in mismatches.tsv against an independent oracle.

Oracle = Python `decimal` with an IEEE decimal128 context (34 digits, HALF_EVEN,
Emax=6144, Emin=-6143, clamp=1) for arithmetic/exp/ln/sqrt/quantize/scaleb, and
mpmath (150 digits, then rounded once into decimal128) for pow.  For the scale
ops (ceiling/floor/round_*) the oracle evaluates dsntk's own algorithm
(validate ilogb+scale, scale by 10^s, round to integral, unscale) in exact
arithmetic, so it measures backend correctness, not FEEL-spec conformance.

Usage: python3 oracle.py mismatches.tsv [max_examples_per_bucket]
"""
import sys, collections, re
import decimal
from decimal import Decimal, Context, ROUND_HALF_EVEN, ROUND_CEILING, ROUND_FLOOR, ROUND_DOWN, ROUND_HALF_UP
import mpmath
sys.set_int_max_str_digits(0)

mpmath.mp.dps = 150
D34 = Context(prec=34, rounding=ROUND_HALF_EVEN, Emax=6144, Emin=-6143, clamp=1, traps=[])
BIG = Context(prec=20000, rounding=ROUND_HALF_EVEN, Emax=decimal.MAX_EMAX, Emin=decimal.MIN_EMIN, traps=[])
I32 = (-2**31, 2**31 - 1)

NONE = 'None'  # non-finite / null


def lit(s):
    """dsntk parse: bid128_from_string; returns D34 value or None if invalid/non-finite."""
    s = s.replace('\\t', '\t').lstrip(' \t')
    m = re.match(r'^([+-]?)(\d*)(?:\.(\d*))?(?:[eE]([+-]?\d+))?', s)
    if not m or (m.group(2) == '' and (m.group(3) or '') == '' and s[:1] != '.'):
        pass
    try:
        d = D34.create_decimal(s.strip())
    except Exception:
        return None
    if not d.is_finite():
        return None
    return d


def out(s):
    """Parse a backend Debug output: '+123E-5' -> Decimal; None/Err/PANIC -> tag."""
    s = s.strip()
    m = re.fullmatch(r'([+-])(\d+)E([+-]?\d+)', s)
    if m:
        return Decimal(m.group(1) + m.group(2) + 'E' + m.group(3))
    if s in ('+Inf', '-Inf'):
        return 'Inf'
    if 'NaN' in s:
        return 'NaN'
    if s.startswith('Err'):
        return 'Err'
    if s.startswith('PANIC'):
        return 'PANIC'
    if s == 'None' or s == 'null':
        return NONE
    return s


def fin_or_none(d):
    return d if isinstance(d, Decimal) and d.is_finite() else NONE


def pow_truth(x, y):
    if y.is_zero():
        return Decimal(1)
    if x == 1:
        return Decimal(1)
    yint = y == y.to_integral_value()
    if x.is_zero():
        odd = yint and (int(y) % 2 == 1)
        neg = x.is_signed() and odd
        if y < 0:
            return NONE  # Inf
        return Decimal('-0') if neg else Decimal('0')
    if x < 0 and not yint:
        return NONE
    mx, my = mpmath.mpf(str(x)), mpmath.mpf(str(y))
    mag = mpmath.log10(abs(mx)) * my
    if mag > 6200:
        return NONE  # overflow -> Inf -> FeelNumber::pow returns None
    if mag < -6300:
        return Decimal('-0') if (x < 0 and int(y) % 2 == 1) else Decimal('0')  # underflow
    r = mpmath.power(abs(mx), my)
    if x < 0 and int(y) % 2 == 1:
        r = -r
    if not mpmath.isfinite(r):
        return NONE
    # mpmath exponent range is unbounded: format and let D34 round/overflow/underflow
    d = D34.create_decimal(mpmath.nstr(r, 120, strip_zeros=False, min_fixed=1, max_fixed=0))
    return fin_or_none(d)


def scale_truth(op, x, s):
    ilogb = x.adjusted() if not x.is_zero() else -2**31
    def integral(y):
        if op == 'ceiling':
            return y.to_integral_value(rounding=ROUND_CEILING)
        if op == 'floor':
            return y.to_integral_value(rounding=ROUND_FLOOR)
        if op == 'round_down':
            return y.to_integral_value(rounding=ROUND_DOWN)
        if op == 'round_up':
            return y.to_integral_value(rounding=ROUND_FLOOR if y.is_signed() else ROUND_CEILING)
        if op == 'round_half_up':
            return y.to_integral_value(rounding=ROUND_HALF_UP)
        if op == 'round_half_down':
            t = D34.add(y, Decimal('0.1')) if y.is_signed() else D34.subtract(y, Decimal('0.1'))
            return t.to_integral_value(rounding=ROUND_HALF_UP)
    if s == 0:
        r = integral(x)
        if op == 'ceiling' and r.is_zero():
            r = Decimal(0)
        return r
    t = ilogb + s
    if not (I32[0] <= t <= I32[1]):
        return 'PANIC'
    if not (-6111 <= t <= 6144):
        return 'Err'
    y = BIG.scaleb(x, s)
    r = integral(y)
    if r.is_zero():
        return Decimal(0)
    if r == 1 or r == -1:
        return r
    return fin_or_none(D34.create_decimal(BIG.scaleb(r, -s)))


def truth(op, a, b, k):
    if op == 'new':
        # FeelNumber::new(n, s) is scalbn(n, -s) and bid128_scalbn clamps the scale to -6176..=6144
        # (the Intel backend panics on -s for s = i32::MIN in a debug build; the core saturates)
        return fin_or_none(D34.create_decimal(Decimal('%dE%d' % (k, max(-6176, min(6144, -int(b)))))))
    x = lit(a)
    if x is None:
        return '?'
    y = lit(b) if b else None
    if op == 'add': return fin_or_none(D34.add(x, y))
    if op == 'sub': return fin_or_none(D34.subtract(x, y))
    if op == 'mul': return fin_or_none(D34.multiply(x, y))
    if op == 'div': return fin_or_none(D34.divide(x, y))
    if op == 'sqrt': return fin_or_none(D34.sqrt(x))
    if op == 'square': return fin_or_none(D34.multiply(x, x))
    if op == 'exp': return fin_or_none(D34.exp(x))
    if op == 'ln': return fin_or_none(D34.ln(x)) if x > 0 else NONE
    if op == 'feel_log': return fin_or_none(D34.ln(x)) if x > 0 else NONE
    if op in ('pow', 'pow_int', 'pow_frac', 'pow_special'): return pow_truth(x, y)
    if op in ('ceiling', 'floor', 'round_down', 'round_up', 'round_half_up', 'round_half_down'):
        return scale_truth(op, x, k)
    if op in ('round_quantize', 'feel_decimal'):
        sc = y.to_integral_value(rounding=ROUND_DOWN)
        if op == 'feel_decimal' and not (-6111 <= sc < 6176):
            return 'null'
        if not (-6176 <= -sc <= 6111):
            return 'out-of-range quantum'
        return fin_or_none(D34.quantize(x, Decimal('1E%d' % -int(sc))))
    return '?'


def same(v, t):
    if isinstance(v, Decimal) and isinstance(t, Decimal):
        return v == t and (v.is_signed() == t.is_signed() or not v.is_zero())
    if t == NONE:
        return v in (NONE, 'Inf', 'NaN')
    if t == 'PANIC':
        # `ilogb + scale` overflows i32 (zero has ilogb i32::MIN): the Intel backend panics in a
        # debug build (wraps in release and then reports the invalid scale); the decimal128 core
        # validates the scale in i64 and reports the invalid scale, which is the non-panicking
        # equivalent of the release behaviour.
        return v in ('PANIC', 'Err')
    return v == t


def ulps(v, t):
    if not (isinstance(v, Decimal) and isinstance(t, Decimal)):
        return 'special: %s vs truth %s' % (v if not isinstance(v, Decimal) else 'finite', t if not isinstance(t, Decimal) else 'finite')
    if t.is_zero() or v.is_zero():
        return 'gross: zero vs nonzero' if not (t.is_zero() and v.is_zero()) else 'signed zero'
    u = Decimal(1).scaleb(max(t.adjusted() - 33, -6176))
    C60 = Context(prec=60, Emax=decimal.MAX_EMAX, Emin=decimal.MIN_EMIN, traps=[])
    d = C60.divide(abs(C60.subtract(v, t)), u)
    if d <= 1:
        return '<=1 ulp'
    if d < 10:
        return '2-9 ulp'
    return 'gross: >=10 ulp'


def judge(line):
    f = line.rstrip('\n').split('\t')
    if len(f) < 9:
        return None
    op, cat, a, b, k, idisp, fdisp, idbg, fdbg = f[:9]
    k = int(k)
    try:
        t = truth(op, a, b, k)
    except Exception as e:
        t = '?' + type(e).__name__
    iv, fv = out(idbg), out(fdbg)
    if isinstance(t, str) and (t.startswith('?') or t == 'out-of-range quantum'):
        verdict = 'no oracle (%s)' % t
    else:
        io, fo = same(iv, t), same(fv, t)
        if io and fo:
            verdict = 'both correct value (cohort/format only)'
        elif fo:
            verdict = 'dashu correct, intel wrong (%s)' % ulps(iv, t)
        elif io:
            verdict = 'INTEL correct, DASHU WRONG (%s)' % ulps(fv, t)
        else:
            verdict = 'both wrong (intel %s, dashu %s)' % (ulps(iv, t), ulps(fv, t))
    exl = 'a=%r b=%r k=%s | truth=%s | intel=%s | dashu=%s' % (a[:60], b[:60], k, str(t)[:60], idbg[:60], fdbg[:60])
    return (op, verdict), exl, '\t'.join([op, verdict, a, b, str(k), idbg, fdbg, str(t)])


def main():
    import multiprocessing
    path = sys.argv[1]
    nex = int(sys.argv[2]) if len(sys.argv) > 2 else 3
    stats = collections.Counter()
    ex = collections.defaultdict(list)
    lines = open(path).readlines()
    with multiprocessing.Pool(8) as pool, open(path + '.judged', 'w') as jf:
        for r in pool.imap(judge, lines, chunksize=200):
            if r is None:
                continue
            key, exl, row = r
            jf.write(row + '\n')
            stats[key] += 1
            if len(ex[key]) < nex:
                ex[key].append(exl)
    for (op, v), n in sorted(stats.items()):
        print('%-16s %8d  %s' % (op, n, v))
    print()
    for key, lines in sorted(ex.items()):
        print('==', *key)
        for l in lines:
            print('    ', l)


if __name__ == '__main__':
    main()
