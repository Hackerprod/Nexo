import argparse, hashlib, itertools, json, pathlib, random, subprocess, sys, time
ROOT=pathlib.Path(__file__).resolve().parent
SEED=4071
LEFT=['true','TryAdd(p,1)','AssertEq(p,1)','(true+1)']
RIGHT=['r','[r]','r[0]','([r] or r)','([r] and r)','(if p then [r] else r)','Some([r])','Len([r])']
OPS=['+','-','*','/','%','and','or','==']
WRAP=['{}','[{}]','Some({})','TryAdd({},1)','AssertEq({},1)','(if p then {} else r)','Len({})','({})[0]','Ok({})','({} or r)','({} and r)','{}']
TAIL=['r=1;r=[[1]];','r=[[1]];r=1;','r=[[1]];r=[[1]];','r=1;r=1;']
def sources():
    out=[]
    for l,r,op,w in itertools.product(LEFT,RIGHT,OPS,range(12)):
        decl='var' if w%2==0 else 'let'
        expr=WRAP[w].format('('+l+' '+op+' '+r+')')
        out.append(f'fn Main(){{var p=1;var r=[];{decl} z={expr};{TAIL[w%4]}}}')
    random.Random(SEED).shuffle(out)
    assert len(out)==3072 and len(set(out))==3072
    return out
p=argparse.ArgumentParser(); p.add_argument('action',choices=['freeze','run']); a=p.parse_args()
corpus=ROOT/'family-corpus.json'
genhash=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()
if a.action=='freeze':
    raw=json.dumps(sources(),ensure_ascii=False,separators=(',',':')).encode('utf-8')
    corpus.write_bytes(raw)
    print('generator',genhash,'corpus',hashlib.sha256(raw).hexdigest(),'count',len(sources()),'seed',SEED)
    sys.exit()
raw=corpus.read_bytes(); srcs=json.loads(raw)
assert srcs==sources()
print('generator',genhash,'corpus',hashlib.sha256(raw).hexdigest(),'count',len(srcs),'seed',SEED,flush=True)
binaries={
'debug':{'be8c47c':ROOT/'base-nexo.exe','3100a29':ROOT/'3100a29-debug-nexo.exe','fixed':ROOT/'candidate-nexo.exe'},
'release':{'be8c47c':ROOT/'base-release-nexo.exe','3100a29':ROOT/'3100a29-release-nexo.exe','fixed':ROOT/'candidate-release-nexo.exe'}}
for profile,bs in binaries.items():
    for label,b in bs.items(): print(profile,label,hashlib.sha256(b.read_bytes()).hexdigest(),flush=True)
source=ROOT/'family-current.n'
result={'seed':SEED,'count':len(srcs),'generator_sha256':genhash,'corpus_sha256':hashlib.sha256(raw).hexdigest(),'invocations':0,'differences':[],'faults':[],'types_reached':0}
start=time.time()
for i,s in enumerate(srcs):
    source.write_text(s,encoding='utf-8')
    for profile,bs in binaries.items():
        for mode in ('check','run'):
            observed={}
            for label,binary in bs.items():
                try:
                    cp=subprocess.run([str(binary),mode,'--profile','f0-0005-b-v1','--format','json',str(source)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
                    value=(cp.returncode,cp.stdout,cp.stderr)
                    if any(x in cp.stdout+cp.stderr for x in (b'internal_error',b'InvalidIr',b'panicked at')): result['faults'].append([i,profile,mode,label,'internal'])
                except subprocess.TimeoutExpired:
                    value=('timeout',b'',b'');result['faults'].append([i,profile,mode,label,'timeout'])
                observed[label]=value;result['invocations']+=1
            if observed['be8c47c']!=observed['fixed'] or observed['3100a29']!=observed['fixed']:
                result['differences'].append([i,profile,mode,{k:(v[0],hashlib.sha256(v[1]).hexdigest(),hashlib.sha256(v[2]).hexdigest()) for k,v in observed.items()}])
    if i%256==255:
        print('progress',i+1,'differences',len(result['differences']),'faults',len(result['faults']),'elapsed',round(time.time()-start,1),flush=True)
        (ROOT/'family-fuzz-recheck-progress.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
result['elapsed_seconds']=round(time.time()-start,1)
(ROOT/'family-fuzz-recheck-result.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print('final',result['invocations'],len(result['differences']),len(result['faults']),result['elapsed_seconds'],flush=True)


