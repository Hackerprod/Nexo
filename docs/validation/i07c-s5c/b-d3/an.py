import json,sys,collections
R=json.load(open(sys.argv[1]))
def norm(x): return x if isinstance(x,str) else json.dumps(x)
C=collections.Counter(); ex=collections.defaultdict(list)
unm=0
for p,r in R:
    if r['main'] in ('ERR',) or isinstance(r['main'],str): C['mainERR']+=1; continue
    for k in ('P','N','N2'):
        pass
    if isinstance(r['P'],str) or isinstance(r['N'],str):
        unm+=1; continue
    P=r['P'];N=r['N2'];M=r['main'];Ca=r['cand']
    nz=lambda L:[tuple(None if v=='' else v for v in x) for x in L]; P=nz(P);N=nz(N);M=nz(M);Ca=nz(Ca)
    if P!=N:
        C['P!=N2']+=1
        k=('main=N2' if M==N else '')+('main=P' if M==P else '')
        C['  '+(k or 'neither')]+=1
        C['  cand: '+('=N2' if Ca==N else '')+('=P' if Ca==P else '')]+=1
        if not k: ex['neither'].append((p,M,P,N))
        if M==P: ex['mainP'].append((p,M,P,N))
    else:
        C['P==N2']+=1
        C['  main=P==N2' if M==P else '  main differs']+=1
        if M!=P: ex['common'].append((p,M,P,N))
C['unmodeled']=unm
for k,v in sorted(C.items()): print(k,v)
json.dump({k:v[:200] for k,v in ex.items()},open(sys.argv[1]+'.ex','w'),default=str)
