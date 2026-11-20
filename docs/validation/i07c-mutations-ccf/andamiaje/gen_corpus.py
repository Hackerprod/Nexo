import sys,random,json,glob,os
src=open('D:/nexo-work/val-D1/h/fuzz.py',encoding='utf-8').read()
cut=src.index('progs=[prog()')
seg=src[:cut]
progs=[]
for seed in range(101,141):
    sys.argv=['x',str(seed),'900']
    ns={}
    exec(compile(seg,'fuzz_seg','exec'),ns)
    progs+= [ns['prog']() for _ in range(900)]
extra=['fn Main(){let p=true;var r=Some(1);var q=r;r=(if p then 1 else (if true then 1 else (Some(r)==q)));}']
R='D:/nexo-work/gate-d'
for pat in ('corpus/**/*.n','tests/fixtures/**/*.n'):
    for f in sorted(glob.glob(R+'/'+pat,recursive=True)): extra.append(open(f,encoding='utf-8',errors='replace').read())
rd=json.load(open(R+'/tests/fixtures/i07c_rd.json'))['rows']
for r in rd: extra.append(r['source'])
allp=list(dict.fromkeys(extra+progs))
json.dump(allp,open('D:/nexo-work/mut-s3/hunt-corpus.json','w',encoding='utf-8'))
print('corpus',len(allp),'de',len(progs)+len(extra))
