import json,sys,os,subprocess,threading
from concurrent.futures import ThreadPoolExecutor
import h
tl=threading.local()
def runbin(exe,src):
    if not hasattr(tl,'f'):
        tl.f=os.path.join(h.W,'t%d.n'%threading.get_ident())
    open(tl.f,'w',newline='').write(src)
    p=subprocess.run([exe,'check','--format','json','--profile','f0-0002-b-v2',tl.f],capture_output=True,text=True,timeout=90)
    try: return h.sig_bin(json.loads(p.stdout))
    except Exception: return 'ERR'
progs=json.load(open(sys.argv[1]))
def one(src):
    return dict(main=runbin(h.EXE%'base',src),cand=runbin(h.EXE%'cand',src),
                P=h.orc(h.orc_P,src),N=h.orc(h.orc_N,src),N2=h.orc(h.orc_N2,src))
with ThreadPoolExecutor(6) as ex: res=list(ex.map(one,progs))
json.dump([[p,r] for p,r in zip(progs,res)],open(sys.argv[2],'w'),default=str)
