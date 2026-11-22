"""I07c-S6-COR r2 directed sweep (deterministic, no seed): every call-target form x the four effect
contracts x six contexts x 0..3 groups. Counts diagnostics whose primary or call step is empty.
Usage: python -B sweep_targets.py <nexo.exe> <scratch dir>. Expected result: no EMPTY counts."""
import json,subprocess,os,itertools,concurrent.futures as cf,collections,hashlib,sys
NEXO=sys.argv[1]
SCRATCH=sys.argv[2]
EFF={'no_io':'Print(1);','no_alloc':'let a=[1];','no_block':'let r=MockBlockingOpen(1);','no_suspend':'MockUnknown();'}
def tgt(kind,eff,g):
    w=lambda s:'('*g+s+')'*g
    if kind=='literal': return w('fn (){%s}'%eff)+'()'
    if kind=='literal_bool': return w('fn ()->Bool{%s return true;}'%eff)+'()'
    if kind=='named': return w('f')+'()'
    if kind=='fn': return w('H')+'()'
    if kind=='literal_arg': return w('fn (x:view I64){%s}'%eff)+'(1)'
    if kind=='literal_capture': return w('fn [view v](){%s}'%eff)+'()'
    if kind=='callresult': return w('K()')+'()'
    if kind=='index': return w('arr[0]')+'()'
    if kind=='ifexpr': return w('if true then {f} else {f}')+'()'
    if kind=='unit': return w('()')+'()'
    if kind=='if': return w('if true then f else f')+'()'
    if kind=='move': return w('move f')+'()'
    if kind=='edit': return w('edit f')+'()'
    if kind=='view': return w('view f')+'()'
    if kind=='block': return w('{f}')+'()'
    if kind=='blockvalue': return w('{value:f;}')+'()'
    if kind=='match': return w('match true{_=>f}')+'()'
    if kind=='literal_nested': return w('fn (){%s (fn (){%s})();}'%(eff,eff))+'()'
    if kind=='literal_calls_f': return w('fn (){f();}')+'()'
    if kind=='field': return w('f.x')+'()'
    if kind=='neg': return w('-f')+'()'
    if kind=='not': return w('!f')+'()'
    if kind=='binary': return w('f+f')+'()'
    if kind=='some': return w('Some(f)')+'()'
    if kind=='none': return w('None')+'()'
    if kind=='arrlit': return w('[f]')+'()'
    if kind=='indexlit': return w('[f][0]')+'()'
    if kind=='callnested': return w('f()')+'()'
    if kind=='int': return w('1')+'()'
KINDS=['if','move','edit','view','block','blockvalue','match','literal_nested','literal_calls_f','field','neg','not','binary','some','none','arrlit','indexlit','callnested','literal','literal_bool','named','fn','literal_arg','literal_capture','callresult','index','ifexpr','unit','int']
def prog(contract,kind,ctx,g):
    eff=EFF[contract]
    call=tgt(kind,eff,g)
    heads='fn H(){%s}fn K()->I64{return 1;}'%eff if kind in ('fn','callresult') else ''
    pre=('let v=1;' if kind=='literal_capture' else '')+('let f=fn (){%s};'%eff if kind in ('named','ifexpr','if','move','edit','view','block','blockvalue','match','literal_calls_f','field','neg','not','binary','some','none','arrlit','indexlit','callnested') else '')+('let arr=[fn (){%s}];'%eff if kind=='index' else '')
    body={'stmt':call+';','guard':'match true{true if %s=>{},_=>{}}'%call,'closure':'let h=fn (){%s;};h();'%call,'for':'for i in 0..<2{%s;}'%call,'arg':'Drop(%s);'%call,'let':'let z=%s;'%call,'nested':'let h=fn (){let g=fn (){%s;};g();};h();'%call}[ctx]
    return heads+'fn Main() with %s{%s%s}'%(contract,pre,body)
srcs=[(c,k,x,g,prog(c,k,x,g)) for c in EFF for k in KINDS for x in ['stmt','guard','closure','arg','let','nested'] for g in (0,1,2,3)]
def work(a):
    i,(c,k,x,g,s)=a
    out=[]
    for pf in ('J','F5'):
        r=p3.run(NEXO,pf,s) if False else None
    return None
res=collections.Counter(); ex={}
def one(a):
    i,(c,k,x,g,s)=a
    d=SCRATCH+'/s3_%d'%(i%10); os.makedirs(d,exist_ok=True)
    f=d+'/case.n'; open(f,'w').write(s)
    out=[]
    for pf in ('f0-0003-b-v1','f0-0005-b-v1'):
        r=subprocess.run([NEXO,'check','case.n','--profile',pf,'--format','json'],cwd=d,capture_output=True,text=True)
        try: j=json.loads(r.stdout)
        except Exception: out.append((pf,'BAD')); continue
        for dg in j['diagnostics']:
            ps=dg['primary_span']; empty_p=ps['start']==ps['end']
            emp_c=[c_ for c_ in dg['cause_chain'] if c_.get('span') and c_['span']['start']==c_['span']['end']]
            out.append((pf,dg['rule'],empty_p,len(emp_c)))
    return (c,k,x,g,s,out)
with cf.ThreadPoolExecutor(6) as ex_:
    rows=list(ex_.map(one,list(enumerate(srcs))))
agg=collections.Counter();first={}
for c,k,x,g,s,out in rows:
    for o in out:
        if o[1]=='BAD': continue
        key=(k,x,'grouped' if g else 'bare',o[1],'EMPTY' if (o[2] or o[3]) else 'ok')
        agg[key]+=1; first.setdefault(key,s)
print(len(rows),'programs')
agg2=collections.Counter(); tot=collections.Counter()
for c,k,x,g,s_,out in rows:
    for o in out:
        if o[1]=='BAD': continue
        tot[(k,x,g>0)]+=1
        if o[2] or o[3]: agg2[(k,x)]+=1
print('programs with an empty primary/step, by target kind:')
bykind=collections.Counter(); rej=collections.defaultdict(collections.Counter)
for c,k,x,g,s_,out in rows:
    for o in out:
        if o[1]=='BAD': continue
        if o[2] or o[3]: bykind[k]+=1
        rej[k][o[1]]+=1
for k in KINDS:
    print(' ',k,'EMPTY:',bykind.get(k,0),'| rules:',dict(rej[k]))
print(len(rows),'programs; sha256',hashlib.sha256(chr(0).join(r[4] for r in rows).encode()).hexdigest())
