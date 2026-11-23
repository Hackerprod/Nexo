"""Lista COMPLETA de diferencias base/variante en las 3.674 fuentes del diferencial de S5a.1 (perfil f0-0002-b-v2, check).

Uso: TEMP=... python -B r1-lista-diferencias.py <base.exe> <variante.exe> <cand.exe> <salida.json>
Cada fila: fuente, diagnosticos completos (codigo, subcausa, primario, expected/found, relaciones) de base, variante y
candidato V6, y las banderas: solo_e0103 (la unica diferencia es E0103 retirado), cambia_{codigo,primario,tipos,relaciones,
orden,cardinalidad}. `derivada` queda en null: la rellena quien derive la fila del ADR (no se infiere de una regla).
"""
import importlib.util, json, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("d", os.path.join(HERE, "..", "i07c-s5", "s5a1-differential.py"))
D = importlib.util.module_from_spec(spec); spec.loader.exec_module(D)
def diags(exe, src, d):
    p = os.path.join(d, "c.n"); open(p, "wb").write(src.encode())
    out = subprocess.run([exe, "check", p, "--profile", "f0-0002-b-v2", "--format", "json"], capture_output=True)
    return [[g["code"] + ("/" + g["subcause"] if g["subcause"] else ""), g["primary_span"]["start"], g["primary_span"]["end"],
             g["data"].get("expected_type"), g["data"].get("found_type"),
             [[x["role"], x["span"]["start"], x["span"]["end"]] for x in g["related_spans"]]] for g in json.loads(out.stdout)["diagnostics"]]
def flags(a, b):
    na = [x for x in a if x[0] != "NX-E0103"]; nb = [x for x in b if x[0] != "NX-E0103"]
    return {"solo_e0103": na == nb and a != b,
            "cambia_codigo": sorted(x[0] for x in na) != sorted(x[0] for x in nb),
            "cambia_primario": sorted((x[1], x[2]) for x in na) != sorted((x[1], x[2]) for x in nb),
            "cambia_tipos": sorted((x[3] or "", x[4] or "") for x in na) != sorted((x[3] or "", x[4] or "") for x in nb),
            "cambia_relaciones": sorted(map(str, (x[5] for x in na))) != sorted(map(str, (x[5] for x in nb))),
            "cambia_orden": [x[0] for x in na] != [x[0] for x in nb] and sorted(x[0] for x in na) == sorted(x[0] for x in nb),
            "cambia_cardinalidad": len(na) != len(nb), "e0103_base": sum(x[0] == "NX-E0103" for x in a), "e0103_variante": sum(x[0] == "NX-E0103" for x in b)}
def main():
    base, var, cand, out = sys.argv[1:]
    def work(it):
        sid, src = it
        with tempfile.TemporaryDirectory(prefix="ld-", dir=os.environ["TEMP"]) as d:
            a, b = diags(base, src, d), diags(var, src, d)
            if a == b: return None
            return {"id": sid, "source": src, "base": a, "variante": b, "candidato": diags(cand, src, d),
                    "flags": flags(a, b), "derivada": None}
    with ThreadPoolExecutor(4) as pool: rows = [r for r in pool.map(work, list(D.sources())) if r]
    json.dump(rows, open(out, "w"), indent=1)
    print(len(rows), "filas;", sum(r["flags"]["solo_e0103"] for r in rows), "solo E0103")
main()
