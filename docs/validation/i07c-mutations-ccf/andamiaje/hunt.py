import json,os,subprocess,sys,time,hashlib,shutil
from concurrent.futures import ThreadPoolExecutor
W=int(sys.argv[1]); NW=int(sys.argv[2])          # indice de trabajador y total
ROOT='C:/nexo-work/hunt'; TREE=f'C:/nexo-work/val/hunt-w{W}'; TARGET=f'C:/nexo-work/target-hunt{W}'
POP={m['id']:m for m in json.load(open('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json'))['mutations']}
IDS=json.load(open('D:/nexo-work/mut-s3/sin-programa-ids.json'))
mine=[i for k,i in enumerate(IDS) if k%NW==W]
OUT=f'D:/nexo-work/mut-s3/hunt-{W}.jsonl'
done=set()
if os.path.exists(OUT): done={json.loads(l)['id'] for l in open(OUT,encoding='utf-8') if l.strip()}
env=dict(os.environ,CARGO_TARGET_DIR=TARGET,CARGO_PROFILE_RELEASE_DEBUG='0',CARGO_INCREMENTAL='1',TEMP='C:/nexo-work/tmp',TMP='C:/nexo-work/tmp')
def run(argv,timeout,cwd=None):
    p=subprocess.Popen(argv,cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',errors='replace')
    try:
        o,e=p.communicate(timeout=timeout); return p.returncode,o,e,False
    except subprocess.TimeoutExpired:
        subprocess.run(['taskkill','/PID',str(p.pid),'/T','/F'],capture_output=True)
        try: o,e=p.communicate(timeout=10)
        except Exception: o,e='',''
        return None,o,e,True
N=len(json.load(open('D:/nexo-work/mut-s3/hunt-corpus.json',encoding='utf-8')))
def one(args):
    exe,mode,i=args
    p=f'{ROOT}/corpus/p{i:05d}.n'
    r,o,e,to=run([exe,mode,'--profile','f0-0003-b-v1','--format','json',p],15)
    return (i,'TIMEOUT' if to else (r,o.replace(p,'S')))
def corpus_run(exe,modes,stop_after=None,base=None):
    res={};diffs=[]
    with ThreadPoolExecutor(8) as ex:
        for mode in modes:
            idx=range(N)
            for chunk in range(0,N,2000):
                batch=[(exe,mode,i) for i in range(chunk,min(N,chunk+2000))]
                for i,v in ex.map(one,batch):
                    res[(mode,i)]=v
                    if base is not None and base[(mode,i)]!=v: diffs.append((mode,i,'TIMEOUT' if v=='TIMEOUT' else 'DIF'))
                if stop_after and len(diffs)>=stop_after: return res,diffs
    return res,diffs
def build():
    t=time.monotonic(); r,o,e,to=run(['cargo','build','--release','--locked','--offline','--bin','nexo'],1800,TREE)
    return r,round(time.monotonic()-t,1),(e or '')[-200:]
if W==0 and not os.path.exists(f'{ROOT}/base.json'):
    pass
# base compartida: la crea el lanzador antes (base.json con salidas en orden)
base_raw=json.load(open(f'{ROOT}/base.json',encoding='utf-8'))
base={}
for k,v in base_raw.items():
    mode,i=k.split(':'); base[(mode,int(i))]=v if v=='TIMEOUT' else (v[0],v[1])
def log(r): open(OUT,'a',encoding='utf-8').write(json.dumps(r,ensure_ascii=False)+'\n'); print(json.dumps(r,ensure_ascii=False),flush=True)
for i in mine:
    if i in done: continue
    m=POP[i]; path=f"{TREE}/{m['file']}"; orig=open(path,'rb').read(); sha0=hashlib.sha256(orig).hexdigest()
    src=orig.decode('utf-8'); lit,rep=m['literal'],m['replacement']
    if src.count(lit)!=1:
        try: lit2,rep2=lit.encode('cp1252').decode('utf-8'),rep.encode('cp1252').decode('utf-8')
        except Exception: lit2,rep2=None,None
        if lit2 is not None and src.count(lit2)==1: lit,rep=lit2,rep2
        else:
            log({'id':i,'resultado':'LITERAL_NO_ENCONTRADO','apariciones':src.count(lit)}); continue
    open(path,'wb').write(src.replace(lit,rep,1).encode('utf-8'))
    try:
        r,bs,tail=build()
        if r!=0: log({'id':i,'resultado':'NO_COMPILA','build_s':bs,'tail':tail}); continue
        exe=f'{TARGET}/release/nexo.exe'
        modes=['check','run'] if m['file'].endswith('lower_match.rs') else ['check']
        t=time.monotonic(); res,diffs=corpus_run(exe,modes,stop_after=5,base=base)
        log({'id':i,'archivo':m['file'].split('/')[-1],'linea':m['line'],'resultado':'DISCRIMINADO' if diffs else 'SIN_DIFERENCIA','n_diferencias':len(diffs),'primeras':[(a,b,c) for a,b,c in diffs[:5]],'build_s':bs,'corpus_s':round(time.monotonic()-t),'programas':N})
    finally:
        open(path,'wb').write(orig); assert hashlib.sha256(open(path,'rb').read()).hexdigest()==sha0
log({'fin':True,'trabajador':W})
