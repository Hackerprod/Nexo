import sys, json, random, hashlib
LEAVES = ['q','r','s','1','2','true','false','ar','aar','r3','[q,r]','r[0]','[q]','[r]','q[0]','[ar]','[[q]]','[s,q]']
LEAF_W = [5,5,2,2,1,2,1,3,2,1,2,2,1,1,1,2,1,1]
ASGS = ['q=[[ar]];','q=[ar];','q=[[3]];','q=[3];','q=ar;','r=[true];','r=[[ar]];','r=[ar];','r=[3];','r=ar;','s=[[ar]];','s=[true];','q=r;','r=q;','q=aar;','r=[[3]];','s=q;','s=[3];','q=[false];']
CTX = ['let a0=%s;']*3+['Print(%s);','let a0=[%s];','let a0=%s==ar;','let a0=%s==aar;','let a0:I64=%s;','let a0=[1,%s];','let a0=[%s,%s2];']
def tree(rng,d):
    if d==0: return rng.choices(LEAVES,LEAF_W)[0]
    a=rng.random()<0.5; b=rng.random()<0.5
    return ('if', tree(rng,d-1) if a else tree(rng,0), tree(rng,d-1) if b else tree(rng,0))
def depth_of(t): return 0 if isinstance(t,str) else 1+max(depth_of(t[1]),depth_of(t[2]))
def leaves_of(t): return [t] if isinstance(t,str) else leaves_of(t[1])+leaves_of(t[2])
def render(t): return t if isinstance(t,str) else '(if p then %s else %s)'%(render(t[1]),render(t[2]))
def make(rng):
    d=rng.choice([1,1,2,2,3,3,4]); t=tree(rng,d)
    if depth_of(t)<1: t=('if',t,rng.choice(LEAVES))
    lv=leaves_of(t)
    if len(lv)>8: return None
    pre='fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];'
    if 's' in ''.join(lv).replace('ar','').replace('aar','') or rng.random()<0.15: pre+='var s=[];'
    asg=rng.sample(ASGS,rng.choice([0,1,1,2,2,3]))
    if 'var s' not in pre: asg=[a for a in asg if 's' not in a.replace('ar','').replace('aar','')]
    sp=rng.randint(0,len(asg)); before,after=asg[:sp],asg[sp:]
    ctx=rng.choice(CTX); u=render(t)
    if '%s2' in ctx:
        s=ctx.replace('%s2','%s')%(u,render(tree(rng,rng.choice([0,1]))))
    else: s=ctx%u
    if 'var s' not in pre and 's' in set(leaves_of(t)): return None
    return pre+''.join(before)+s+''.join(after)+'}'
seed,n,out=int(sys.argv[1]),int(sys.argv[2]),sys.argv[3]
rng=random.Random(seed); seen=set(); progs=[]; g=0
while len(progs)<n and g<n*60:
    g+=1; s=make(rng)
    if s is None or s in seen: continue
    seen.add(s); progs.append(s)
json.dump(progs,open(out,'w',newline=''),indent=0)
print(len(progs),'seed',seed,hashlib.sha256(open(out,'rb').read()).hexdigest())
