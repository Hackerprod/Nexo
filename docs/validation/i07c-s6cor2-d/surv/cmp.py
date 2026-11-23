import subprocess,json,sys
D='C:/nexo-work/val/d-s6cor2/'
CAND='C:/nexo-work/cache/033567b4f8eff7edfbf4d273db3fe81ce35ae343/585154a1ef365515/debug.exe'
def run(exe,verb,f,prof='f0-0003-b-v1'):
    d=subprocess.run([exe,verb,f,'--profile',prof,'--format','json'],capture_output=True,timeout=300)
    j=json.loads(d.stdout)
    return (d.returncode,j['outcome']['kind'],[(x['code'],x['subcause'],(x['primary_span'] or {}).get('start'),(x['primary_span'] or {}).get('end')) for x in j['diagnostics']])
jobs=[('M00038','check','p38.n'),('M00040','check','p40.n'),('M00047','check','p48.n'),('M00048','check','p48.n'),('M00051','check','p51.n'),
 ('M00057','check','p57b.n'),('M00057','check','p57.n'),('M00061','check','p61.n'),('M00006','run','p6.n'),('M00029','run','p29.n')]
for m,verb,f in jobs:
    a=run(CAND,verb,f); b=run(D+'mutbin_%s.exe'%m,verb,f)
    print(m,f,verb,'DIFF' if a!=b else 'same'); 
    if a!=b: print('   cand',a);print('   mut ',b)
