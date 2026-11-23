import random, json, hashlib, sys
rng=random.Random(0xB7C3E1)
PRE="fn Main(){let p=true;let ar=[1];let aar=[[1]];let br=[true];var q=[];var w=[];"
GOOD=["1","2","true","p","ar","aar","br","[ar,ar]","[1,2]","[true]","[[1]]","[br,br]"]
BAD=["[ar,aar]","[1,true]","[true,1]","[ar,1]","[aar,ar]","[q,1]","[1,q]","[true+1,true]","[ar,[1,true]]","[[ar,aar]]","[w,aar]","[br,ar]"]
def union(d,needbad):
    # returns source of if-union with depth d; needbad forces a bad literal somewhere
    def leaf(bad):
        return rng.choice(BAD) if bad else rng.choice(GOOD)
    def build(d,bad):
        if d==0 or rng.random()<0.35:
            return leaf(bad)
        # choose side holding the bad leaf
        side=rng.choice([0,1])
        a=build(d-1, bad and side==0) if rng.random()<0.5 else leaf(bad and side==0)
        b=build(d-1, bad and side==1) if rng.random()<0.5 else leaf(bad and side==1)
        return "(if p then %s else %s)"%(a,b)
    u=build(d,needbad)
    if not u.startswith("(if"):
        u="(if p then %s else %s)"%((u,leaf(False)) if rng.random()<0.5 else (leaf(False),u))
    return u
WR=["let a0=%s;","let a0=%s==aar;","Print(%s);","let a0=[%s,ar];","let a0=[%s];","let a0:I64=%s;","let a0=%s+1;","let a0=Ok(%s);"]
progs=[]
while len(progs)<2100:
    d=rng.choice([1,1,2,2,3])
    u=union(d,True)
    w=rng.choice(WR)
    tail=rng.choice(["","q=[2];","q=[ar];w=[1];"])
    progs.append(PRE+(w%u)+tail+"}")
progs=list(dict.fromkeys(progs))
json.dump(progs,open('fam.json','w'))
h=hashlib.sha256(open('fam.json','rb').read()).hexdigest()
open('fam.sha256','w').write(h)
print(len(progs),h)
