import json,os,subprocess,shutil,time,sys
from concurrent.futures import ThreadPoolExecutor
ROOT='C:/nexo-work/hunt'; TREE='C:/nexo-work/val/hunt-w0'; TARGET='C:/nexo-work/target-hunt0'
env=dict(os.environ,CARGO_TARGET_DIR=TARGET,CARGO_PROFILE_RELEASE_DEBUG='0',CARGO_INCREMENTAL='1',TEMP='C:/nexo-work/tmp',TMP='C:/nexo-work/tmp')
t=time.monotonic()
p=subprocess.run(['cargo','build','--release','--locked','--offline','--bin','nexo'],cwd=TREE,env=env,capture_output=True,text=True,encoding='utf-8',errors='replace')
print('build base',p.returncode,round(time.monotonic()-t),'s',flush=True)
assert p.returncode==0,p.stderr[-300:]
shutil.copy(TARGET+'/release/nexo.exe',ROOT+'/base.exe')
exe=ROOT+'/base.exe'
N=len(json.load(open('D:/nexo-work/mut-s3/hunt-corpus.json',encoding='utf-8')))
def one(a):
    mode,i=a
    f=f'{ROOT}/corpus/p{i:05d}.n'
    q=subprocess.Popen([exe,mode,'--profile','f0-0003-b-v1','--format','json',f],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',errors='replace')
    try:
        o,e=q.communicate(timeout=15); return f'{mode}:{i}',(q.returncode,o.replace(f,'S'))
    except subprocess.TimeoutExpired:
        subprocess.run(['taskkill','/PID',str(q.pid),'/T','/F'],capture_output=True); return f'{mode}:{i}','TIMEOUT'
base={}
t=time.monotonic()
with ThreadPoolExecutor(8) as ex:
    for mode in ('check','run'):
        for k,v in ex.map(one,[(mode,i) for i in range(N)]): base[k]=v
        print(mode,'listo',round(time.monotonic()-t),'s',flush=True)
json.dump(base,open(ROOT+'/base.json','w',encoding='utf-8'))
to=sum(1 for v in base.values() if v=='TIMEOUT'); print('base: salidas',len(base),'timeouts',to)
