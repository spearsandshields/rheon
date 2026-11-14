"""The requests the number oracle sends, by family.

Each family has fixed edge vectors (always sent, in literal form: every operand is written in the
expression) and a random part drawn from random.Random(f'{seed}/{family}'): 200 cases per family
in a quick run, 20,000 in a full one. A random case sends its operands through the request context
as JSON numbers, which reach the engine from their exact text (CONTRACT §2).

  python3 -I cases.py <quick|full> [--seed N]   prints the requests, one per line

A case is {"family", "edge", "tree", "request"}; judge.py draws the same cases again and evaluates
each tree with oracle.evaluate.
"""
import json
import os
import random
import re
import sys
from decimal import Decimal

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle  # noqa: E402

SIZES = {'quick': 200, 'full': 20_000}
DEFAULT_SEED = 1

# --- rendering -------------------------------------------------------------------------------------

INFIX = {'add': '+', 'sub': '-', 'mul': '*', 'div': '/', 'pow': '**',
         'lt': '<', 'le': '<=', 'gt': '>', 'ge': '>=', 'eq': '='}
CALLS = {'abs', 'sqrt', 'exp', 'log', 'modulo', 'odd', 'even', 'decimal', 'floor', 'ceiling', 'round up',
         'round down', 'round half up', 'round half down', 'number', 'string', 'string length'}
JSON_NUMBER = re.compile(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?')
assert set(INFIX) | CALLS | {'neg', 'id'} == set(oracle.TRUTHS)


def render(tree, form, context):
    """The FEEL text of a tree. In the context form every number leaf becomes a variable of the
    request context (a, b, c, …), holding the leaf's text as a JSON number."""
    if isinstance(tree, str):
        if form == 'context':
            assert JSON_NUMBER.fullmatch(tree), tree
            context.append(tree)
            return 'abcdefgh'[len(context) - 1]
        return tree
    op, *args = tree
    if op == 'str':
        return json.dumps(args[0])
    if op == 'null':
        return 'null'
    if op == 'id':
        return render(args[0], form, context)

    def operand(arg):
        text = render(arg, form, context)
        nested = isinstance(arg, list) and (arg[0] in INFIX or arg[0] == 'neg')
        negative = isinstance(arg, str) and form == 'literal' and arg.startswith('-')
        return f'({text})' if nested or negative else text

    if op in INFIX:
        left = operand(args[0])
        return f'{left} {INFIX[op]} {operand(args[1])}'
    if op == 'neg':
        return f'-{operand(args[0])}'
    return f'{op}({", ".join(render(a, form, context) for a in args)})'


def request(tree, form):
    context = []
    expression = render(tree, form, context)
    text = '{"op":"evaluate","expression":' + json.dumps(expression)
    if context:
        text += ',"context":{' + ','.join(f'"{"abcdefgh"[i]}":{v}' for i, v in enumerate(context)) + '}'
    return text + '}'


# --- operands --------------------------------------------------------------------------------------

def dec(sign, coefficient, exponent):
    """The text of sign·coefficient·10^exponent, as Python's decimal writes it (valid JSON)."""
    return str(Decimal((sign, tuple(int(c) for c in str(coefficient)), exponent)))


def coefficient(rng, digits=None):
    digits = digits or rng.randint(1, 34)
    return rng.randrange(10 ** (digits - 1), 10 ** digits) if digits > 1 else rng.randint(1, 9)


def in_range(rng, digits, adjusted):
    """A random coefficient of the given digits with the given adjusted exponent, clamped into
    the range of decimal128."""
    c = coefficient(rng, digits)
    exponent = max(-6176, min(6144 - digits + 1, adjusted - digits + 1))
    return c, exponent


ZEROS = ['0', '-0', '0.00', '-0.00', '0E+6111', '-0E+6111', '0E-6176', '-0E-6176', '0E+10']
COHORTS = ['1', '1.0', '1.00', '1.00E+2', '100', '0.10', '0.1', '1.50', '2.50', '10', '1E+1',
           '0.0', '1.10', '5.000', '2.5E+3', '0.250']
EXTREMES = ['9.999999999999999999999999999999999E+6144', '1E-6143', '1E-6176',
            '9.99999999999999999999999999999999E-6144', '9999999999999999999999999999999999',
            '1E+6144', '1E+6111', '1234567890123456789012345678901234E+6111']


def pool(rng):
    """The shared operand pool: ordinary numbers, the whole exponent range with weight on its
    ends, zeros, cohorts of one value, and the extremes."""
    kind = rng.random()
    sign = rng.randint(0, 1)
    if kind < 0.45:
        return dec(sign, *in_range(rng, rng.randint(1, 34), rng.randint(-20, 20)))
    if kind < 0.65:
        return dec(sign, *in_range(rng, rng.randint(1, 34), rng.randint(-6176, 6144)))
    if kind < 0.75:
        end = rng.choice([-6176, -6143, 6111, 6144])
        return dec(sign, *in_range(rng, rng.randint(1, 34), end + rng.randint(-3, 3)))
    if kind < 0.85:
        return rng.choice(ZEROS)
    if kind < 0.93:
        text = rng.choice(COHORTS)
        return text if sign == 0 else '-' + text
    text = rng.choice(EXTREMES)
    return text if sign == 0 else '-' + text


def small(rng, digits=34, spread=20):
    return dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, digits), rng.randint(-spread, spread)))


def signed(rng, text):
    return text if text.startswith('-') or rng.randint(0, 1) == 0 else '-' + text


# --- families --------------------------------------------------------------------------------------

FAMILIES = []


