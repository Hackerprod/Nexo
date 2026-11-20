import json,os,subprocess,time,hashlib
TREE='C:/nexo-work/val/i07c-verify'
POP={m['id']:m for m in json.load(open('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json'))['mutations']}
IDS=['M00121','M00676','M00732']
OUT='D:/nexo-work/mut-s3/verify-lib.jsonl'
env=dict(os.environ,CARGO_TARGET_DIR='C:/nexo-work/target-verify',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='1',TEMP='C:/nexo-work/tmp',TMP='C:/nexo-work/tmp')
def run(argv,timeout):
    t=time.monotonic()
    p=subprocess.Popen(argv,cwd=TREE,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8',errors='replace')
    try:
        out,_=p.communicate(timeout=timeout); return p.returncode,out,round(time.monotonic()-t,1),False
    except subprocess.TimeoutExpired:
        subprocess.run(['taskkill','/PID',str(p.pid),'/T','/F'],capture_output=True)
        out,_=p.communicate(); return None,out,round(time.monotonic()-t,1),True
def log(r):
    open(OUT,'a',encoding='utf-8').write(json.dumps(r,ensure_ascii=False)+'\n'); print(json.dumps(r,ensure_ascii=False),flush=True)
LIB=['cargo','test','--locked','--offline','--lib']
r,o,s,to=run(LIB+['--no-run'],1500)
r2,o2,s2,to2=run(LIB+['--','--quiet'],900)
log({'tipo':'base','perfil':'debug','build_exit':r,'run_exit':r2,'run_seconds':s2,'timeout':to2,'tail':o2[-160:] if r2!=0 else ''})
for i in IDS:
    m=POP[i]; path=f"{TREE}/{m['file']}"; orig=open(path,'rb').read(); sha0=hashlib.sha256(orig).hexdigest()
    src=orig.decode('utf-8'); assert src.count(m['literal'])==1,i
    open(path,'wb').write(src.replace(m['literal'],m['replacement'],1).encode('utf-8'))
    try:
        r,o,s,to=run(LIB+['--no-run'],1500)
        if to or r!=0: log({'tipo':'mutante','id':i,'resultado':'BUILD_TIMEOUT' if to else 'NO_COMPILA','tail':o[-200:]}); continue
        r2,o2,s2,to2=run(LIB+['--','--quiet'],900)
        res='NO_TERMINA_EN_900s' if to2 else ('PASA' if r2==0 else 'FALLA')
        fails=[l for l in o2.splitlines() if l.startswith('---- ')][:3]
        log({'tipo':'mutante','id':i,'archivo':m['file'],'linea':m['line'],'resultado':res,'run_seconds':s2,'fallos':fails})
    finally:
        open(path,'wb').write(orig); assert hashlib.sha256(open(path,'rb').read()).hexdigest()==sha0
log({'tipo':'fin'})
