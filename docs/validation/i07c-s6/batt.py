import subprocess, sys, os, itertools, json, hashlib, concurrent.futures as cf
BASE='C:/nexo-work/val/s6mut/bin/base.exe'
MUT=sys.argv[1]
WORK='C:/nexo-work/tmp/batt'
os.makedirs(WORK,exist_ok=True)
DV='(if true then {return;} else {return;})'
PROFILES=['f0-0002-b-v2','f0-0003-b-v1']

def run(exe,verb,path,profile):
    try:
        p=subprocess.run([exe,verb,path,'--profile',profile,'--format','json'],capture_output=True,timeout=30)
        return p.returncode,p.stdout.decode('utf-8','replace'),p.stderr.decode('utf-8','replace')
    except subprocess.TimeoutExpired:
        return 'T','',''

def gen():
    srcs=[]
    G=lambda e:'('+e+')'
    GG=lambda e:'(('+e+'))'
    wrap=[lambda e:e,G,GG]
    atoms=['true','1','v','w','[1]','[true]','[]','Len(a)','a[0]','(if true then 1 else 2)','()','false','b']
    # prelude gives v (deferred I64), w (Bool), a array, b Bool
    pre='var v=%s;v=1;let w=true;var a=[1];let b=false;'%DV
    ctx=[
      'let x:Bool=@;', 'let x:I64=@;', 'let x=@+1;','let x=1+@;','let x=@==1;','let x=1==@;','let x=@==true;','let x=true==@;',
      'let x=@<1;','let x=1<@;','let x=[1,@];','let x=[@,1];','let x=[true,@];','let x=[@,true];','let x=[@];',
      'let x=-@;','let x=not @;','let x=@ and true;','let x=true or @;','if @ {}','while @ {break;}','let x=a[@];','a[0]=@;','v=@;','w=@;',
      'let x=Len(@);','Print(@);','let x=Some(@);','let x:Option<I64>=Some(@);','match @ {_=>{}}','let x=if @ then 1 else 2;','let x=if true then @ else 1;',
      'let x=if true then 1 else @;','for i in @ {}','let x:Array<I64>=@;','let x:Array<Bool>=[@];','let x=@+@;','let x=@==@;','let x=[@,@];',
      'var q=@;q=1;','var q=@;q=true;','var q=[@];q=[1];','let x:Bool=@+1;','let x=(@)+1;','let x=@;let y:Bool=x;','let x=@;let y:I64=x;',
    ]
    for c in ctx:
        for at in atoms:
            for k,wp in enumerate(wrap):
                e=wp(at)
                srcs.append((f'{c}|{at}|{k}','fn Main(){'+pre+c.replace('@',e)+'}'))
            # nested group of whole contexts: groups around binary subexpr
    # operators with grouped sides, mixed atoms
    ops=['+','-','*','==','!=','<','and','or']
    for op in ops:
        for l,r in itertools.product(['true','1','v','w',DV,'[1]','Len(a)'],repeat=2):
            for gl,gr in itertools.product(range(3),range(3)):
                srcs.append((f'op{op}|{l}|{r}|{gl}{gr}','fn Main(){'+pre+f'let x={wrap[gl](l)} {op} {wrap[gr](r)};'+'}'))
                srcs.append((f'opnest{op}|{l}|{r}|{gl}{gr}','fn Main(){'+pre+f'let x=({wrap[gl](l)} {op} {wrap[gr](r)}) {op} 1;'+'}'))
    return srcs

def one(item):
    name,src=item
    h=hashlib.md5(src.encode()).hexdigest()
    path=f'{WORK}/{h}.n'
    open(path,'w').write(src)
    out=[]
    for pr in PROFILES:
        for verb in ('check','run'):
            b=run(BASE,verb,path,pr); m=run(MUT,verb,path,pr)
            if b!=m: out.append((name,src,pr,verb,b,m))
    return out

if __name__=='__main__':
    items=gen()
    if len(sys.argv)>2: items=items[:int(sys.argv[2])]
    print('programas',len(items),flush=True)
    diffs=[]
    with cf.ThreadPoolExecutor(8) as ex:
        for r in ex.map(one,items):
            diffs.extend(r)
    print('diferencias',len(diffs))
    json.dump(diffs,open('C:/nexo-work/val/s6mut/diffs_%s.json'%os.path.basename(MUT),'w'),indent=1,ensure_ascii=False)
    for d in diffs[:10]:
        print(d[0],d[2],d[3]); print(' base',d[4][0],d[4][1][:600]); print(' mut ',d[5][0],d[5][1][:600])
