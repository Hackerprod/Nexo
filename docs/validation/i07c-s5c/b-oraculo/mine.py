import json,subprocess,sys
O='C:/projects/Nexo-i07c/scripts/i07c_s5c_oraculo.py'
PF="fn Main(){let p=true;let ar=[1];let aar=[[1]];"
# (id, source, [ (code, leaf_marker, occurrence_after, expected, found) ]) derivado a mano del ADR antes de ver al oraculo
A=lambda t:"Array<%s>"%t
I="I64"
P=[
('P01',PF+"let a=(if p then ar else (if p then 1 else aar));}",[('NX-E1303','1',0,A(I),I)]),
('P02',PF+"let a=(if p then 1 else (if p then ar else 2));}",[('NX-E1303','ar',2,I,A(I))]),
('P03',PF+"var r=[];let a=(if p then r else (if p then ar else 1));}",[('NX-E1303','1',1,A(I),I)]),
('P04',PF+"var r=[];let a=(if p then r else (if p then 1 else ar));}",[('NX-E1303','1',1,A(I),I)]),
('P05',PF+"var r=[];let a=(if p then r else (if p then 1 else true));}",[('NX-E0103','[]',0,None,None)]),
('P06',PF+"var r=[];let a=(if p then r else (if p then 1 else true));r=[2];}",[('NX-E1303','1',1,A(I),I)]),
('P07',PF+"let a=(if p then ar else aar);}",[('NX-E1303','aar',1,A(I),A(A(I)))]),
('P08',PF+"var r=[];let a=(if p then ar else r);r=[true];}",[('NX-E0102','true',0,I,'Bool')]),
('P09',PF+"var r=[];let a=(if p then ar else r);}",[]),
('P10',PF+"var r=[];let a=(if p then r else ar);}",[]),
('P11',PF+"var q=[];var r=[];let a=(if p then q else (if p then r else 1));}",[('NX-E0103','[]',0,None,None),('NX-E0103','[]',1,None,None)]),
('P12',PF+"var q=[];var r=[];let a=(if p then q else (if p then r else 1));q=[[ar]];}",[('NX-E1303','1',1,A(A(A(I))),I)]),
('P13',PF+"let a=(if p then (if p then ar else 1) else aar);}",[('NX-E1303','1',1,A(I),I),('NX-E1303','aar',1,A(I),A(A(I)))]),
('P14',PF+"let a=(if p then 1 else 2);let b=(if p then ar else 3);}",[('NX-E1303','3',0,A(I),I)]),
('P15',PF+"var r=[];let a=(if p then ar else (if p then aar else r));}",[('NX-E1303','aar',1,A(I),A(A(I)))]),
('P16',PF+"var r=[];let a=(if p then ar else (if p then aar else r));r=[true];}",[('NX-E1303','aar',1,A(I),A(A(I))),('NX-E0102','true',0,I,'Bool')]),
('P17',PF+"var r=[];let a=(if p then r else (if p then ar else aar));}",[('NX-E1303','aar',1,A(I),A(A(I)))]),
('P18',PF+"var r=[];r=[1];let a=(if p then r else 1);}",[('NX-E1303','1',2,A(I),I)]),
]
def pos(src,marker,k):
    # k-esima aparicion del marcador tras 'let a'/'let b' o 'var r' ... : se usa k contando desde el primer 'let a' (o 'r=[' para asignaciones)
    base=src.index('let a=') if 'let a=' in src else 0
    if marker=='true' and src.count('r=[true]'): 
        i=src.index('r=[true]')+3; return (i,i+4)
    if marker=='[]':
        i=-1
        for _ in range(k+1): i=src.index('[]',i+1)
        return (i,i+2)
    if marker=='3' and 'let b=' in src:
        i=src.index('else 3')+5; return (i,i+1)
    i=base-1
    for _ in range(k+1): i=src.index(marker,i+1) if marker not in('1','2','3') else src.index(marker,i+1)
    return (i,i+len(marker))
res={}
for n,s,exp in P:
    out=json.loads(subprocess.run([sys.executable,'-B',O,'--source',s],capture_output=True,text=True,cwd='C:/projects/Nexo-i07c').stdout)
    got=sorted((d['code'],tuple(d['primary']),d['expected_type'],d['found_type']) for d in out['diagnostics'])
    res[n]=(s,got,out)
json.dump({n:(s,[list(map(str,g)) for g in got]) for n,(s,got,_) in res.items()},open('oraculo_out.json','w'),indent=0)
for n,s,exp in P:
    print(n,s[len(PF):]); print('   oraculo:',res[n][1])
