#!/usr/bin/env python3
"""Harvest FEEL expressions from dsntk feel-evaluator unit tests and bbt .feel/.ctx files.
Output: JSON lines {"src":..., "ctx":<context-string or null>, "expr":...}"""
import os, re, sys, json, glob

ROOT = sys.argv[1]  # dsntk checkout
out = []

def lex_string(s, i):
    """s[i] starts a rust string literal (", r", r#", b"?). Return (value, end) or None."""
    if s[i] == 'r':
        j = i + 1; h = 0
        while j < len(s) and s[j] == '#': h += 1; j += 1
        if j >= len(s) or s[j] != '"': return None
        term = '"' + '#' * h
        k = s.find(term, j + 1)
        return s[j+1:k], k + len(term)
    if s[i] != '"': return None
    j = i + 1; buf = []
    while True:
        c = s[j]
        if c == '\\':
            n = s[j+1]
            if n == 'n': buf.append('\n'); j += 2
            elif n == 't': buf.append('\t'); j += 2
            elif n == 'r': buf.append('\r'); j += 2
            elif n == '0': buf.append('\0'); j += 2
            elif n in '"\\\'': buf.append(n); j += 2
            elif n == 'u':
                k = s.index('}', j); buf.append(chr(int(s[j+3:k], 16))); j = k + 1
            elif n == '\n':
                j += 2
                while s[j] in ' \t\n': j += 1
            else: buf.append('\\' + n); j += 2
        elif c == '"': return ''.join(buf), j + 1
        else: buf.append(c); j += 1

def split_args(s, i):
    """s[i] == '(' ; return list of (raw_text, literal_or_None) args and end index."""
    depth = 0; j = i; args = []; cur_start = i + 1; lits = {}
    while True:
        c = s[j]
        if c == '"' or (c == 'r' and s[j+1:j+2] in ('"', '#') and not s[j-1].isalnum() and s[j-1] != '_'):
            r = lex_string(s, j)
            if r:
                lits.setdefault(len(args), []).append(r[0]); j = r[1]; continue
        if c in '([{': depth += 1
        elif c in ')]}':
            depth -= 1
            if depth == 0:
                args.append((s[cur_start:j].strip(), lits.get(len(args)))); return args, j + 1
        elif c == ',' and depth == 1:
            args.append((s[cur_start:j].strip(), lits.get(len(args)))); cur_start = j + 1
        elif c == '/' and s[j+1] == '/':
            j = s.index('\n', j); continue
        j += 1

def lit(arg):
    raw, l = arg
    if l and len(l) == 1 and re.fullmatch(r'r?#*".*"#*', raw, re.S): return l[0]
    return None

EXPR_IDX = {  # helper name -> arg indexes that hold FEEL expressions (scope is arg 1)
}
for f in sorted(glob.glob(os.path.join(ROOT, 'feel-evaluator/src/tests/**/*.rs'), recursive=True)):
    s = open(f).read()
    rel = os.path.relpath(f, ROOT)
    # split per test fn
    for m in re.finditer(r"#\[test\]\s*\nfn (\w+)\(\)\s*\{", s):
        start = m.end(); body_end = s.find('\n}\n', start)
        if body_end < 0: body_end = len(s)
        body = s[start:body_end]
        scopes = {}
        for sm in re.finditer(r'let (\w+) = &?te_scope\(', body):
            a, _ = split_args(body, sm.end() - 1)
            v = lit(a[0])
            if v is not None: scopes[sm.group(1)] = v
        for cm in re.finditer(r'\b(te_\w+|textual_expression)\(', body):
            name = cm.group(1)
            if name in ('te_scope', 'te_none'): continue
            a, _ = split_args(body, cm.end() - 1)
            if len(a) < 3: continue
            sc = a[1][0].replace(' ', '')
            if sc in ('&scope!()', 'scope!()', '&FeelScope::default()'): ctx = ''
            else:
                key = sc.lstrip('&')
                if key in scopes: ctx = scopes[key]
                else: continue
            idxs = [2]
            if name in ('te_be_value', 'te_value', 'te_context'): idxs = [2, 3]
            for k in idxs:
                if k < len(a):
                    e = lit(a[k])
                    if e is not None and e.strip():
                        out.append({'src': f'{rel}::{m.group(1)}[{name}]', 'ctx': ctx, 'expr': e})
# bbt feel tests
for f in sorted(glob.glob(os.path.join(ROOT, 'bbt/tests/**/*.feel'), recursive=True)):
    ctxf = f[:-5] + '.ctx'
    ctx = open(ctxf).read() if os.path.exists(ctxf) else ''
    out.append({'src': os.path.relpath(f, ROOT), 'ctx': ctx, 'expr': open(f).read()})
seen = set(); uniq = []
for o in out:
    k = (o['ctx'], o['expr'])
    if k in seen: continue
    seen.add(k); uniq.append(o)
with open(sys.argv[2], 'w') as w:
    for o in uniq: w.write(json.dumps(o) + '\n')
print(f'harvested {len(out)} raw, {len(uniq)} unique', file=sys.stderr)
