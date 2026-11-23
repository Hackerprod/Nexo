import json,subprocess,sys
B1='C:/projects/Nexo-i07c-c/.scratch/bin/'
BM='C:/projects/Nexo-i07c/.scratch/cand-bin/'
O='C:/projects/Nexo-i07c/scripts/i07c_s5c_oraculo.py'
BIN=[('main',BM+'base-release.exe'),('v6',BM+'cand-release.exe'),('lit',B1+'lit-release.exe'),('b1',B1+'b1-release.exe')]
def run(exe,src,verb='check'):
    open('m/t.n','w',newline='').write(src)
    p=subprocess.run([exe,verb,'--format','json','--profile','f0-0002-b-v2','m/t.n'],capture_output=True,text=True)
    try: j=json.loads(p.stdout)
    except Exception: return ['ERR '+p.stdout[:80]+p.stderr[:80]]
    r=[]
    for d in j['diagnostics']:
        dd=d.get('data') or {}
        r.append((d['code'].replace('NX-',''),d['primary_span']['start'],d['primary_span']['end'],dd.get('expected_type'),dd.get('found_type')))
    return sorted(r,key=lambda x:(x[1],x[2],x[0]))
def orc(src):
    o=json.loads(subprocess.run([sys.executable,'-B',O,'--source',src],capture_output=True,text=True,cwd='C:/projects/Nexo-i07c').stdout)
    return sorted([(d['code'].replace('NX-',''),d['primary'][0],d['primary'][1],d['expected_type'] or None,d['found_type'] or None) for d in o['diagnostics']],key=lambda x:(x[1],x[2],x[0])),o.get('unmodeled')
def show(name,src,oracle=True):
    print('##',name,src[src.index('let a'):] if 'let a' in src else src)
    if oracle:
        r,u=orc(src); print('  oraculo',r,'unmodeled' if u else '')
    for n,e in BIN:
        a=run(e,src); print('  %-5s'%n,a)
PF="fn Main(){let p=true;let ar=[1];let aar=[[1]];"
if __name__=='__main__':
    D='C:/projects/Nexo-i07c/docs/validation/i07c-s5a/d-repros/'
    show('R4',open(D+'R4.n').read())
    show('P15',PF+"var r=[];let a=(if p then ar else (if p then aar else r));}")
    show('P16',PF+"var r=[];let a=(if p then ar else (if p then aar else r));r=[true];}")