def family(name, edges=(), random_part=None, edge_form='literal', random_form='context'):
    """A family: its fixed edge vectors (trees), a function rng -> tree for its random part, and
    the form of each."""
    FAMILIES.append((name, list(edges), random_part, edge_form, random_form))


def table_77_and_40():
    """The examples of DMN 1.6 Table 77 and Table 40, as written there."""
    rows = [['decimal', ['div', '1', '3'], '2'], ['decimal', '1.5', '0'], ['decimal', '2.5', '0'],
            ['floor', '1.5'], ['floor', '-1.56', '1'], ['ceiling', '1.5'], ['ceiling', '-1.56', '1']]
    for op in ('round up', 'round down', 'round half up', 'round half down'):
        rows += [[op, '5.5'], [op, '-5.5', '0'], [op, '1.121', '2'], [op, '-1.126', '2']]
    rows += [['abs', '10'], ['abs', '-10'], ['sqrt', '16'], ['log', '10'], ['exp', '5']]
    rows += [['modulo', a, b] for a, b in [('12', '5'), ('-12', '5'), ('12', '-5'), ('-12', '-5'),
                                           ('10.1', '4.5'), ('-10.1', '4.5'), ('10.1', '-4.5'),
                                           ('-10.1', '-4.5')]]
    rows += [['decimal', '1', '2'], ['add', '.25', '.2'], ['mul', '.10', '30.00'],
             ['sub', ['add', '1', ['mul', ['div', '3', '2'], '2']], ['pow', '2', '3']],
             ['div', '1', '3'], ['eq', '1', '1.000'], ['div', '1.01', '2'],
             ['decimal', '0.505', '2'], ['decimal', '0.515', '2'], ['mul', '1.0', ['pow', '10', '3']],
             ['id', '1.2e3'], ['string', '1.1'], ['number', '5'], ['number', ['str', '1.1']],
             ['number', ['str', '1 000,0'], ['str', ' '], ['str', ',']]]
    return rows


family('spec/examples', table_77_and_40())

# + - * /
MAX = '9.999999999999999999999999999999999E+6144'


def add_tie(rng):
    shift = rng.randint(-20, 20)
    x = coefficient(rng, 34)
    return dec(rng.randint(0, 1), x, shift), dec(rng.randint(0, 1), 5, shift - 1)


def mul_tie(rng):
    b = rng.choice([3, 5, 7, 9])
    while True:
        a = coefficient(rng, 33) * 10 + 5
        if a * b >= 10 ** 34:
            break
    return dec(rng.randint(0, 1), a, rng.randint(-20, 20)), dec(rng.randint(0, 1), b, rng.randint(-5, 5))


def div_tie(rng):
    b = rng.choice([2, 4, 8])
    r = rng.randrange(10 ** 33, (10 ** 35 // b) // 10) * 10 + 5
    a = r * b // 10
    shift = rng.randint(-20, 20)
    return dec(rng.randint(0, 1), a, shift + 1), dec(rng.randint(0, 1), b, 0)


def cancellation(rng, op):
    x = coefficient(rng, 34)
    e = rng.randint(-20, 20)
    k = rng.randint(-1000, 1000)
    y = x + k
    if not 0 < y < 10 ** 34:
        y = x
    sign = rng.randint(0, 1)
    other = 1 - sign if op == 'add' else sign
    return dec(sign, x, e), dec(other, y, e)


def overflow(rng, op):
    x = signed(rng, dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(6130, 6144))))
    if op == 'add':
        return x, x
    if op == 'sub':
        return x, x[1:] if x.startswith('-') else '-' + x
    if op == 'mul':
        return x, dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, 5), rng.randint(0, 30)))
    return x, dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, 5), rng.randint(-30, 0)))


def underflow(rng, op):
    x = signed(rng, dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(-6176, -6100))))
    if op in ('add', 'sub'):
        return x, signed(rng, dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(-6176, -6140))))
    if op == 'mul':
        return x, signed(rng, rng.choice(['0.5', '0.1', '1E-5', '0.3', '0.25', '1E-30']))
    return x, signed(rng, rng.choice(['2', '3', '10', '1E+10', '4', '7']))


def scale_limit(rng):
    x = dec(rng.randint(0, 1), coefficient(rng), rng.choice([6111, 6110, -6176, -6175]))
    y = rng.choice(['0', '1', '10', '1E+33', '1E-33', '0.1', '1E-6176', '1E+6111'])
    return x, signed(rng, y)


def cohort_pair(rng):
    return signed(rng, rng.choice(COHORTS)), signed(rng, rng.choice(COHORTS))


def zero_pair(rng):
    pair = [rng.choice(ZEROS), rng.choice(ZEROS + [pool(rng)])]
    rng.shuffle(pair)
    return tuple(pair)


