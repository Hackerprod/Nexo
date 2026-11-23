import json,subprocess,sys
O='C:/projects/Nexo-i07c/scripts/i07c_s5c_oraculo.py'
R=json.load(open('fuentes-recuperadas.json'))
D='C:/projects/Nexo-i07c/docs/validation/i07c-s5a/d-repros/'
for n in ('R1','R2','R2b','R3b','R4'): R[n]=open(D+n+'.n').read()
A=lambda t:"Array<%s>"%t
I="I64"; A3=A(A(A(I)))
# derivado a mano del ADR antes de comparar con el oraculo
X={
'R1':[('NX-E1303','then 1 else',5,1,A3,I)],
'R2':[('NX-E1303','else 1)',5,1,A3,I)],
'R2b':[('NX-E1303','else 1)',5,1,A3,I)],
'R3b':[('NX-E1303','then 1 else',5,1,A3,I)],
'R4':[('NX-E1303','then 1 else',5,1,A3,I)],
'X_flat3':[('NX-E1303','then 1 else',5,1,A3,I)],
'Y_pend_if2':[('NX-E0103','var r=[]',6,2,None,None)],
'X_mix3':[('NX-E1303','then 1 else',5,1,A3,I)],
'X_R4asg_r':[('NX-E1303','then 1 else',5,1,A3,I)],
'Y_conc_if':[('NX-E1303','then 1 else',5,1,A(I),I)],
'Y_pend_if_asg':[('NX-E1303','then 1 else',5,1,A(I),I)],
'A_ctrl':[('NX-E1303','[r]))',0,3,A(A(I)),A3)],
'A':[('NX-E1303','[r]))',0,3,A(A(I)),A3),('NX-E0102','r=[3]',3,1,A(I),I)],
'A_noasg':[('NX-E1303','[r]))',0,3,A(A(I)),A3)],
'X_R1pre':[('NX-E1303','then 1 else',5,1,A3,I)],
}
ok=0
for n,exp in X.items():
    s=R[n]
    E=[]
    for code,snip,off,ln,e,f in exp:
        i=s.index(snip)+off; E.append((code,(i,i+ln),e or '',f or ''))
    out=json.loads(subprocess.run([sys.executable,'-B',O,'--source',s],capture_output=True,text=True,cwd='C:/projects/Nexo-i07c').stdout)
    G=[(d['code'],tuple(d['primary']),d['expected_type'] or '',d['found_type'] or '') for d in out['diagnostics']]
    same=sorted(E)==sorted(G); ok+=same
    print(n,'OK' if same else 'DIF')
    if not same: print('  mia',sorted(E),'\n  orc',sorted(G))
print(ok,'/',len(X))
