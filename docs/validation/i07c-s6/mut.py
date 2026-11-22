import sys, json, hashlib, subprocess, os, re, time
TREE='C:/nexo-work/val/s6mut/tree'
OUT='D:/nexo-work/mut-s3/s6mut'
def sha(b): return hashlib.sha256(b).hexdigest()
def sh(cmd, timeout=1800):
    env=dict(os.environ)
    pre='. /c/nexo-work/val/s6mut/env.sh && cd /c/nexo-work/val/s6mut/tree && '
    p=subprocess.run(['sh','-c',pre+cmd],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=timeout)
    return p.returncode,p.stdout+p.stderr
muts=json.load(open('C:/nexo-work/val/s6mut/mutants.json',encoding='utf-8'))
ids=sys.argv[1:]
for mid in ids:
    m=muts[mid]
    path=f"{TREE}/{m['file']}"
    orig=open(path,'rb').read(); s0=sha(orig)
    text=orig.decode('utf-8')
    cnt=text.count(m['before'])
    res={'id':mid,'desc':m.get('desc'),'file':m['file'],'sha_initial':s0,'occurrences':cnt}
    if cnt<1 or (cnt>1 and 'occurrence' not in m):
        res['result']='ANCLA NO UNICA' ; print(json.dumps(res)); continue
    occ=m.get('occurrence',0)
    pos=[x.start() for x in re.finditer(re.escape(m['before']),text)][occ]
    line=text.count('\n',0,pos)+1
    res['line']=line
    new=text[:pos]+m['after']+text[pos+len(m['before']):]
    for b2,a2 in m.get('extra',[]):
        assert new.count(b2)==1, b2
        new=new.replace(b2,a2)
    nb=new.encode('utf-8'); res['sha_mutated']=sha(nb)
    json.dump({'file':m['file'],'before':m['before'],'after':m['after'],'occurrence':occ,'sha_initial':s0,'sha_mutated':res['sha_mutated']},open(f"{OUT}/{mid}.patch.json",'w'))
    try:
        open(path,'wb').write(nb)
        t=time.time()
        rc,out=sh(f"cargo nextest run --profile default --offline --retries 0 -E '{m['filter']}' 2>&1 | tail -60")
        res['seconds']=round(time.time()-t)
        failed=re.findall(r'(?:FAIL|TIMEOUT)\s+\[[^\]]*\]\s+\(\s*\d+/\d+\)\s+(\S+\s+\S+)',out)
        compile_err='error: could not compile' in out or 'error[E' in out
        res['failed']=failed[:5]
        res['tail']=out[-1500:]
        res['result']='NO COMPILA' if compile_err else ('MUERTO' if failed else ('SUPERVIVIENTE-DIRIGIDO' if 'passed' in out else 'INCONCLUSO'))
    finally:
        open(path,'wb').write(orig)
        res['sha_restored']=sha(open(path,'rb').read()); res['restored_ok']=res['sha_restored']==s0
    open(f"{OUT}/{mid}.result.json",'w',encoding='utf-8').write(json.dumps(res,indent=1,ensure_ascii=False))
    print(mid,res['result'],res['failed'],'line',line,'restored',res['restored_ok'],'occ',cnt,flush=True)
