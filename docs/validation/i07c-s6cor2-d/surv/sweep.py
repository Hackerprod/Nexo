import subprocess,json
D='C:/nexo-work/val/d-s6cor2/'
CAND='C:/nexo-work/cache/033567b4f8eff7edfbf4d273db3fe81ce35ae343/585154a1ef365515/release.exe'
def run(exe,src):
    open('sw.n','w').write(src)
    d=subprocess.run([exe,'run','sw.n','--profile','f0-0003-b-v1','--format','json'],capture_output=True,timeout=300)
    j=json.loads(d.stdout)
    return (d.returncode,j['outcome']['kind'],[(x['code'],(x['primary_span'] or {}).get('start'),(x['primary_span'] or {}).get('end'),json.dumps(x['data'])) for x in j['diagnostics']],j['observation_bytes'])
progs=[]
for pad in range(0,8):
    progs.append(('M00029','fn Inc(x:edit I64){x=x+1;} fn Main(){'+'{}'*pad+'var n=0;while true{Inc(edit n);}}'))
    progs.append(('M00029','fn Id(x:move I64)->I64{return x;} fn Main(){'+'{}'*pad+'var n=0;while true{let y=Id(move n);}}'))
    progs.append(('M00006','fn Main(){'+'{}'*pad+'var a=[1,2,3];while true{a;}}'))
    progs.append(('M00006','fn Main(){'+'{}'*pad+'var a=[1,2,3];while true{a[0];}}'))
    progs.append(('M00006','fn Main(){'+'{}'*pad+'var a=[1,2,3];let b=[a,a];while true{b;}}'))
for m,s in progs:
    a=run(CAND,s); b=run(D+'mutbin_%s.exe'%m,s)
    if a!=b: print('DIFF',m,s[:120]);print('  ',a);print('  ',b)
print('done')
