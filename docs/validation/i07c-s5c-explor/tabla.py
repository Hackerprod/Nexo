import json
c=json.load(open('corpus-congelado.json'));o=json.load(open('oraculo-res.json'));r=json.load(open('res-all.json'))
core=lambda s: json.dumps([d[:4] for d in s]); full=lambda s: json.dumps(s)
FL={'abl':'1a = ALT-2 S1 + retorno con testigo (pendfa) [binario de C]','A':'1a (xv)','B':'1a + retfa','C':'1a + retfa + retract','F1':'1c = 1a + retfa + retract + anchw','H1':'1d = F1 + nojoin',
'v6':'V6 original (2296dc8)','G1':'2a = F1 + V6 completo','H2':'2b = H1 + V6 completo','I1':'2c = H1 + V6 solo rejected_assignments','J2':'2d = I1 + atomic0','J1':'2e = I1 + atomic0 + atomic','K2':'2f = 2d + arrx','K1':'2g = 2e + arrx',
'F2':'3a = F1 + sin evidencia sembrada (noseed)','F3':'3b = F1 + sin forma sembrada (noshape)','D':'1x = C + anchored=false (unanch)'}
print('| variante | nivel | acierta ambos | (A) regresiones | (B) arreglos | (C) fallan igual | (D) fallan distinto |')
print('|---|---|---|---|---|---|---|')
for v in ['abl','B','C','F1','H1','D','v6','G1','H2','I1','J2','J1','K2','K1','F2','F3']:
    for nm,f in (('core',core),('full',full)):
        A=B=C=D=ok=0
        for i in range(len(c)):
            if o[i] is None: continue
            bo=f(r['main'][i])==f(o[i]); co=f(r[v][i])==f(o[i])
            if bo and not co: A+=1
            elif not bo and co: B+=1
            elif bo and co: ok+=1
            elif f(r['main'][i])==f(r[v][i]): C+=1
            else: D+=1
        print('| %s | %s | %d | %d | %d | %d | %d |'%(FL.get(v,v),nm,ok,A,B,C,D))