ARITH_EDGES = {
    'add': {'tie': [('9999999999999999999999999999999999', '0.5'), ('9999999999999999999999999999999998', '0.5'),
                    ('1234567890123456789012345678901234', '0.5'), ('-1234567890123456789012345678901235', '-0.5')],
            'cancellation': [('1.10', '-1.10'), ('1E+34', '-9999999999999999999999999999999999')],
            'cohort': [('1.0', '1.00'), ('0.1', '0.2'), ('1.50', '2.50'), ('1.00E+2', '0')],
            'zero': [('0', '-0'), ('-0', '-0'), ('0.00', '-0'), ('0E+6111', '0E-6176')],
            'overflow': [(MAX, MAX), (MAX, '5E+6111'), (MAX, '4E+6110')],
            'underflow': [('1E-6176', '1E-6176'), ('1E-6143', '-1E-6176')],
            'scale-limit': [('1E+6144', '0'), ('1E-6176', '0E+6111'), ('1234567890123456789012345678901234E+6111', '1')]},
    'sub': {'tie': [('9999999999999999999999999999999999', '-0.5'), ('1000000000000000000000000000000000', '0.5')],
            'cancellation': [('1.10', '1.10'), ('1', '0.9999999999999999999999999999999999')],
            'cohort': [('1.10', '1.10'), ('2.50', '1.50'), ('1.00E+2', '100')],
            'zero': [('0', '0'), ('-0', '0'), ('0.00', '0.00')],
            'overflow': [(MAX, '-' + MAX)], 'underflow': [('2E-6176', '1E-6176'), ('1E-6143', '9E-6144')],
            'scale-limit': [('1E+6144', '1'), ('0E-6176', '1E-6176')]},
    'mul': {'tie': [('1000000000000000000000000000000005', '3'), ('9999999999999999999999999999999995', '9')],
            'cohort': [('1.0', '1.00'), ('0.25', '2'), ('.10', '30.00')],
            'zero': [('0', '-5'), ('-0', '5'), ('0', '1E-10'), ('0E+6111', '1E+6111'), ('0E-6176', '1E-6176')],
            'overflow': [('9E+6144', '10'), (MAX, MAX), ('1E+6144', '1.0000000000000000000000000000000001')],
            'underflow': [('1E-6176', '0.5'), ('3E-6176', '0.5'), ('1E-6176', '0.1'), ('1E-3088', '1E-3089'), ('1E-6143', '0.1')],
            'scale-limit': [('1E+6111', '1E+33'), ('12345E+6110', '10'), ('1E-6176', '1')]},
    'div': {'tie': [('2', '4'), ('20000000000000000000000000000000010', '4')],
            'cohort': [('1.0', '1'), ('1.00', '0.1'), ('1.01', '2'), ('100', '1.00E+2')],
            'zero': [('0', '-5'), ('-0', '5'), ('0.00', '3'), ('0E+6111', '1E-6176')],
            'overflow': [('1', '1E-6176'), (MAX, '0.1')],
            'underflow': [('1E-6176', '10'), ('1E-6176', '2'), ('3E-6176', '2'), ('5E-6176', '2'), ('1E-6143', '3')],
            'scale-limit': [('1E+6144', '1'), ('1', '3'), ('2', '3')]},
}

TIE = {'add': add_tie, 'sub': add_tie, 'mul': mul_tie, 'div': div_tie}
for op in ('add', 'sub', 'mul', 'div'):
    edges = ARITH_EDGES[op]

    def pair_family(name, pairs, gen, op=op):
        family(f'{op}/{name}', [[op, a, b] for a, b in pairs],
               (lambda rng: [op, *gen(rng)]) if gen else None)

    pair_family('random', [], lambda rng: (pool(rng), pool(rng)))
    pair_family('tie', edges['tie'], TIE[op])
    if op in ('add', 'sub'):
        pair_family('cancellation', edges['cancellation'], lambda rng, op=op: cancellation(rng, op))
    pair_family('cohort', edges['cohort'], cohort_pair)
    pair_family('zero', edges['zero'], zero_pair)
    pair_family('overflow', edges['overflow'], lambda rng, op=op: overflow(rng, op))
    pair_family('underflow', edges['underflow'], lambda rng, op=op: underflow(rng, op))
    pair_family('scale-limit', edges['scale-limit'], scale_limit)
family('div/zero-divisor', [['div', a, b] for a, b in [('1', '0'), ('-1', '-0'), ('0', '0'), ('5', '0.00'), (MAX, '0E+6111')]],
       lambda rng: ['div', pool(rng), rng.choice(ZEROS)])
family('arith/compound', [['div', '1', '3'], ['add', '0.1', '0.2'], ['div', '1E-6176', '10'],
                          ['mul', '9E+6144', '10'], ['div', '1', '1E-6176'], ['sub', '1.10', '1.10'],
                          ['mul', '0', '1E-10'], ['mul', '0', '-5'], ['div', '0', '-5'],
                          ['abs', '-1.50'], ['abs', ['mul', '0.25', '2']]],
       lambda rng: [rng.choice(['add', 'sub', 'mul', 'div']), [rng.choice(['add', 'sub', 'mul', 'div']), small(rng), small(rng)], small(rng)])

# unary minus, abs, comparisons
for op in ('neg', 'abs'):
    family(f'{op}/zero', [[op, z] for z in ZEROS], lambda rng, op=op: [op, rng.choice(ZEROS)])
    family(f'{op}/cohort', [[op, c] for c in ['1.50', '-1.50', '1.00E+2', '0.10']],
           lambda rng, op=op: [op, signed(rng, rng.choice(COHORTS))])
    family(f'{op}/random', [[op, e] for e in EXTREMES], lambda rng, op=op: [op, pool(rng)])
COMPARE = ['lt', 'le', 'gt', 'ge', 'eq']
family('compare/zero', [[op, a, b] for op in COMPARE for a, b in [('-0', '0'), ('0.00', '0E+6111'), ('-0', '0E-6176')]],
       lambda rng: [rng.choice(COMPARE), rng.choice(ZEROS), rng.choice(ZEROS)])
family('compare/cohort', [[op, a, b] for op in COMPARE for a, b in [('1', '1.0'), ('1.00E+2', '100'), ('0.10', '0.1')]],
       lambda rng: [rng.choice(COMPARE), signed(rng, rng.choice(COHORTS)), signed(rng, rng.choice(COHORTS))])
family('compare/random', [[op, a, b] for op in COMPARE for a, b in [(MAX, '1E+6144'), ('1E-6176', '0'), ('-1E-6176', '-0')]],
       lambda rng: [rng.choice(COMPARE), pool(rng), pool(rng)])

