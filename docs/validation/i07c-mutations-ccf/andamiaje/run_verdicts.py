import json,hashlib,os,subprocess,sys,time
TREE='C:/nexo-work/val/i07c-verify'
SCR='C:/projects/Nexo-i07c/scripts'
POP={m['id']:m for m in json.load(open('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json'))['mutations']}
IDS=json.load(open('D:/nexo-work/mut-s3/fullsuite-ids.json'))
OUT='D:/nexo-work/mut-s3/verdict'
os.makedirs(OUT,exist_ok=True)
env=dict(os.environ)
for i in IDS:
    m=POP[i]; rel=m['file']
    src=open(f'{TREE}/{rel}','rb').read()
    text=src.decode('utf-8')
    assert text.count(m['literal'])==1,i
    mut=text.replace(m['literal'],m['replacement'],1).encode('utf-8')
    patch={'file':rel,'before':m['literal'],'after':m['replacement'],'occurrence':0,
           'sha_initial':hashlib.sha256(src).hexdigest(),'sha_mutated':hashlib.sha256(mut).hexdigest()}
    pj=f'{OUT}/{i}.patch.json'; json.dump(patch,open(pj,'w',encoding='utf-8'),ensure_ascii=False)
    t=time.time()
    cmd=['python',f'{SCR}/heavy-lock.py','run','--session',f'i07c-mut-verdict-{i}','--','python',f'{SCR}/mutant-verdict.py','--tree',TREE,'--patch',pj,'--id',i,'--output',f'{OUT}/{i}.json','--timeout','2400']
    p=subprocess.run(cmd,env=env,capture_output=True,text=True,encoding='utf-8',errors='replace')
    row={'id':i,'exit':p.returncode,'segundos':round(time.time()-t),'stderr':p.stderr[-300:]}
    try:
        r=json.load(open(f'{OUT}/{i}.json',encoding='utf-8')); row['veredicto']=r.get('verdict'); row['perfiles']=[(x['profile'],x['verdict'],x.get('test')) for x in r['profiles']]
    except Exception as e: row['veredicto']='SIN_SALIDA'; row['error']=str(e)[:100]
    open(f'{OUT}/resumen.jsonl','a',encoding='utf-8').write(json.dumps(row,ensure_ascii=False)+'\n')
    print(json.dumps(row,ensure_ascii=False),flush=True)
open(f'{OUT}/resumen.jsonl','a',encoding='utf-8').write(json.dumps({'fin':True})+'\n')
