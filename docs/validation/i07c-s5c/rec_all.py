import json,subprocess,os
B='../cand-bin/'
bins={'main':B+'base-release.exe','v6':B+'cand-release.exe','c2':'C:/nexo-work/target-a-s5c/release/nexo.exe'}
base='../../docs/validation/i07c-s5a/'
M={}
for n in ['R1','R1twin','R2','R2b','R3','R3b','R3c','R4','R5']: M[n]=open(base+'d-repros/%s.n'%n).read()
for n in ['P1','P2','P3','P4','P4pos','c3','w1','w3']: M[n]=open(base+'limites/%s.n'%n).read()
for c in json.load(open('../../docs/validation/i07c-twin/casos.json')):
    if c['id'] in('O01','O03','O07','O08'): M[c['id']]=c['reduced']
Pf="fn Main(){let p=true;let ar=[1];"
M.update({
'A':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}",
'A_ctrl':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[[3]];}",
'A_noasg':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));}",
'A_reord':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then [r] else aar));r=[3];}",
'B_qr':"fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];}",
'B_rq':"fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];r=[3];q=[3];}",
'F06':"fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then r else ar));r=[3];}",
'M2':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}",
'M2_noasg':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));}",
'M2_ctrl':"fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=[[[3]]];}",
'DREF_valid':"fn Main(){let aar=[[1]];var r=[];let a=(if false then r[0] else aar);r=[[[3]]];}",
'X_R1na':Pf+"let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));}",
'X_R1ord':Pf+"let r=[[[1]]];var q=[];let a0=(if p then q else (if p then r else 1));}",
'X_R1pre':Pf+"let r=[[[1]]];var q=[];q=[[ar]];let a0=(if p then q else (if p then 1 else r));}",
'X_R1inc':Pf+"let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));q=[3];}",
'X_R2na':Pf+"var r=[];var q=[];let a0=(if p then q else (if p then r else 1));}",
'X_R4na':Pf+"var r=[];var q=[];let a0=(if p then q else (if p then 1 else r));}",
'X_R4asg_r':Pf+"var r=[];var q=[];let a0=(if p then q else (if p then 1 else r));q=[[ar]];r=[[ar]];}",
'X_flat3':Pf+"var q=[];let a0=(if p then q else (if p then 1 else (if p then 2 else 3)));q=[[ar]];}",
'X_mix3':Pf+"var q=[];var r=[];let a0=(if p then q else (if p then r else (if p then 1 else 2)));q=[[ar]];}",
'X_nest_first':Pf+"var q=[];let a0=(if p then (if p then q else 1) else 2);q=[[ar]];}",
'Y_conc_if':"fn Main(){let p=true;let r=[1];let a0=(if p then r else (if p then 1 else true));}",
'Y_pend_if':Pf+"var r=[];let a0=(if p then r else (if p then 1 else true));}",
'Y_pend_if2':Pf+"var r=[];let a0=(if p then r else (if p then true else 1));}",
'Y_pend_if_asg':"fn Main(){let p=true;var r=[];let a0=(if p then r else (if p then 1 else true));r=[2];}",
'W0':"fn Main(){let ar=[1];var r=[];var q=[];let a=(if true then true else q);Print([r,q]);q=[[1<2]];r=[];}",
'W0nounion':"fn Main(){let ar=[1];var r=[];var q=[];let a=1;Print([r,q]);q=[[1<2]];r=[];}",
})
for k,v in M.items(): open('m/%s.n'%k,'w').write(v)
def run(b,n):
    p=subprocess.run([b,'check','--format','json','--profile','f0-0002-b-v2','m/%s.n'%n],capture_output=True)
    j=json.loads(p.stdout); out=[]
    for d in j.get('diagnostics') or []:
        pr=d.get('primary_span') or d.get('span'); rel=' '.join('exp=[%d,%d)'%(r['span']['start'],r['span']['end']) for r in d.get('related_spans',[]) if r.get('role')=='expected_type')
        dt=d.get('data',{}); sc=d.get('subcause')
        out.append('%s%s [%d,%d) %s|%s %s'%(d['code'][3:],'/'+sc[:4] if sc else '',pr['start'],pr['end'],(dt.get('expected_type') or '').replace('Array<','A<'),(dt.get('found_type') or '').replace('Array<','A<'),rel))
    return sorted(out)
def S(n,s,k=0):
    t=M[n]; i=-1
    for _ in range(k+1): i=t.index(s,i+1)
    return '[%d,%d)'%(i,i+len(s))
