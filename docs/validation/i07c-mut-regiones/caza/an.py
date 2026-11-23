import json,sys
n=d=0
for l in open(sys.argv[1],encoding='utf-8'):
    r=json.loads(l); n+=1
    if r["texts"]: d+=1; print("DIFF",r["name"],list(r["texts"])[:2])
print("sources",n,"with-difference",d)
