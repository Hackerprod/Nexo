import sys, json, subprocess, os, importlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import orc_P, orc_N, orc_N2
EXE=r'C:\projects\Nexo-i07c\.scratch\cand-bin\%s-release.exe'
W=os.path.join(os.environ.get('TEMP','.'),'vbd3'); os.makedirs(W,exist_ok=True)
def sig_bin(j):
    out=[]
    for d in j['diagnostics']:
        data=d.get('data') or {}
        out.append((d['code'].replace('NX-',''),d['primary_span']['start'],d['primary_span']['end'],data.get('expected_type'),data.get('found_type')))
    return sorted(out,key=str)
def run(exe,src,sub='check'):
    f=os.path.join(W,'p.n'); open(f,'w',newline='').write(src)
    p=subprocess.run([exe,sub,'--format','json','--profile','f0-0002-b-v2',f],capture_output=True,text=True,timeout=60)
    try: return sig_bin(json.loads(p.stdout))
    except Exception: return 'ERR'+p.stdout[:100]
def orc(m,src):
    try:
        r=m.derive(src)
        if r.get('unmodeled') or r.get('error'): return 'unmod'
        return sorted([(d['code'].replace('NX-',''),d['primary'][0],d['primary'][1],d.get('expected_type'),d.get('found_type')) for d in r['diagnostics']],key=str)
    except Exception as e: return 'EXC'+str(e)[:50]
if __name__=='__main__':
    for src in json.load(open(sys.argv[1])):
        print(src[src.index('aar=[[1]];')+10:] if 'aar=[[1]];' in src else src)
        print('  main',run(EXE%'base',src)); print('  P   ',orc(orc_P,src)); print('  N   ',orc(orc_N,src))
