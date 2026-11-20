import random,subprocess,os,sys,json,hashlib
from concurrent.futures import ThreadPoolExecutor
DV='(if true then {return;} else {return;})'
BR='D:/nexo-work/val-D1/target/release/nexo.exe'
BD='D:/nexo-work/val-D1/target/debug/nexo.exe'
seed=int(sys.argv[1]);N=int(sys.argv[2])
rnd=random.Random(seed)
VARS=['r','q','w','a','b']
def atom(vs):
    c=rnd.random()
    if c<.25 and vs: return rnd.choice(vs)
    return rnd.choice(['1','true','[]','[1]','[true]','[[1]]','[[]]',DV,'Some([])','Some([1])','None','[DV]'.replace('DV',DV)])
def expr(vs,d=0):
    if d>2: return atom(vs)
    c=rnd.random()
    if c<.25: return atom(vs)
    if c<.35: return 'Len(%s)'%expr(vs,d+1)
    if c<.5: return '(%s%s%s)'%(expr(vs,d+1),rnd.choice(['+','-','<','==','<=']),expr(vs,d+1))
    if c<.6: return '[%s]'%','.join(expr(vs,d+1) for _ in range(rnd.randint(1,2)))
    if c<.7: return '(if %s then %s else %s)'%(expr(vs,d+1),expr(vs,d+1),expr(vs,d+1))
    if c<.78: return 'Some(%s)'%expr(vs,d+1)
    if c<.86 and vs: return '%s[%s]'%(rnd.choice(vs),expr(vs,d+1))
    if c<.92: return '(if true then {return;} else {%s})'%('return;')
    return atom(vs)
def stmt(vs,i):
    c=rnd.random()
    if c<.3:
        n=rnd.choice(VARS)+str(i)
        vs.append(n); return 'var %s=%s;'%(n,expr(vs[:-1]))
    if c<.45:
        n='l'+str(i); ann=rnd.choice(['','','',':Bool',':I64',':Array<I64>',':Array<Bool>',':Option<Array<I64>>'])
        r='let %s%s=%s;'%(n,ann,expr(vs)); vs.append(n); return r
    if c<.65 and vs: return '%s=%s;'%(rnd.choice(vs),expr(vs))
    if c<.75 and vs: return '%s[%s]=%s;'%(rnd.choice(vs),expr(vs),expr(vs))
    if c<.88:
        v=rnd.choice(['v','x'])
        return 'match %s{Some(%s)=>{%s},_=>{}}'%(expr(vs),v,rnd.choice(['','let k:Array<I64>=%s;'%v,'let k:Bool=%s;'%v,'let k=%s;'%v]))
    if c<.94: return 'let u%d=Len(%s);'%(i,expr(vs))
    return 'if %s then {%s} else {}'%(expr(vs),'')
def prog():
    vs=[];s=''.join(stmt(vs,i) for i in range(rnd.randint(1,5)))
    return 'fn Main(){'+s+'}'
progs=[prog() for _ in range(N)]
def go(i):
    src=progs[i];p='D:/nexo-work/val-D1/h/f_%d_%d.n'%(seed,i)
    open(p,'w').write(src)
    res={}
    try:
        for tag,b in(('r',BR),('d',BD)):
            for m in('check','run'):
                pr=subprocess.run([b,m,'--profile','f0-0003-b-v1','--format','json',p],capture_output=True,text=True,timeout=60)
                res[(tag,m)]=(pr.returncode,pr.stdout.replace(p,'S'),pr.stderr[:100])
    except subprocess.TimeoutExpired: return (src,'TIMEOUT')
    finally: os.remove(p)
    bad=[]
    for k,v in res.items():
        if v[0] not in(0,1) or any(x in v[1] for x in('internal_error','InvalidIr','driver_error','panick')) or v[2]: bad.append((k,v[0],v[1][:200],v[2]))
    if res[('r','check')]!=res[('d','check')] or res[('r','run')]!=res[('d','run')]: bad.append('rel!=dbg')
    def diag(k):
        try: return json.loads(res[k][1])['diagnostics']
        except: return 'RAW'
    if diag(('r','check'))!=diag(('r','run')): bad.append('check!=run diags')
    return (src,bad) if bad else None
with ThreadPoolExecutor(3) as ex: out=[x for x in ex.map(go,range(N)) if x]
stats={}
print('seed',seed,'N',N,'bad',len(out))
for s,b in out[:8]: print(s,'\n  ',b)
