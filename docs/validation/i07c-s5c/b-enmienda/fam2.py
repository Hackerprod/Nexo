import re,sys
src=open('fam.py').read()
src=src.replace("""ASG=['','q=[[ar]];','r=[true];','q=[[3]];r=[[ar]];']""","""ASG=['','q=[[ar]];','r=[true];','q=[[3]];r=[[ar]];']
PRE=['','q=[[ar]];','r=[ar];q=[[ar]];']""")
# nueva generacion
i=src.index("progs=[]"); j=src.index("progs=list(dict.fromkeys(progs))")
gen='''progs=[]
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
'''
src=src[:i]+gen+src[j:]
open('fam2_run.py','w').write(src)
