import json,subprocess,sys
O='C:/projects/Nexo-i07c/scripts/i07c_s5c_oraculo.py'
PF="fn Main(){let p=true;let ar=[1];let aar=[[1]];"
A=lambda t:"Array<%s>"%t
I="I64"
# (id, sufijo, [(codigo, snippet unico, desplazamiento, longitud, esperado, encontrado)]) derivado a mano del ADR ANTES de ejecutar el oraculo
P=[
('P01',"let a=(if p then ar else (if p then 1 else aar));}",[('NX-E1303','then 1 else',5,1,A(I),I)]),
('P02',"let a=(if p then 1 else (if p then ar else 2));}",[('NX-E1303','then ar else',5,2,I,A(I))]),
('P03',"var r=[];let a=(if p then r else (if p then ar else 1));}",[('NX-E1303','else 1)',5,1,A(I),I)]),
('P04',"var r=[];let a=(if p then r else (if p then 1 else ar));}",[('NX-E1303','then 1 else',5,1,A(I),I)]),
('P05',"var r=[];let a=(if p then r else (if p then 1 else true));}",[('NX-E0103','var r=[]',6,2,None,None)]),
('P06',"var r=[];let a=(if p then r else (if p then 1 else true));r=[2];}",[('NX-E1303','then 1 else',5,1,A(I),I)]),
('P07',"let a=(if p then ar else aar);}",[('NX-E1303','else aar',5,3,A(I),A(A(I)))]),
('P08',"var r=[];let a=(if p then ar else r);r=[true];}",[('NX-E0102','r=[true]',3,4,I,'Bool')]),
('P09',"var r=[];let a=(if p then ar else r);}",[]),
('P10',"var r=[];let a=(if p then r else ar);}",[]),
('P11',"var q=[];var r=[];let a=(if p then q else (if p then r else 1));}",[('NX-E0103','var q=[]',6,2,None,None),('NX-E0103','var r=[]',6,2,None,None)]),
('P12',"var q=[];var r=[];let a=(if p then q else (if p then r else 1));q=[[ar]];}",[('NX-E1303','else 1)',5,1,A(A(A(I))),I)]),
('P13',"let a=(if p then (if p then ar else 1) else aar);}",[('NX-E1303','else 1)',5,1,A(I),I),('NX-E1303','else aar',5,3,A(I),A(A(I)))]),
('P14',"let a=(if p then 1 else 2);let b=(if p then ar else 3);}",[('NX-E1303','else 3)',5,1,A(I),I)]),
('P15',"var r=[];let a=(if p then ar else (if p then aar else r));}",[('NX-E1303','then aar else',5,3,A(I),A(A(I)))]),
('P16',"var r=[];let a=(if p then ar else (if p then aar else r));r=[true];}",[('NX-E1303','then aar else',5,3,A(I),A(A(I))),('NX-E0102','r=[true]',3,4,I,'Bool')]),
('P17',"var r=[];let a=(if p then r else (if p then ar else aar));}",[('NX-E1303','else aar)',5,3,A(I),A(A(I)))]),
('P18',"var r=[];r=[1];let a=(if p then r else 1);}",[('NX-E1303','else 1)',5,1,A(I),I)]),
]
ok=0
for n,suf,exp in P:
    s=PF+suf
    E=[]
    for code,snip,off,ln,e,f in exp:
        i=s.index(snip)+off; E.append((code,(i,i+ln),e,f))
    out=json.loads(subprocess.run([sys.executable,'-B',O,'--source',s],capture_output=True,text=True,cwd='C:/projects/Nexo-i07c').stdout)
    G=[(d['code'],tuple(d['primary']),d['expected_type'],d['found_type']) for d in out['diagnostics']]
    same=sorted(E)==sorted(G)
    ok+=same
    print(n,'OK ' if same else 'DIF',suf)
    if not same: print('   mia    :',sorted(E)); print('   oraculo:',sorted(G))
print(ok,'/',len(P))
