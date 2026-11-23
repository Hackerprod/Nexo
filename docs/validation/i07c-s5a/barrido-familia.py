#!/usr/bin/env python3
"""Barrido dirigido a la familia del limite conocido de S5a.1 (ver limites-conocidos.md).

Familia: asignacion ACEPTADA previa + asignacion RECHAZADA posterior sobre bindings `var`
con componente pendiente (uniones `if`, joins, igualdades), RHS intrinsecos y no
intrinsecos, capas 1..4. Compara TODAS las relaciones (`related_spans`: expected_type,
found_type, ...), los datos (`data`), codigos, subcausas y primarios entre dos binarios.

Uso (solo biblioteca estandar; semilla y corpus deterministas):
  python -B barrido-familia.py --base BIN_MAIN --cand BIN_CANDIDATO [--n 500] [--seed 1]
      [--profile f0-0002-b-v2] [--corpus-dir DIR] [--keep-corpus] [--jobs 4]
      [--summary RESUMEN.md] [--extra-dir DIR_CON_.n]
  python -B barrido-familia.py --show BIN FICHERO.n      # resumen de un programa

Salida: una linea por diferencia, clasificada
  PROCEDENCIA  codigo, subcausa, primario y `data` (tipos) iguales; difieren solo relaciones
  DIAGNOSTICO  difiere codigo, subcausa, primario, `data` o el numero de diagnosticos
  FALLO        un binario no produjo JSON (panico, cuelgue o salida inesperada)
Codigo de salida: 0 si no hay diferencias, 1 si las hay (informativo; el limite declarado
debe aparecer como PROCEDENCIA y nunca como DIAGNOSTICO).
El SHA-256 del corpus es el de la concatenacion `nombre\\0contenido\\0` en orden de nombre.
"""
import argparse, hashlib, json, os, random, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

# ---- generador -----------------------------------------------------------------------
# RHS escalares: intrinsecos (enteros, bool, nombres concretos) y no intrinsecos
# (binarios, llamadas, grupos) segun intrinsic_assignment_type.
INTRINSIC = ["true", "false", "3", "0", "x", "p", "()"]
NON_INTRINSIC = ["1<2", "(1<2)", "x+1", "Len(ar)", "(x)", "0-x", "x<1", "f(x)"]
SCALARS = INTRINSIC + NON_INTRINSIC

# Uniones / joins / igualdades que dejan `r` y/o `q` con componente pendiente.
UNIONS = [
    "((if true then true else q)==ar)",
    "(if p then [r,q] else (if (x<1) then r else [true]))",
    "[q,(if p then r[0] else [r])]",
    "[r,q]",
    "(if p then r else q)",
    "(if p then [r,q] else r)",
    "[(if p then r else q),[1]]",
    "(if p then r[0] else (if p then q else r))",
    "((if p then [r] else [q])==[ar])",
    "[(r[0]),(if (x<0) then r else [[2]]),[1]]",
]
PRINTS = ["", "", "Print([r,q]);", "Print(q);"]


def layers(e, k):
    return "[" * k + e + "]" * k


def accepted(rng):
    v = rng.choice(["q", "r"])
    k = rng.randint(1, 4)
    kind = rng.random()
    if kind < 0.15:
        return f"{v}=[];"
    if kind < 0.30:
        # capas con un array concreto del contexto: `[[ar]]` (ar=Array<I64>), etc.
        return f"{v}={layers('ar', rng.randint(0, 3)) if rng.random() < .5 else layers('[ar]', 0)};"
    return f"{v}={layers(rng.choice(SCALARS), k)};"


def rejected(rng):
    # asignaciones que chocan con el tipo ya fijado (suelen dejar E0102)
    v = rng.choice(["q", "r"])
    return f"{v}={rng.choice(['ar', '[ar]', '[[ar]]', 'true', '[true]', '[[true]]', '1', '[1]', '[r]', '[q]'])};"


def program(rng):
    pre = rng.choice(PRINTS)
    stmts = [accepted(rng) for _ in range(rng.randint(1, 3))]
    stmts += [rejected(rng) for _ in range(rng.randint(1, 2))]
    if rng.random() < 0.35:  # orden aleatorio (independencia de orden)
        rng.shuffle(stmts)
    elif rng.random() < 0.3:  # una aceptada tras la rechazada
        stmts.append(accepted(rng))
    return ("fn Main(){let x=1;let p=true;let ar=[1];var r=[];var q=[];let a="
            + rng.choice(UNIONS) + ";" + pre + "".join(stmts) + "}")


def corpus(n, seed):
    rng = random.Random(seed)
    seen, out = set(), []
    while len(out) < n:
        s = program(rng)
        if s not in seen:
            seen.add(s)
            out.append(s)
    return out


# ---- ejecucion y comparacion -----------------------------------------------------------
def run(binary, path, profile):
    try:
        p = subprocess.run([binary, "check", "--format", "json", "--profile", profile, path],
                           capture_output=True, timeout=120)
    except subprocess.TimeoutExpired:
        return None, "timeout"
    try:
        return json.loads(p.stdout.decode("utf-8")), None
    except Exception as e:  # salida no JSON
        return None, f"exit={p.returncode} no-json ({e})"