# the scale functions
SCALED = ['decimal', 'floor', 'ceiling', 'round up', 'round down', 'round half up', 'round half down']
BIG_SCALES = ['-6111', '6176', '-6112', '6177', '6145', '6150', '-6000', '2147483647', '2147483648',
              '-2147483648', '-2147483649', '9223372036854775807', '9223372036854775808',
              '-9223372036854775809', '1E+10', '-1E+10']


def scale_in_domain(rng):
    return str(rng.randint(-40, 40)) if rng.random() < 0.8 else str(rng.randint(-6111, 6176))


def tie_at(rng):
    s = rng.randint(-15, 15)
    k = coefficient(rng, rng.randint(1, 33))
    return dec(rng.randint(0, 1), k * 10 + 5, -s - 1), str(s)


def near_tie_at(rng):
    s = rng.randint(-10, 10)
    m = rng.randint(1, 30)
    k = coefficient(rng, rng.randint(1, 33 - m)) if m < 33 else 0
    c = (k * 10 + 5) * 10 ** m + rng.choice([-1, 1])
    return dec(rng.randint(0, 1), c, -s - 1 - m), str(s)


def to_zero_at(rng):
    s = rng.randint(-5, 5)
    digits = rng.randint(1, 34)
    return dec(1, coefficient(rng, digits), -s - digits - rng.randint(0, 3)), str(s)


def carry_at(rng):
    digits = rng.randint(1, 34)
    s = rng.randint(-5, 5)
    return dec(rng.randint(0, 1), 10 ** digits - 1, -s - rng.randint(1, digits)), str(s)


def fine_at(rng):
    c, e = in_range(rng, rng.randint(1, 34), rng.randint(-30, 6144))
    return dec(rng.randint(0, 1), c, e), str(rng.randint(max(-e, -6111), 6176))


def coarse_at(rng):
    return small(rng, 34, 10), str(rng.randint(-40, -1))


def limit_at(rng):
    return pool(rng), rng.choice(BIG_SCALES + [str(rng.randint(-6200, -6100)), str(rng.randint(6100, 6200))])


def subnormal_at(rng):
    return (dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, 33), rng.randint(-6176, -6144))),
            str(rng.randint(6100, 6176)))


for op in SCALED:
    name = op.replace(' ', '-')

    def scaled(kind, edges, gen, op=op, name=name):
        family(f'{name}/{kind}', [[op, n, s] for n, s in edges], lambda rng: [op, *gen(rng)])

    scaled('tie', [(n, s) for n in ['0.5', '1.5', '2.5', '-0.5', '-1.5', '-2.5'] for s in ['0']]
           + [('2.25', '1'), ('2.35', '1'), ('-2.25', '1'), ('-2.35', '1'), ('0.125', '2'), ('5E+2', '-3'),
              ('2.56', '0'), ('0.55', '0'), ('1.1251', '2'), ('-2.51', '0'), ('1234.5678', '0'), ('99.951', '1')],
           tie_at)
    scaled('near-tie', [('2.499999999999999999999999999999999', '0'), ('2.500000000000000000000000000000001', '0'),
                        ('-2.499999999999999999999999999999999', '0'), ('-2.500000000000000000000000000000001', '0'),
                        ('1.124999999999999999999999999999999', '2'), ('1.125000000000000000000000000000001', '2')],
           near_tie_at)
    scaled('zero', [(z, str(s)) for z in ZEROS for s in range(-5, 6)],
           lambda rng: (rng.choice(ZEROS), scale_in_domain(rng)))
    scaled('negative-to-zero', [(n, s) for n in ['-0.4', '-0.5', '-0.6', '-0.04', '-0.0000001', '-0.001']
                                for s in ['0', '1', '2']], to_zero_at)
    scaled('carry', [(n, s) for n in ['9.99', '99.95', '-9.99', '9.999999999999999999999999999999999',
                                      '999999999999999999999999999999999.9'] for s in ['0', '1']], carry_at)
    scaled('scale-fine', [('1E+30', '10'), ('1.5', '5'), ('123', '2'), ('1E-6176', '6176'), (MAX, '6176'),
                          ('1', '2'), ('0.5', '3'), ('-0.001', '2'), ('-1.56', '-6111')], fine_at)
    scaled('scale-coarse', [('1', '-3'), ('500', '-3'), ('1500', '-3'), ('-1', '-3'), ('499', '-3'),
                            ('501', '-3'), ('1E+6144', '-6111'), ('5E+6110', '-6111'), ('5.5', '-6111')], coarse_at)
    scaled('scale-limit', [(n, s) for n in ['1.5', '0.25', '5.5'] for s in BIG_SCALES]
           + [('1E-10', '6150'), ('1E-200', '-6000'), ('1E-6000', '-6000'), ('-1.56', '-6111')], limit_at)
    scaled('subnormal', [(n, s) for n in ['1E-6176', '5E-6176', '1.5E-6170', '-2.5E-6175',
                                          '9.99999999999999999999999999999999E-6144']
                         for s in ['6176', '6175', '6170', '6143', '6100', '0']], subnormal_at)
    scaled('scale-fraction', [('1.121', '2.7'), ('-1.126', '2.999'), ('1.129', '-0.5'), ('1.5', '0.5'),
                              ('2.5', '-0.9'), ('1500', '-3.5'), ('1.5', '6176.9'), ('1.5', '-6111.9'),
                              ('1.5', '6177.1'), ('1.5', '-6112.5'), ('1.5', '1E-6176')],
           lambda rng: (small(rng, 34, 10), str(rng.randint(-40, 40)) + '.' + str(rng.randint(1, 99))))
    scaled('random', [], lambda rng: (pool(rng), scale_in_domain(rng)))
    if op != 'decimal':
        family(f'{name}/one-argument',
               [[op, n] for n in ['5.5', '-5.5', '1.5', '2.5', '-0.5', '0.5', '1E+40', '1E-40', '-1E-40', '123.456',
                                  '9999999999999999999999999999999999', '0.9999999999999999999999999999999999',
                                  '-0.4', '-0.0', '0.05', '2.56', '-2.51', MAX, '1E-6176']],
               lambda rng, op=op: [op, pool(rng) if rng.random() < 0.5 else small(rng, 34, 3)])

