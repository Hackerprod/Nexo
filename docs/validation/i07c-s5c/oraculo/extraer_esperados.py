"""Extrae los esperados de la matriz (columna «Esperado», derivados por A y B) de `rec_all.py` sin ejecutar ningun binario.
Uso: python -B extraer_esperados.py  ->  escribe matriz-esperados.json junto a este archivo."""
import json, os, re, sys
here = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(here, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(root, "scripts"))
import i07c_s5c_oraculo as O
M = O.load_sources()
txt = open(os.path.join(here, "..", "rec_all.py"), encoding="utf-8").read()
i = txt.index("def S(n,s,k=0):")
j = txt.index("tot={k:0")
ns = {"M": M, "json": json}
exec(txt[i:j], ns)
E = ns["E"]
json.dump({k: sorted(v) for k, v in E.items()}, open(os.path.join(here, "matriz-esperados.json"), "w", encoding="utf-8"),
          indent=1, ensure_ascii=False)
print(len(E), "filas con esperado")
