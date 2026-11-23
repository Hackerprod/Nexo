import sys,subprocess,re
sys.path.insert(0,'.')
from progs import P
def run(exe,name,prof):
    r=subprocess.run([exe,'check','--profile',prof,f'src/{name}.n'],capture_output=True,text=True)
    res=[];cur=None
    for line in (r.stdout+r.stderr).splitlines():
        m=re.match(r'\s*(NX-[A-Z]\d+)',line)
        if m: cur=[m.group(1),None,None]; res.append(cur); continue
        m=re.search(r'at: \{"end":(\d+),"file":"[^"]*","start":(\d+)\}',line)
        if m and cur and cur[1] is None: cur[1]=f"[{m.group(2)},{m.group(1)})"
    return [(a,b) for a,b,_ in res],r.returncode,r.stdout
B='C:/nexo-work/target-b-tests/base/debug/nexo.exe';C='C:/nexo-work/target-b-tests/cand/debug/nexo.exe'
if __name__=="__main__":
    names=sys.argv[1:] or list(P)
    for n in names:
        prof='f0-0003-b-v1' if n in('bN','bP','dD') else 'f0-0002-b-v2'
        for lab,e in(('main',B),('cand',C)):
            res,rc,_=run(e,n,prof)
            print(n,lab,rc,res)