# modulo
family('modulo/sign', [['modulo', a, b] for a, b in [('6', '3'), ('-6', '3'), ('6', '-3'), ('-6', '-3'),
                                                     ('0.6', '0.3'), ('-0.6', '-0.3'), ('7', '-2'), ('-7', '2')]],
       lambda rng: (lambda b, k: ['modulo', dec(rng.randint(0, 1), k * b, 0), dec(rng.randint(0, 1), b, 0)])(
           coefficient(rng, rng.randint(1, 16)), coefficient(rng, rng.randint(1, 17))))


def near_integer_quotient(rng):
    """a/b within 1/b·10^-m of an integer: a = b·k·10^m ± 1 in units of 10^(e-m)."""
    b = coefficient(rng, rng.randint(1, 20))
    q = b * coefficient(rng, rng.randint(1, 13))
    m = 34 - len(str(q))
    e = rng.randint(-20, 5)
    return ['modulo', dec(rng.randint(0, 1), q * 10 ** m + rng.choice([-1, 1]), e - m), dec(rng.randint(0, 1), b, e)]


family('modulo/quotient-crosses-integer', [['modulo', ['sub', '1', '2e-34'], ['div', '1', '3']],
                                           ['modulo', '0.9999999999999999999999999999999999', '0.3333333333333333333333333333333333']],
       near_integer_quotient)
family('modulo/huge-quotient', [['modulo', a, b] for a, b in [('1e40', '3'), ('1e40', '7'), ('1e34', '7'),
                                                              ('5e33', '0.7'), ('1e6000', '1e-6000'), ('1', '3e-35'),
                                                              (MAX, '7'), ('-1E+6144', '3')]],
       lambda rng: ['modulo', dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, 34), rng.randint(34, 6144))),
                    small(rng, 10, 5)])
family('modulo/underflow', [['modulo', a, b] for a, b in [('-1e-40', '1'), ('-1E-6176', '1E+6000'), ('-1E-6176', '1'),
                                                          ('1E-6176', '-1E+10')]],
       lambda rng: ['modulo', dec(rng.randint(0, 1), *in_range(rng, rng.randint(1, 34), rng.randint(-6176, -40))),
                    small(rng, 34, 30)])
family('modulo/zero-divisor', [['modulo', a, b] for a, b in [('5', '0'), ('0', '0'), ('5', '-0'), ('-1', '0.00')]],
       lambda rng: ['modulo', pool(rng), rng.choice(ZEROS)])
family('modulo/zero-dividend', [['modulo', a, b] for a, b in [('0', '5'), ('-0', '5'), ('0.00', '-3'), ('0E+6111', '1E-6176')]],
       lambda rng: ['modulo', rng.choice(ZEROS), pool(rng)])
family('modulo/random', [], lambda rng: ['modulo', pool(rng), pool(rng)])

# powers with an integer exponent


def growth_base(rng):
    k = rng.randint(1, 30)
    return dec(0, 10 ** k + rng.choice([1, -1]) * coefficient(rng, rng.randint(1, min(k, 6))), -k)


family('pow-int/exact', [['pow', a, b] for a, b in [('3', '50'), ('2', '100'), ('7', '40'), ('1.5', '10'), ('5', '50'),
                                                    ('2', '-50'), ('2', '-3'), ('0.5', '10'), ('-3', '33')]],
       lambda rng: ['pow', small(rng, 3, 3), str(rng.randint(-12, 12))])
family('pow-int/inexact', [['pow', '1.05', '360'], ['pow', ['add', '1', ['div', '0.05', '12']], '360'],
                           ['pow', '1.0025', '120'], ['pow', '1.000001', '1000000'], ['pow', '1.5', '200'],
                           ['pow', '1.0001', '1000000'], ['pow', '1.1', '100000'], ['pow', '1.1', '-100000'],
                           ['eq', ['pow', '1.1', '100000'], '1.855734417546823747428055637305764e4139']],
       lambda rng: ['pow', growth_base(rng) if rng.random() < 0.6 else small(rng, 34, 2),
                    str(rng.randint(2, 10 ** rng.randint(1, 7)))])
family('pow-int/negative', [['eq', ['pow', '2', '-20000'], ['pow', '0.5', '20000']], ['pow', '3', '-7'],
                            ['pow', '1.1', '-1000'], ['pow', '7', '-40']],
       lambda rng: ['pow', small(rng, 34, 2), str(-rng.randint(1, 10 ** rng.randint(1, 6)))])
family('pow-int/domain', [['pow', a, b] for a, b in [('1', '999999999'), ('-1', '999999999'), ('2', '999999999'),
                                                     ('10', '-999999999'), ('0.1', '999999999'), ('1', '1e10'),
                                                     ('-1', '1e10'), ('0.5', '1e10'), ('1', '1000000000'),
                                                     ('1', '-1000000000'), ('2', '1e10'), ('1.000000001', '999999999'),
                                                     ('1.000000001', '-999999999'), ('1.000000001', '1000000000'),
                                                     ('-1', '-1000000000'), ('0', '1000000000')]],
       lambda rng: ['pow', rng.choice(['1', '-1', '1.000000001', '0.999999999', '0', '2']),
                    str(rng.choice([1, -1]) * rng.choice([999_999_999, 1_000_000_000, 999_999_998,
                                                          rng.randint(10 ** 9, 10 ** 12)]))])
