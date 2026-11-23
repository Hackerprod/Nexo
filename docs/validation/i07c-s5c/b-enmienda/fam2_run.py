import sys,json,subprocess,itertools,random
sys.path.insert(0,'C:/projects/Nexo-i07c/scripts')
import i07c_s5c_oraculo as O
from concurrent.futures import ThreadPoolExecutor
B1='C:/projects/Nexo-i07c-c/.scratch/bin/'
PF="fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];"
LEAF=['q','r','1','ar','aar','r3','r[0]','[q,r]']
def prog(leaves,shape,asg):
    a,b,c=leaves
    if shape=='flat2': u="(if p then %s else (if p then %s else %s))"%(a,b,c)
    elif shape=='nest1': u="(if p then (if p then %s else %s) else %s)"%(a,b,c)
    else: u="(if p then %s else %s)"%(a,b)
    return PF+"let a0=%s;%s}"%(u,asg)
ASG=['','q=[[ar]];','r=[true];','q=[[3]];r=[[ar]];']
PRE=['','q=[[ar]];','r=[ar];q=[[ar]];']
progs=[]
def prog2(L,shape,pre,asg):
    if shape=='flat3':
        a,b,c,d=L; u="(if p then %s else (if p then %s else (if p then %s else %s)))"%(a,b,c,d)
    elif shape=='nest1':
        a,b,c=L; u="(if p then (if p then %s else %s) else %s)"%(a,b,c)
    elif shape=='nest2':
        a,b,c,d=L; u="(if p then %s else (if p then (if p then %s else %s) else %s))"%(a,b,c,d)
    return PF+pre+"let a0=%s;%s}"%(u,asg)
import random
random.seed(11)
for shape,n in (('nest1',3),('flat3',4),('nest2',4)):
    for L in itertools.product(LEAF,repeat=n):
        if L[-1] in ('q','r'):
            if shape!='nest1' and random.random()>0.12: continue
            for pre in PRE:
                for asg in ASG: progs.append(prog2(L,shape,pre,asg))
progs=list(dict.fromkeys(progs))
print(len(progs),'programas',flush=True)
def run(exe,src,i):
    f='m/f%d.n'%i
    open(f,'w',newline='').write(src)
    p=subprocess.run([exe,'check','--format','json','--profile','f0-0002-b-v2',f],capture_output=True,text=True)
    try: j=json.loads(p.stdout)
    except Exception: return 'ERR'
    return tuple(sorted((d['code'],d['primary_span']['start'],d['primary_span']['end']) for d in j['diagnostics']))
def oracle(src):
    try: r=O.derive(src)
    except Exception as e: return None
    if r.get('unmodeled') or r.get('error'): return None
    return tuple(sorted((d['code'],d['primary'][0],d['primary'][1]) for d in r['diagnostics']))
def work(a):
    i,s=a
    return s,run(B1+'lit-release.exe',s,i),run(B1+'b1-release.exe',s,i),run('C:/projects/Nexo-i07c/.scratch/cand-bin/base-release.exe',s,i),oracle(s)
with ThreadPoolExecutor(6) as ex: res=list(ex.map(work,enumerate(progs)))
json.dump(res,open('fam_res.json','w'))
n=len(res); mod=[r for r in res if r[4] is not None]
bo=[r for r in mod if r[2]==r[4]]; lo=[r for r in mod if r[1]==r[4]]; mo=[r for r in mod if r[3]==r[4]]
print('modelados',len(mod),'b1=oraculo',len(bo),'lit=oraculo',len(lo),'main=oraculo',len(mo))
reg=[r for r in mod if r[1]==r[4] and r[2]!=r[4]]
print('REGRESIONES b1 (lit acierta, b1 no):',len(reg))
for r in reg[:15]: print(' ',r[0][len(PF):],'\n   orc',r[4],'\n   lit',r[1],'\n   b1 ',r[2])
diff=[r for r in res if r[1]!=r[2]]
print('lit!=b1:',len(diff),' de ellas b1=oraculo:',sum(1 for r in diff if r[4] is not None and r[2]==r[4]))
bad=[r for r in mod if r[2]!=r[4]]
print('b1 distinto del oraculo (modelados):',len(bad))
for r in bad[:12]: print(' ',r[0][len(PF):],'\n   orc',r[4],'\n   lit',r[1],'\n   b1 ',r[2],'\n   main',r[3])
