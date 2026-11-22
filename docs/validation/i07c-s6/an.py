import json,sys,re
d=json.load(open(sys.argv[1]))
def sig(out):
    try: j=json.loads(out)
    except Exception: return out[:80]
    r=[]
    for x in j.get('diagnostics',[]):
        p=x.get('primary_span') or {}
        r.append((x['code'],x.get('subcause'),p.get('start'),p.get('end'),[(q['role'],q['span']['start'],q['span']['end']) for q in x.get('related_spans',[])],x.get('data')))
    return (j.get('outcome',{}).get('kind'),r)
seen=set()
n=0
for name,src,pr,verb,b,m in d:
    if pr!='f0-0002-b-v2' or verb!='check': continue
    n+=1
    if n>int(sys.argv[2]) : break
    sb,sm=sig(b[1]),sig(m[1])
    print('NAME',name); print(' SRC',src[src.index('let w=true;'):] if 'let w=true' in src else src)
    print(' BASE',sb); print(' MUT ',sm)