family('pow-int/zero-base', [['pow', a, b] for a, b in [('0', '0'), ('0', '-1'), ('-0', '3'), ('0', '5'),
                                                        ('0.00', '2'), ('-0', '-2'), ('0E+6111', '0')]],
       lambda rng: ['pow', rng.choice(ZEROS), str(rng.randint(-5, 5))])
family('pow-int/negative-base', [['pow', a, b] for a, b in [('-2', '3'), ('-2', '4'), ('-1.5', '7'), ('-1', '999999999'),
                                                            ('-1.1', '-1001')]],
       lambda rng: ['pow', '-' + small(rng, 34, 2).lstrip('-'), str(rng.randint(-200, 200))])
family('pow-int/overflow', [['pow', a, b] for a, b in [('10', '6145'), ('2', '30000'), ('9E+6144', '2'), ('-10', '6145')]],
       lambda rng: ['pow', small(rng, 34, 10), str(rng.randint(3000, 10 ** 6))])
family('pow-int/underflow', [['pow', a, b] for a, b in [('10', '-6177'), ('0.5', '30000'), ('1E-6176', '2'),
                                                        ('3E-3089', '2'), ('10', '-6176')]],
       lambda rng: ['pow', small(rng, 34, 10), str(-rng.randint(3000, 10 ** 6))])

# powers with a fractional exponent


def fraction(rng, digits, places):
    """A non-integer exponent: a random sign, the given digits, places of them after the point."""
    places = min(places, digits)
    c = coefficient(rng, digits)
    while c % 10 ** places == 0:
        c = coefficient(rng, digits)
    return dec(rng.randint(0, 1), c, -places)


def tie_root(rng):
    s = rng.randrange(215443469004 // 10 + 1, 464158883361 // 10) * 10 + 5
    return ['pow', str(s * s), '1.5']


family('pow-frac/random', [['pow', a, b] for a, b in [('2', '0.5'), ('5', '2.55'), ('12.2384283', '4.25')]],
       lambda rng: ['pow', small(rng, 34, 5).lstrip('-'),
                    fraction(rng, rng.randint(2, 8), rng.randint(1, 6))])
family('pow-frac/exact-root', [['pow', a, b] for a, b in [('4', '0.5'), ('0.25', '0.5'), ('1024', '0.1'),
                                                          (str(215443469005 ** 2), '1.5'), ('8', '0.333')]],
       lambda rng: tie_root(rng) if rng.random() < 0.5 else
       ['pow', str(coefficient(rng, rng.randint(1, 8)) ** 4), rng.choice(['0.5', '0.25', '0.75', '1.5', '2.5', '-0.5'])])
family('pow-frac/negative-base', [['pow', '-8', ['div', '1', '3']], ['pow', '-4', '0.5'], ['pow', '-1', '0.5']],
       lambda rng: ['pow', '-' + small(rng, 34, 5).lstrip('-'), fraction(rng, rng.randint(2, 34), rng.randint(1, 20))])
family('pow-frac/zero-base', [['pow', '0', '0.5'], ['pow', '0', '-0.5'], ['pow', '-0', '1.5']],
       lambda rng: ['pow', rng.choice(ZEROS), fraction(rng, 3, 2)])
family('pow-frac/overflow', [['pow', '1E+6144', '1.5'], ['pow', '10', '6144.5'], ['pow', '1E+3000', '2.5']],
       lambda rng: ['pow', small(rng, 34, 3000).lstrip('-'), fraction(rng, 4, 2).lstrip('-')])
family('pow-frac/underflow', [['pow', '1E-6176', '1.5'], ['pow', '10', '-6176.5'], ['pow', '1E-3000', '2.5']],
       lambda rng: ['pow', dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(-6176, -1000))),
                    fraction(rng, 4, 2).lstrip('-')])
family('pow-frac/domain', [['pow', a, b] for a, b in [('1', '1000000000.5'), ('1', '999999999.5'), ('2', '-1000000000.5'),
                                                      ('1', '1E+10')]],
       lambda rng: ['pow', rng.choice(['1', '1.000000001', '0.5', '2']),
                    dec(rng.randint(0, 1), rng.randint(99_999_999, 100_000_001) * 100 + 50, -2)])

# odd, even
PARITY = ['5', '2', '-3', '-4', '0', '1', '2.0', '1E+1', '20E-1', '1.50', '1.5', '0.5', '-0.5', '2.000000000000000000000000000000001',
          '9999999999999999999999999999999999', '9999999999999999999999999999999998', '1E+6144', '1E-6176', MAX]
for op in ('odd', 'even'):
    family(f'{op}/integer', [[op, x] for x in PARITY + ZEROS + COHORTS],
           lambda rng, op=op: [op, dec(rng.randint(0, 1), coefficient(rng, rng.randint(1, 34)), rng.randint(-3, 3))])
    family(f'{op}/not-a-number', [[op, ['null']], [op, ['str', '2']]])
    family(f'{op}/random', [[op, e] for e in EXTREMES], lambda rng, op=op: [op, pool(rng)])

# sqrt, exp, log
family('sqrt/perfect', [['sqrt', '16'], ['sqrt', '1E-6176'], ['sqrt', '1E+6144'], ['sqrt', '1.21']],
       lambda rng: (lambda k, j: ['sqrt', dec(0, k * k, 2 * j)])(coefficient(rng, rng.randint(1, 17)), rng.randint(-20, 20)))