A1,A2,A3,A4='A<I64>','A<A<I64>>','A<A<A<I64>>>','A<A<A<A<I64>>>>'
E={}
def put(n,*l): E[n]=sorted(l)
put('R1','E1303 [91,92) %s|I64 exp=[73,74)'%A3)
put('R1twin','E1303 [95,96) %s|I64 exp=[77,78)'%A3)
put('R2','E1303 [93,94) %s|I64 exp=[68,69)'%A3)
put('R2b','E1303 [98,99) %s|I64 exp=[73,74)'%A3)
put('R3','E1303 [99,104) I64|%s exp=[81,82)'%A4)
put('R3b','E1303 [77,78) %s|I64 exp=[59,60)'%A3)
put('R3c','E1303 [82,83) %s|I64 exp=[64,65)'%A3)
put('R4','E1303 [94,95) %s|I64 exp=[76,77)'%A3)
put('X_R1na','E1303 [91,92) %s|I64 exp=[73,74)'%A3)
put('X_R1ord','E1303 [98,99) %s|I64 exp=[73,74)'%A3)
put('X_R2na','E0103 [38,40) | ','E0103 [47,49) | ')
put('X_R4na','E0103 [38,40) | ','E0103 [47,49) | ')
put('X_R1pre','E1303 [100,101) %s|I64 exp=[82,83)'%A3)
put('X_R4asg_r','E1303 [86,87) %s|I64 exp=[68,69)'%A3)
put('X_flat3','E1303 [77,78) %s|I64 exp=[59,60)'%A3)
i1=M['X_mix3'].index('then 1 else')+5
put('X_mix3','E1303 [%d,%d) %s|I64 exp=%s'%(i1,i1+1,A3,S('X_mix3','q',1)))
put('Y_pend_if','E0103 %s | '%S('Y_pend_if','[]'))
put('Y_pend_if2','E0103 %s | '%S('Y_pend_if2','[]'))
put('Y_conc_if','E1303 [67,68) A<I64>|I64 exp=[49,50)')
put('Y_pend_if_asg','E1303 [66,67) A<I64>|I64 exp=[48,49)')
t=M['X_R1inc']; i3=t.index('[3]')+1
put('X_R1inc','E1303 [91,92) %s|I64 exp=[73,74)'%A3,'E0102/type [%d,%d) %s|I64 exp=[98,99)'%(i3,i3+1,A2))
put('X_nest_first','E1303 [77,78) %s|I64 exp=[70,71)'%A3,'E1303 [85,86) %s|I64 exp=[60,78)'%A3)
put('A','E1303 [83,86) %s|%s exp=[53,54)'%(A2,A3),'E0102/type [92,93) A<I64>|I64 exp=[74,77)')
put('A_ctrl','E1303 [83,86) %s|%s exp=[53,54)'%(A2,A3))
put('A_noasg','E1303 [83,86) %s|%s exp=[53,54)'%(A2,A3))
put('A_reord','E1303 [74,77) %s|%s exp=[53,54)'%(A2,A3),'E0102/type [92,93) A<I64>|I64 exp=[83,86)')
put('B_qr','E1303 [63,64) Bool|%s exp=[56,57)'%A2,'E0102/type [83,84) A<I64>|I64 exp=[76,77)')
put('B_rq','E1303 [63,64) Bool|%s exp=[56,57)'%A2,'E0102/type [89,90) A<I64>|I64 exp=[76,77)')
put('F06','E1303 [76,77) A<I64>|%s exp=[51,55)'%A2,'E0102/type [91,92) A<I64>|I64 exp=[83,85)')
put('M2','E1303 [77,78) %s|%s exp=[53,57)'%(A2,A3),'E0102/type [92,95) %s|%s exp=[84,87)'%(A3,A2))
put('M2_ctrl','E1303 [77,78) %s|%s exp=[53,57)'%(A2,A3))
put('M2_noasg','E1303 [77,78) %s|%s exp=[53,57)'%(A2,A3))
put('DREF_valid')
put('O03','E1303 [212,215) %s|%s exp=[185,186)'%(A2,A3),'E0102/type [277,278) Bool|A<I64> ','E0102/type [358,359) A<I64>|I64 exp=[203,206)')
put('O07','E1303 [149,152) %s|%s exp=[119,120)'%(A2,A3),'E0102/type [159,160) A<I64>|I64 exp=[140,143)')
put('O01','E0102/type [93,99) Bool|%s '%A3,'E0102/not_ [139,144) domain:array|Bool ','E1303 [191,192) %s|%s exp=[166,170)'%(A2,A3),'E0102/type [209,212) %s|%s exp=[198,203)'%(A3,A2))
put('O08','E0102/type [93,94) Bool|%s '%A3,'E0102/type [100,101) Bool|%s '%A3,'E0102/type [107,109) Bool|A<I64> ','E0102/type [130,131) Bool|I64 ','E1303 [137,138) %s|%s exp=[116,120)'%(A2,A3),'E0102/type [155,158) %s|%s exp=[144,147)'%(A3,A2))
put('P1','E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)','E0102/type [109,111) A<A<Bool>>|A<I64> exp=[93,101)')
put('w1','E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)','E0102/type [108,110) A<A<Bool>>|A<I64> exp=[93,100)')
put('c3','E1303 [70,71) Bool|%s exp=[60,64)'%A3,'E0102/type [107,109) %s|A<I64> exp=[93,99)'%A3)
put('W0','E1303 [69,70) Bool|A<A<Bool>> exp=[59,63)')
put('W0nounion')
put('P2','E1303 [101,102) %s|A<A<I64>> exp=[75,80)'%A3,'E0102/type [127,129) %s|A<I64> exp=[119,124)'%A2,'E0102/type [133,134) A<I64>|A<A<I64>> exp=[119,124)')
put('P3','E0102/type [81,82) A<A<I64>>|%s exp=[57,58)'%A4,'E0102/type [99,100) A<A<I64>>|I64 exp=[88,94)')
put('w3','E1303 [101,102) %s|A<A<I64>> exp=[75,80)'%A3,'E0102/type [129,131) A<A<I64>>|A<I64> exp=[119,126)','E0102/type [135,136) A<I64>|A<A<I64>> exp=[119,126)')
tot={k:0 for k in bins}
strip=lambda L:sorted(x.split(' exp=')[0] for x in L)
for n in sorted(E):
    r={k:run(b,n) for k,b in bins.items()}
    row=[]
    for k in bins:
        good=r[k]==E[n]; tot[k]+=good
        row.append('OK' if good else ('NO(I)' if strip(r[k])==strip(E[n]) else 'NO'))
    print('%-14s main=%-5s v6=%-5s c2=%-5s'%(n,*row))
    if n in('X_mix3','W0','P1','c3'): print('    got',{k:r[k] for k in bins})
print(len(E),tot)
