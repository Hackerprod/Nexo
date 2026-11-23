import json,itertools,hashlib
PRE='fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];'
anch=['[q,r]','q','[q]','ar','[r]','[s,q]']
bad=['ar','1','true','aar','[ar]','[true]','r3']
tail=['ar','1','r','q','s','[q]','[[q]]','r[0]','aar','[r]','[q,r]','true','q[0]','[[1]]']
asg=['','r=q;q=[[ar]];','q=[[ar]];','r=[3];','q=[3];r=[true];','r=q;','q=r;r=[[ar]];','s=q;q=[true];','q=ar;']
out=[]
for a,b,t,g in itertools.product(anch,bad,tail,asg):
    for shape in ('(if p then %s else (if p then %s else %s))','(if p then %s else (if p then %s else (if p then %s else %s)))'):
        if shape.count('%s')==4:
            u=shape%(a,b,t,'r')
        else: u=shape%(a,b,t)
        for ctx in ('let a0=%s;',):
            out.append(PRE+(ctx%u)+g+'}')
out=list(dict.fromkeys(out))
json.dump(out,open('corpus-t.json','w'))
print(len(out),hashlib.sha256(open('corpus-t.json','rb').read()).hexdigest())