family('sqrt/neighbour', [['sqrt', '15'], ['sqrt', '17']],
       lambda rng: (lambda k: ['sqrt', dec(0, k * k + rng.choice([-1, 1]), rng.randint(-20, 20))])(coefficient(rng, rng.randint(2, 17))))
family('sqrt/random', [['sqrt', '2'], ['sqrt', '10'], ['sqrt', MAX], ['sqrt', '1E+6111']],
       lambda rng: ['sqrt', pool(rng).lstrip('-')])
family('sqrt/negative', [['sqrt', '-1'], ['sqrt', '-1E-6176']], lambda rng: ['sqrt', '-' + pool(rng).lstrip('-')])
family('sqrt/zero', [['sqrt', z] for z in ZEROS] + [['sqrt', '-0.0']], lambda rng: ['sqrt', rng.choice(ZEROS)])
family('sqrt/subnormal', [['sqrt', '1E-6176'], ['sqrt', '2E-6176'], ['sqrt', '9.99999999999999999999999999999999E-6144']],
       lambda rng: ['sqrt', dec(0, *in_range(rng, rng.randint(1, 33), rng.randint(-6176, -6144)))])
family('sqrt/largest', [['sqrt', MAX], ['sqrt', '9E+6144']],
       lambda rng: ['sqrt', dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(6100, 6144)))])

def exp_in_range(rng):
    """exp of a number in [-14250, 14150] with up to 20 fraction digits."""
    k = rng.randint(0, 20)
    c = rng.randint(-14250 * 10 ** k, 14150 * 10 ** k)
    return ['exp', dec(int(c < 0), abs(c), -k)]


EXP_EDGES = ['14149.38539644841072829055748903541', '14149.38539644841072829055748903542', '14149', '14150',
             '-14144.65580082753', '-14220.76553433122614449511522413063', '-14221.4', '-14222', '-14250', '14300']
family('exp/random', [['exp', '1'], ['exp', ['log', '2.5']], ['abs', ['exp', '1e-34']], ['exp', '74.971090'],
                      ['exp', '-0.6931471805599453094172321214581766']],
       lambda rng: ['exp', dec(rng.randint(0, 1), coefficient(rng), rng.randint(-34, 0))]
       if rng.random() < 0.5 else exp_in_range(rng))
family('exp/threshold', [['exp', x] for x in EXP_EDGES],
       lambda rng: ['exp', dec(rng.randint(0, 1), rng.randint(14100 * 10 ** 20, 14250 * 10 ** 20), -20)])
family('exp/zero', [['exp', z] for z in ZEROS], lambda rng: ['exp', rng.choice(ZEROS)])
family('exp/tiny', [['exp', x] for x in ['1e-40', '1E-6176', '-1E-6176', '1E-34', '-1E-35', '5E-35', '-5E-35']],
       lambda rng: ['exp', dec(rng.randint(0, 1), coefficient(rng), -rng.randint(34, 6176))])
family('log/random', [['log', '10'], ['log', ['exp', '7']], ['log', '1e30'], ['log', '2']],
       lambda rng: ['log', pool(rng).lstrip('-')])
family('log/near-one', [['log', x] for x in ['1.000000000000000000000000000000001', '0.9999999999999999999999999999999999',
                                             '1.000000000000000000000000000000002', '0.9999999999999999999999999999999998',
                                             '1.0000001']],
       lambda rng: (lambda k: ['log', dec(0, 10 ** k + rng.choice([1, -1]) * rng.randint(1, 10 ** 6), -k)])(rng.randint(28, 33)))
family('log/zero', [['log', z] for z in ZEROS], lambda rng: ['log', rng.choice(ZEROS)])
family('log/negative', [['log', '-1'], ['log', '-1E-6176']], lambda rng: ['log', '-' + pool(rng).lstrip('-')])
family('log/one', [['log', x] for x in ['1', '1.0', '1.00E+0', '1.000']], lambda rng: ['log', rng.choice(['1', '1.0', '1.00'])])
family('log/largest', [['log', MAX], ['log', '1E+6144']],
       lambda rng: ['log', dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(6000, 6144)))])
family('log/smallest-subnormal', [['log', '1E-6176'], ['log', '2E-6176'], ['log', '1E-6143']],
       lambda rng: ['log', dec(0, *in_range(rng, rng.randint(1, 34), rng.randint(-6176, -6100)))])

# number()


def text_number(rng):
    return dec(rng.randint(0, 1), coefficient(rng), rng.randint(-40, 40))


family('number/literal-forms', [['number', ['str', s]] for s in ['1', '-1', '1.5', '.5', '-.5', '1e3', '1E3', '1e+3', '1e-3',
                                                               '-1.5E-3', '007', '0.000', '1.10', '1.2e3']],
       lambda rng: ['number', ['str', rng.choice([text_number(rng), text_number(rng).lower(), '0' * rng.randint(1, 3) + str(coefficient(rng))])]])
family('number/long', [['number', ['str', '1' * 40]], ['number', ['str', '1.' + '0' * 33 + '5']],
                       ['number', ['str', '1.' + '0' * 32 + '15']], ['number', ['str', '2.' + '0' * 32 + '25']]],
       lambda rng: ['number', ['str', str(coefficient(rng, 34)) + str(rng.randrange(10 ** 25))]])
family('number/overflow', [['number', ['str', s]] for s in ['1e6145', '1E+9999', '9.9999999999999999999999999999999999E+6144']],
       lambda rng: ['number', ['str', f'{coefficient(rng)}E+{rng.randint(6145, 9000)}']])
family('number/underflow', [['number', ['str', s]] for s in ['1e-6177', '1.5e-6176', '5E-6177', '1E-9999']],
       lambda rng: ['number', ['str', f'{coefficient(rng)}E-{rng.randint(6150, 6220)}']])
