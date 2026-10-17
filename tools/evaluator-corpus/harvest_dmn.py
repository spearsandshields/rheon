#!/usr/bin/env python3
"""Harvest FEEL texts (<text> of literalExpression / entries) from all .dmn files under a dsntk checkout.
Free variables evaluate to null on both backends; kept anyway as parse/eval coverage."""
import sys, os, glob, json, re, html
ROOT=sys.argv[1]; out=[]; seen=set()
for f in sorted(glob.glob(os.path.join(ROOT,'**/*.dmn'),recursive=True)):
    if '/target/' in f: continue
    s=open(f,encoding='utf-8',errors='replace').read()
    for m in re.finditer(r'<(?:\w+:)?text>(.*?)</(?:\w+:)?text>', s, re.S):
        e=html.unescape(m.group(1)).strip()
        if not e or e=='-' or e in seen: continue
        seen.add(e); out.append({'src':os.path.relpath(f,ROOT)+'#text','ctx':'','expr':e})
with open(sys.argv[2],'w') as w:
    for o in out: w.write(json.dumps(o)+'\n')
print(f'dmn texts unique {len(out)}',file=sys.stderr)