def span(s):
    return None if s is None else [s["start"], s["end"]]


def summarize(doc):
    """Lista de diagnosticos como tuplas comparables, con TODAS las relaciones."""
    out = []
    for d in doc.get("diagnostics", []):
        out.append({
            "code": d["code"], "subcause": d["subcause"], "primary": span(d.get("primary_span")),
            "data": d.get("data", {}),
            "rel": [(r["role"], span(r.get("span"))) for r in d.get("related_spans", [])],
        })
    return {"kind": doc.get("outcome", {}).get("kind"), "diags": out}


def fmt(sm):
    if sm is None:
        return "-"
    parts = [f"outcome={sm['kind']}"]
    for d in sm["diags"]:
        rel = ",".join(f"{r}={a}" for r, a in d["rel"])
        parts.append(f"{d['code']}/{d['subcause']} {d['primary']} {json.dumps(d['data'], sort_keys=True)} [{rel}]")
    return " | ".join(parts)


def compare(a, b):
    """None si iguales; 'PROCEDENCIA' o 'DIAGNOSTICO' si difieren."""
    if a == b:
        return None
    if a["kind"] != b["kind"] or len(a["diags"]) != len(b["diags"]):
        return "DIAGNOSTICO"
    for x, y in zip(a["diags"], b["diags"]):
        if (x["code"], x["subcause"], x["primary"], x["data"]) != (y["code"], y["subcause"], y["primary"], y["data"]):
            return "DIAGNOSTICO"
    return "PROCEDENCIA"


def one(args, name, path):
    da, ea = run(args.base, path, args.profile)
    db, eb = run(args.cand, path, args.profile)
    if da is None or db is None:
        return name, "FALLO", f"base={ea or 'ok'} cand={eb or 'ok'}", None, None
    sa, sb = summarize(da), summarize(db)
    return name, compare(sa, sb), None, sa, sb


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base")
    ap.add_argument("--cand")
    ap.add_argument("--show", nargs=2, metavar=("BIN", "FICHERO"))
    ap.add_argument("--n", type=int, default=500)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--profile", default="f0-0002-b-v2")
    ap.add_argument("--corpus-dir")
    ap.add_argument("--keep-corpus", action="store_true")
    ap.add_argument("--extra-dir", help="directorio con .n/.nx adicionales (p. ej. limites/)")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--summary")
    args = ap.parse_args()

    if args.show:
        doc, err = run(args.show[0], args.show[1], args.profile)
        print(fmt(summarize(doc)) if doc else err)
        return 0
    if not (args.base and args.cand):
        ap.error("--base y --cand son obligatorios")

    items = [(f"gen-{args.seed}-{i:04d}.n", s) for i, s in enumerate(corpus(args.n, args.seed))]
    if args.extra_dir:
        for f in sorted(os.listdir(args.extra_dir)):
            if f.endswith((".n", ".nx")):
                with open(os.path.join(args.extra_dir, f), "rb") as fh:
                    items.append((f"extra-{f}", fh.read().decode("ascii")))
    h = hashlib.sha256()
    for name, s in sorted(items):
        h.update(name.encode() + b"\0" + s.encode() + b"\0")
    corpus_sha = h.hexdigest()

    tmp = args.corpus_dir or tempfile.mkdtemp(prefix="barrido-")
    os.makedirs(tmp, exist_ok=True)
    paths = []
    for name, s in items:
        p = os.path.join(tmp, name)
        with open(p, "wb") as fh:  # ASCII sin BOM ni LF final
            fh.write(s.encode("ascii"))
        paths.append((name, p))

    with ThreadPoolExecutor(args.jobs) as ex:
        results = list(ex.map(lambda t: one(args, *t), paths))

    src = dict(items)
    counts = {"PROCEDENCIA": 0, "DIAGNOSTICO": 0, "FALLO": 0}
    lines = []
    for name, cls, err, sa, sb in results:
        if cls is None:
            continue
        counts[cls] += 1
        lines.append(f"{cls} {name}\n  fuente: {src[name]}")
        if err:
            lines.append(f"  {err}")
        else:
            for i in range(max(len(sa['diags']), len(sb['diags']))):
                a = fmt({"kind": sa["kind"], "diags": sa["diags"][i:i + 1]}) if i < len(sa["diags"]) else "-"
                b = fmt({"kind": sb["kind"], "diags": sb["diags"][i:i + 1]}) if i < len(sb["diags"]) else "-"
                if a != b:
                    lines.append(f"  base: {a}\n  cand: {b}")
    print("\n".join(lines))
    total = len(items)
    resumen = (f"programas={total} (generados={args.n}, semilla={args.seed}, extra={total - args.n}) "
               f"perfil={args.profile} corpus_sha256={corpus_sha}\n"
               f"diferencias: PROCEDENCIA={counts['PROCEDENCIA']} DIAGNOSTICO={counts['DIAGNOSTICO']} FALLO={counts['FALLO']}")
    print(resumen)
    if args.summary:
        with open(args.summary, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(resumen + "\n")
    if not args.keep_corpus and not args.corpus_dir:
        for _, p in paths:
            os.remove(p)
        os.rmdir(tmp)
    return 1 if any(counts.values()) else 0


if __name__ == "__main__":
    sys.exit(main())
