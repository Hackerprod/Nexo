import json,os,subprocess,sys,time,hashlib
TREE='C:/nexo-work/val/i07c-verify'
POP=json.load(open('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json'))['mutations']
ITEM={m['id']:m for m in POP}
TODO=json.load(open('D:/nexo-work/mut-s3/inconclusas-pendientes.json'))
OUT='D:/nexo-work/mut-s3/verify-hang.jsonl'
env=dict(os.environ,CARGO_TARGET_DIR='C:/nexo-work/target-verify',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='1',TEMP='C:/nexo-work/tmp',TMP='C:/nexo-work/tmp')
os.makedirs('C:/nexo-work/target-verify',exist_ok=True)
def run(argv,timeout):
    t=time.monotonic()
    p=subprocess.Popen(argv,cwd=TREE,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8',errors='replace')
    try:
        out,_=p.communicate(timeout=timeout); return p.returncode,out,round(time.monotonic()-t,1),False
    except subprocess.TimeoutExpired:
        subprocess.run(['taskkill','/PID',str(p.pid),'/T','/F'],capture_output=True)
        out,_=p.communicate(); return None,out,round(time.monotonic()-t,1),True
def cargo(prof,extra):
    a=['cargo','test','--locked','--offline']
    if prof=='release': a.append('--release')
    return a+['--lib']+extra
path=TREE+'/src/check/types.rs'; orig=open(path,'rb').read(); sha0=hashlib.sha256(orig).hexdigest()
def log(row):
    open(OUT,'a',encoding='utf-8').write(json.dumps(row,ensure_ascii=False)+'\n'); print(json.dumps(row,ensure_ascii=False),flush=True)
base={}
pairs=[(i,p,w) for i,p,w in TODO if w.startswith('check::tests') or w.startswith('i07c_tests')]
for i,prof,w in pairs:
    flt='i07c_tests' if w.startswith('i07c_tests') else 'check::tests'
    if (prof,flt) not in base:
        r,o,s,to=run(cargo(prof,['--no-run']),1500)
        r2,o2,s2,to2=run(cargo(prof,[flt,'--','--quiet']),300)
        base[(prof,flt)]={'build_exit':r,'run_exit':r2,'run_seconds':s2,'timeout':to2,'tail':(o2[-300:] if r2!=0 else '')}
        log({'tipo':'base','perfil':prof,'filtro':flt,**base[(prof,flt)]})
    m=ITEM[i]; src=orig.decode('utf-8')
    assert src.count(m['literal'])==1,i
    mut=src.replace(m['literal'],m['replacement'],1).encode('utf-8')
    open(path,'wb').write(mut)
    try:
        r,o,s,to=run(cargo(prof,['--no-run']),1500)
        if to or r not in (0,):
            log({'tipo':'mutante','id':i,'perfil':prof,'filtro':flt,'resultado':'BUILD_TIMEOUT' if to else 'NO_COMPILA','build_seconds':s,'tail':o[-300:]}); continue
        r2,o2,s2,to2=run(cargo(prof,[flt,'--','--quiet']),300)
        if to2: res='NO_TERMINA_EN_300s'
        elif r2==0: res='PASA'
        else: res='FALLA'
        log({'tipo':'mutante','id':i,'perfil':prof,'filtro':flt,'resultado':res,'run_seconds':s2,'build_seconds':s,'mut_sha':hashlib.sha256(mut).hexdigest()[:16],'tail':o2[-200:] if res=='FALLA' else ''})
    finally:
        open(path,'wb').write(orig)
        assert hashlib.sha256(open(path,'rb').read()).hexdigest()==sha0
log({'tipo':'fin','sha_restaurado':sha0})
