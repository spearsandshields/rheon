import json, sys
a=[json.loads(l) for l in open(sys.argv[1])]; b=[json.loads(l) for l in open(sys.argv[2])]
assert len(a)==len(b)
dd=dbg=0; rows=[]
for x,y in zip(a,b):
    assert x['expr']==y['expr']
    if x['display']!=y['display']:
        dd+=1; rows.append(('DISPLAY',x,y))
    elif x['debug']!=y['debug']:
        dbg+=1; rows.append(('DEBUG-ONLY',x,y))
print(f"cases={len(a)} display_mismatch={dd} debug_only_mismatch={dbg}")
for k,x,y in rows:
    print(f"--- {k} {x['src']}\n  ctx : {x['ctx'][:200]!r}\n  expr: {x['expr'][:300]!r}\n  intel  : {x['display'][:400]}\n  dashu: {y['display'][:400]}")
    if k=='DEBUG-ONLY': print(f"  intel dbg  : {x['debug'][:400]}\n  dashu dbg: {y['debug'][:400]}")
pi=sum(1 for x in a if 'PANIC' in x['display']); pf=sum(1 for x in b if 'PANIC' in x['display'])
print(f"panics intel={pi} dashu={pf}")
print(f"total eval ns intel={sum(x['ns'] for x in a)} dashu={sum(x['ns'] for x in b)}")