family('number/separators', [['number', ['str', a], ['str', g] if g else ['null'], ['str', d] if d else ['null']]
                             for a, g, d in [('1 000 000.00', ' ', '.'), ('1,000.5', ',', '.'), ('1.000,5', '.', ','),
                                             ('1.000.000,01', '.', ','), ('1 000,0', ' ', ','), ('1,5', None, ','),
                                             ('1.000', '.', None), ('1.5', '.', '.'), ('1,5', ',', ','), ('1;5', ';', None),
                                             ('1.5', None, None), ('1x5', None, 'x')]],
       lambda rng: (lambda g, d: ['number', ['str', f'{coefficient(rng, rng.randint(1, 9)):,}'.replace(',', g or '') + (d or '.') + str(rng.randint(0, 99))],
                                  ['str', g] if g else ['null'], ['str', d] if d else ['null']])(
           rng.choice([' ', ',', '.', None]), rng.choice(['.', ',', None])))
family('number/invalid', [['number', ['str', s]] for s in ['', 'abc', '1.2.3', 'NaN', 'INF', '0x10', '1e', '--1', '1.', 'e5', '-']],
       lambda rng: ['number', ['str', rng.choice(['', 'a', '1.2.3', '1e', '--1', '+-1', '1..2', 'Infinity']) + str(rng.randint(0, 9))]])
family('number/number-argument', [['number', n] for n in ['5', '1.50', '-0', MAX]], lambda rng: ['number', pool(rng)])

# string()
family('string/integers', [['string', n] for n in ['0', '123', '-45', '1E+3', '10']], lambda rng: ['string', str(coefficient(rng))])
family('string/fractions', [['string', n] for n in ['1.1', '0.5', '-0.25', '.5']], lambda rng: ['string', small(rng)])
family('string/trailing-zeros', [['string', '1.10'], ['string', '1.00'], ['string', '0.000'], ['string', '100.0'],
                                 ['eq', ['string', '1.50'], ['str', '1.5']], ['string length', ['string', '1.00']]],
       lambda rng: ['string', signed(rng, rng.choice(COHORTS))])
family('string/zero', [['string', z] for z in ZEROS] + [['eq', ['string', '-0'], ['str', '0']]],
       lambda rng: ['string', rng.choice(ZEROS)])
family('string/exponents', [['string', f'1E{s}{k}'] for s in '+-' for k in (1, 6, 7, 20, 40)],
       lambda rng: ['string', dec(rng.randint(0, 1), coefficient(rng), rng.randint(-40, 40))])
family('string/extremes', [['string', n] for n in ['1E+6144', '1E-6176', MAX, '1234567890123456789012345678901234',
                                                   '0.1234567890123456789012345678901234']],
       lambda rng: ['string', pool(rng)])
family('string/round-trip', [['eq', ['number', ['string', n]], n] for n in ['1.1', '1E+6144', '1E-6176', '-0', '0.00']],
       lambda rng: (lambda n: ['eq', ['number', ['string', n]], n])(pool(rng)))
family('string/length', [['string length', ['string', n]] for n in ['1.00', '-0', '1E+3', '0.10']],
       lambda rng: ['string length', ['string', pool(rng)]])

# literals and JSON numbers: the rounding of an input beyond 34 digits, the range ends


def long_number(rng, top=6144 + 30):
    """1 to 60 digits, adjusted exponents from 30 below the smallest subnormal to top."""
    digits = rng.randint(1, 60)
    return dec(rng.randint(0, 1), coefficient(rng, digits), rng.randint(-6176 - 30, top - digits + 1))


LITERALS = ['-0', '1.50', '1.0', '0.000', '1.2e3', '1' * 40, '1.' + '0' * 33 + '5', '1.' + '0' * 32 + '25', '1E+6144',
            '1E+6145', '1.5E-6176', '5E-6177', '9.9999999999999999999999999999999995E+6144']
family('literal/digits', [['id', x] for x in LITERALS], lambda rng: ['id', long_number(rng)],
       random_form='literal')
# A JSON number too large for decimal128 is a context error (CONTRACT §2), not a number result.
family('json-number/digits', [['id', x] for x in ['-0', '1.50', '1' * 40, '1.5E-6176', '5E-6177', '0.00']],
       lambda rng: ['id', long_number(rng, 6143)], edge_form='context')


def cases(size, seed):
    """Every case, edges first within a family, in family order; a request is sent once."""
    seen = set()
    for name, edges, generate, edge_form, random_form in FAMILIES:
        vectors = [(tree, edge_form, True) for tree in edges]
        if generate is not None:
            rng = random.Random(f'{seed}/{name}')
            vectors += [(generate(rng), random_form, False) for _ in range(SIZES[size])]
        for tree, form, edge in vectors:
            text = request(tree, form)
            if text in seen:
                continue
            seen.add(text)
            yield {'family': name, 'edge': edge, 'tree': tree, 'request': text}


def parse_arguments(argv, usage):
    """<quick|full> [--seed N] -> (size, seed); exits with status 2 on anything else."""
    seed = DEFAULT_SEED
    if len(argv) == 3 and argv[1] == '--seed' and argv[2].isdigit():
        seed = int(argv[2])
        argv = argv[:1]
    if len(argv) != 1 or argv[0] not in SIZES:
        print(f'usage: {usage}', file=sys.stderr)
        sys.exit(2)
    return argv[0], seed


def main(argv):
    size, seed = parse_arguments(argv, 'cases.py <quick|full> [--seed N]')
    for case in cases(size, seed):
        sys.stdout.write(case['request'] + '\n')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
