"""For every Intel-vs-dashu display mismatch, evaluate the expression with mpmath (120 digits),
rounding every intermediate to decimal128 (34 digits, half-even) like FEEL, and report which backend
matches the correctly rounded result. Handles exp/log/sqrt/**,/ and `for i in a..b return f(i)` lists."""
import sys, json, ast, re, mpmath, decimal
sys.set_int_max_str_digits(0)
from decimal import Decimal, Context, ROUND_HALF_EVEN
mpmath.mp.dps = 120
D34 = Context(prec=34, rounding=ROUND_HALF_EVEN, Emax=6144, Emin=-6143, clamp=1)
def r(x):  # mpf -> correctly rounded Decimal128
    return D34.create_decimal(mpmath.nstr(x, 110, strip_zeros=False, min_fixed=1, max_fixed=0))
def m(d): return mpmath.mpf(str(d))
class Ev(ast.NodeVisitor):
    def __init__(s, env): s.env = env
    def visit_Expression(s, n): return s.visit(n.body)
    def visit_Constant(s, n): return D34.create_decimal(s.src[n.col_offset:n.end_col_offset]) if True else None
    def visit_Name(s, n): return s.env[n.id]
    def visit_UnaryOp(s, n):
        v = s.visit(n.operand); return -v if isinstance(n.op, ast.USub) else v
    def visit_BinOp(s, n):
        a, b = s.visit(n.left), s.visit(n.right)
        if isinstance(n.op, ast.Div): return r(m(a) / m(b))
        if isinstance(n.op, ast.Mult): return r(m(a) * m(b))
        if isinstance(n.op, ast.Pow): return r(mpmath.power(m(a), m(b)))
        raise ValueError
    def visit_Call(s, n):
        f = n.func.id; v = s.visit(n.args[0])
        return r({'exp': mpmath.exp, 'log': mpmath.log, 'sqrt': mpmath.sqrt}[f](m(v)))
def ev(expr, env={}):
    e = Ev(env); e.src = expr
    return e.visit(ast.parse(expr, mode='eval'))
def evaluate(expr):
    expr = expr.strip()
    mm = re.fullmatch(r'string\((.*)\)', expr)
    if mm: return evaluate(mm.group(1))
    mm = re.fullmatch(r'for i in (-?\d+)\.\.(-?\d+) return (.*)', expr)
    if mm:
        return [ev(mm.group(3), {'i': Decimal(i)}) for i in range(int(mm.group(1)), int(mm.group(2)) + 1)]
    return ev(expr)
def nums(s):
    return [Decimal(t) for t in re.findall(r'-?\d+(?:\.\d+)?(?:E[+-]?\d+)?', s)]
A = [json.loads(l) for l in open(sys.argv[1])]; B = [json.loads(l) for l in open(sys.argv[2])]
tot = fast_ok = intel_ok = both_bad = unk = 0
for x, y in zip(A, B):
    if x['display'] == y['display']: continue
    tot += 1
    try:
        t = evaluate(x['expr']); t = t if isinstance(t, list) else [t]
    except Exception as e:
        print('UNVERIFIED', x['expr'], e); unk += 1; continue
    i, f = nums(x['display']), nums(y['display'])
    fi = all(a == b for a, b in zip(i, t)) and len(i) == len(t)
    ff = all(a == b for a, b in zip(f, t)) and len(f) == len(t)
    if ff and not fi: fast_ok += 1; tag = 'dashu=correct'
    elif fi and not ff: intel_ok += 1; tag = 'INTEL=correct'
    else: both_bad += 1; tag = 'NEITHER'
    # ulp distance of first differing element
    for a, b, c in zip(i, f, t):
        if a != b:
            ulp = Decimal(1).scaleb(c.adjusted() - 33)
            print(f"{tag:16} {x['expr'][:60]:60} intel_err={float(abs(a - c) / ulp):.0f}ulp dashu_err={float(abs(b - c) / ulp):.0f}ulp")
            break
print(f"mismatches={tot} dashu_correct_only={fast_ok} intel_correct_only={intel_ok} neither={both_bad} unverified={unk}")
