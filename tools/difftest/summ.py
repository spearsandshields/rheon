import sys, collections
seen=collections.Counter()
for line in open(sys.argv[1]):
    f=line.rstrip('\n').split('\t')
    op=f[0]
    if sys.argv[2:] and op not in sys.argv[2].split(','): continue
    a,b,k,idg,fdg=f[2],f[3],f[4],f[7],f[8]
    if op.startswith('chain'):
        I=idg.split(' -> '); F=fdg.split(' -> ')
        j=next(j for j in range(max(len(I),len(F))) if (I[j:j+1] or [''])[0]!=(F[j:j+1] or [''])[0])
        key=(op,a,b)
        print(f"{op} start={a} k={k} step{j}: prev={I[j-1] if j else ''} intel={I[j] if j<len(I) else '-'} fast={F[j] if j<len(F) else '-'} (chain len I={len(I)} F={len(F)})")
    else:
        key=(op,a,b,k)
        if seen[key]: continue
        seen[key]+=1
        print(f"{op}({a}, {b}, k={k}) intel={idg} fast={fdg}")
