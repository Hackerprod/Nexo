import json,os,subprocess,sys,time,hashlib,re
ID=sys.argv[1]
TREE='C:/nexo-work/val/i07c-verify'
POP={m['id']:m for m in json.load(open('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json'))['mutations']}
m=POP[ID]; path=f"{TREE}/{m['file']}"; orig=open(path,'rb').read(); sha0=hashlib.sha256(orig).hexdigest()
env=dict(os.environ,CARGO_TARGET_DIR='C:/nexo-work/target-verify',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='1',TEMP='C:/nexo-work/tmp',TMP='C:/nexo-work/tmp')
src=orig.decode('utf-8'); assert src.count(m['literal'])==1
open(path,'wb').write(src.replace(m['literal'],m['replacement'],1).encode('utf-8'))
t=time.monotonic()
try:
    p=subprocess.Popen(['cargo','nextest','run','--locked','--offline','--profile','default','--retries','0','--no-fail-fast'],cwd=TREE,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8',errors='replace')
    try:
        out,_=p.communicate(timeout=2700); to=False
    except subprocess.TimeoutExpired:
        subprocess.run(['taskkill','/PID',str(p.pid),'/T','/F'],capture_output=True); out,_=p.communicate(); to=True
finally:
    open(path,'wb').write(orig); assert hashlib.sha256(open(path,'rb').read()).hexdigest()==sha0
lines=out.splitlines()
slow=[l.strip()[:160] for l in lines if re.search(r'\bSLOW\b',l)]
tmo=[l.strip()[:160] for l in lines if re.search(r'\bTIMEOUT\b',l)]
fail=[l.strip()[:160] for l in lines if re.search(r'\bFAIL\b',l)]
summ=[l.strip()[:200] for l in lines if 'Summary' in l]
row={'id':ID,'exit':p.returncode,'segundos':round(time.monotonic()-t),'timeout_total':to,'timeouts':tmo[:6],'fallos':fail[:6],'lentos':slow[:8],'resumen':summ,'sha_restaurado':sha0}
open('D:/nexo-work/mut-s3/verify-nextest.jsonl','a',encoding='utf-8').write(json.dumps(row,ensure_ascii=False)+'\n'); print(json.dumps(row,ensure_ascii=False))
