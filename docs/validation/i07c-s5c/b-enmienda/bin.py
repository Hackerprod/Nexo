import json,subprocess,re
exec(open('mine2.py').read().split("ok=0")[0])
B='C:/projects/Nexo-i07c/.scratch/cand-bin/'
for n,suf,exp in P:
    s=PF+suf; open('m/t.n','w',newline='').write(s)
    E=[]
    for code,snip,off,ln,e,f in exp:
        i=s.index(snip)+off; E.append((code,i,i+ln))
    r=[]
    for b in ('base-release.exe','cand-release.exe'):
        j=json.loads(subprocess.run([B+b,'check','--format','json','--profile','f0-0002-b-v2','m/t.n'],capture_output=True,text=True).stdout)
        G=[(d['code'],d['primary_span']['start'],d['primary_span']['end']) for d in j['diagnostics']]
        r.append('=' if sorted(G)==sorted(E) else G)
    print(n,'main',r[0],'v6',r[1])
