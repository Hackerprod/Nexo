import json, random, itertools, hashlib, sys
sys.argv=['x','0','0','/dev/null']
src=open('gen2.py').read().split("seed,n,out=")[0]
exec(src)
rng=random.Random(5150177)
out=[];groups=[]
while len(groups)<260:
    t=tree(rng,rng.choice([1,2,2,3]))
    if depth_of(t)<1: continue
    lv=leaves_of(t)
    if len(lv)>6: continue
    k=rng.choice([2,3]); pool=[a for a in ASGS if 's' not in a.replace('ar','').replace('aar','')] if 's' not in set(lv) else ASGS
    asg=rng.sample(pool,k)
    u=render(t); ctx=rng.choice(['let a0=%s;','let a0=%s;','let a0=%s==ar;','let a0=[%s];','Print(%s);'])
    pre='fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];'+('var s=[];' if 's' in set(lv) or any(a.startswith('s=') or a.endswith('s;') for a in asg) else '')
    ids=[]
    for perm in itertools.permutations(asg):
        for sp in range(k+1):
            ids.append(len(out)); out.append(pre+''.join(perm[:sp])+(ctx%u)+''.join(perm[sp:])+'}')
    groups.append(ids)
json.dump({'progs':out,'groups':groups},open('perm.json','w'))
print(len(groups),len(out),hashlib.sha256(open('perm.json','rb').read()).hexdigest())
