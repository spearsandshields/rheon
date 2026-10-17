"""Round-4 judge: oracle.py for single-op mismatches; for chain/chain_trans, find the first
divergent step (both backends agree on everything before it) and judge that step alone.
Usage: python3 judge4.py mismatches.tsv [examples_per_bucket]"""
import sys, os, re, collections, multiprocessing
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle
from decimal import Decimal

STEP = {0: 'add', 1: 'sub', 2: 'mul', 3: 'div', 9: 'sqrt', 10: 'square', 12: 'floor', 13: 'ceiling', 14: 'round_half_up', 17: 'exp', 18: 'ln', 19: 'pow'}
NAMES = {4: 'rem', 5: 'neg', 6: 'abs', 7: 'trunc', 8: 'frac', 11: 'round(int)', 15: 'pow(x, small int)', 16: 'add(new(..))', 20: 'pow(x, n/10)'}

def dec_of(s):
    m = re.fullmatch(r'([+-])(\d+)E([+-]?\d+)', s)
    return m.group(1) + m.group(2) + 'E' + m.group(3) if m else None

RX = re.compile(r'#(\d+)(?:\[([^;]*);(-?\d+);(-?\d+);(-?\d+)\])?:')

def judge_chain(f):
    op, cat, a, b, k, idisp, fdisp, idbg, fdbg = f[:9]
    I, F = idbg.split(' -> '), fdbg.split(' -> ')
    j = next(j for j in range(max(len(I), len(F))) if (I[j] if j < len(I) else None) != (F[j] if j < len(F) else None))
    prev = I[j - 1].split(':', 1)[-1] if j else a
    mi = RX.match(I[j]) if j < len(I) else None
    mf = RX.match(F[j]) if j < len(F) else None
    m = mi if (mi and mi.group(2) is not None) else (mf or mi)
    o = int(m.group(1))
    y, sc, pw, ps = (m.group(2), int(m.group(3)), int(m.group(4)), int(m.group(5))) if m.group(2) is not None else ('', 0, 0, 0)
    val = lambda L: L[j].split(':', 1)[1] if j < len(L) else '-'
    iv, fv = val(I), val(F)
    iv = 'PANIC' if iv.startswith('PANIC') else iv
    fv = 'PANIC' if fv.startswith('PANIC') else fv
    x = dec_of(prev) or prev
    if o in STEP:
        sub = STEP[o]; yy = (dec_of(y) or y) if sub in ('add', 'sub', 'mul', 'div', 'pow') else ''
    elif o == 15: sub, yy = 'pow', '%d' % pw
    elif o == 16: sub, yy = 'add', '%dE%d' % (pw, -ps)
    elif o == 20: sub, yy = 'pow', '%dE-1' % pw
    elif o == 11: sub, yy = 'round_quantize', '%d' % sc
    else:
        return (op + '/' + NAMES[o], 'no oracle for this step'), f'{op} step{j} {NAMES[o]} x={x} y={y} | intel={iv} | dashu={fv}', None
    if m.group(2) is None:
        return (op + '/' + sub, 'operand not recorded (None/Err step)'), f'{op} step{j} {sub} x={x} | intel={iv} | dashu={fv}', None
    line = '\t'.join([sub, '', x, yy, str(sc), '', '', iv, fv])
    (_, verdict), exl, _ = oracle.judge(line)
    name = {15: 'pow(x,int)', 20: 'pow(x,n/10)', 16: 'add(new)'}.get(o, sub)
    return (op + '/' + name, verdict), f'{op} step{j} {name}: {exl}', None

def judge(line):
    f = line.rstrip('\n').split('\t')
    if f[0].startswith('chain'):
        return judge_chain(f)
    return oracle.judge(line)

def main():
    path = sys.argv[1]; nex = int(sys.argv[2]) if len(sys.argv) > 2 else 3
    stats = collections.Counter(); ex = collections.defaultdict(list)
    with multiprocessing.Pool(8) as pool:
        for r in pool.imap(judge, open(path).readlines(), chunksize=200):
            if r is None: continue
            key, exl, _ = r
            stats[key] += 1
            if len(ex[key]) < nex: ex[key].append(exl)
    for (op, v), n in sorted(stats.items()):
        print('%-28s %8d  %s' % (op, n, v))
    print()
    for key, ls in sorted(ex.items()):
        print('==', *key)
        for l in ls: print('    ', l)

if __name__ == '__main__':
    main()
