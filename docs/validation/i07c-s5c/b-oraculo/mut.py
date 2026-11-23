import subprocess,sys,re,os
src=open('C:/projects/Nexo-i07c/scripts/i07c_s5c_oraculo.py',encoding='utf-8').read()
src=src.replace('ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))','ROOT = "C:/projects/Nexo-i07c"')
M={
'a-absorbe-nada':('        if st["denounced"]:\n            self.notes.append(("absorbida", sp))','        if False:\n            self.notes.append(("absorbida", sp))'),
'b-sin-rollback-atomico':('        for v in self.trail[mark:]:\n            v.ref = None\n            v.wit = None\n        del self.trail[mark:]\n        return False','        return False'),
'c-relacion=hoja':('rel=a_sp, relkind="fixed")','rel=sp, relkind="fixed")'),
'd-sin-occurs-check':None,
'e-sin-aplanado-else':('            self.cont(g.then, st, False)\n            self.cont(g.els, st, False)\n            return','            self.cont(g.then, st, False)\n            return'),
'f-referencia-sin-primera':('        if st["anchor"] is None:\n            # primera continuacion con evidencia valida','        if st["anchor"] is None and not first_pos:\n            # primera continuacion con evidencia valida'),
}
# d: occurs
m=re.search(r'def occurs\(v, t\):\n',src)
for k,v in M.items():
    s=src
    if k.startswith('d-'):
        i=s.index('def occurs(v, t):\n')+len('def occurs(v, t):\n')
        s=s[:i]+'    return False\n'+s[i:]
    else:
        assert v[0] in s,k
        s=s.replace(v[0],v[1],1)
    open('C:/nexo-work/tmp/validador-b-s5cplan/mutorc.py','w',encoding='utf-8').write(s)
    p=subprocess.run([sys.executable,'-B','C:/nexo-work/tmp/validador-b-s5cplan/mutorc.py','--matrix'],capture_output=True,text=True)
    lines=p.stdout.splitlines()
    dis=[l.split()[0] for l in lines if 'DISCREPA' in l]
    last=[l for l in lines if l.startswith('coinciden')]
    print(k,last,'nuevas discrepancias:',[d for d in dis if d not in('W0','P2','w3')], 'err' if p.returncode or p.stderr else '', p.stderr[-150:])
os.remove('C:/nexo-work/tmp/validador-b-s5cplan/mutorc.py')
