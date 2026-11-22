"""Mutation campaign over the oracle itself (class ORQUESTA; plan section 6).

``python -B -m scripts.f4_oracle.mutate F4-Ok``: every mutant is ONE exact substitution applied to a
private copy of the oracle (``C:/nexo-work/tmp/f4-oracle-mut-<PID>-<worker>/``); each worker owns its
copy and a copy lock, so two mutation chains never share a copy.  After every mutant the original
file is restored and its SHA-256 compared with the blob taken before the first mutant (a mismatch
aborts the campaign: exit 2).  A mutant is KILLED when the self-test run on the copy fails (the
first failed check is its label; ``--fail-fast`` stops there) and SURVIVES when it passes; a
mutant declared EQUIVALENT must survive the complete self-test and carries its argument
(MUTACIONES.md).  The skeleton of the F5 ``run`` is not reused: only ``generic.sha`` is.

Exit 0: every mutant killed or equivalent (and every equivalent one survived); 1: a survivor
that is not declared equivalent, or an equivalence that a check refutes; 2: harness error.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

from .generic import sha

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TMP = Path("C:/nexo-work/tmp")
ALL_GROUPS = ",".join(["G%d" % i for i in range(11)])
COPY_DIRS = ["scripts/f4_oracle"]
COPY_FILES = ["Cargo.toml", "docs/adr/F0-0004-text-and-user-types.md", "docs/adr/F0-0002-second-slice.md",
              "docs/adr/F0-0003-third-slice.md", "docs/adr/F0-0005-operators-control-collections.md",
              "docs/adr/F0-D15-sintaxis-oficial.md"]


class MutationError(Exception):
    """Harness error (invalid substitution, failed restoration): exit 2."""


@dataclass(frozen=True)
class Mutation:
    ident: str
    family: str
    file: str
    old: str
    new: str
    groups: str = ALL_GROUPS
    equivalent: str | None = None
    note: str = ""


def M(ident, family, file, old, new, groups=ALL_GROUPS, equivalent=None, note=""):
    return Mutation(ident, family, "scripts/f4_oracle/" + file, old, new, groups, equivalent, note)


def _catalogue() -> list:
    c = []
    # ---- text (K 3, 10.3, 12.2)
    c += [
        M("T01", "text", "model.py", 'if e.name == "ByteLen":\n                return len(b)',
          'if e.name == "ByteLen":\n                return utf8ref.scalar_len(b)', note="ByteLen como ScalarLen"),
        M("T02", "text", "model.py", 'if e.name == "ScalarLen":\n                return utf8ref.scalar_len(b)',
          'if e.name == "ScalarLen":\n                return len(b)', note="ScalarLen como ByteLen"),
        M("T03", "text", "model.py", "return list(scalars_of(b))",
          'return list(scalars_of(b.replace(b"e\\xcc\\x81", b"\\xc3\\xa9")))', note="normalizacion NFC en Scalars"),
        M("T04", "text", "utf8ref.py", "        x = a[i]\n        y = b[i]\n",
          "        x = a[i] - 256 if a[i] > 127 else a[i]\n        y = b[i] - 256 if b[i] > 127 else b[i]\n",
          note="orden con signo"),
        M("T05", "text", "utf8ref.py", "    return -1 if len(a) < len(b) else 1\n\n\ndef text",
          "    return 1 if len(a) < len(b) else -1\n\n\ndef text", note="prefijo igual mal ordenado"),
        M("T06", "text", "model.py", "        a = self.eval(e.left, scope)\n        b = self.eval(e.right, scope)\n        op = e.op",
          "        b = self.eval(e.right, scope)\n        a = self.eval(e.left, scope)\n        op = e.op",
          note="derecha primero en operadores binarios"),
        M("T07", "text", "model.py", '    if op == "!=":\n        return c != 0', '    if op == "!=":\n        return c == 0',
          note="!= igual a =="),
        M("T08", "text", "model.py", "start <= end and end <= len(s)):", "start <= end and end <= len(s) + 1):",
          note="SliceBytes acepta end=len+1"),
        M("T09", "text", "utf8ref.py", "    if i == len(data):\n        return True",
          "    if i == len(data):\n        return False", note="SliceBytes con start=end en EOF"),
        M("T10", "text", "model.py", '    if not (0 <= start and start <= end and end <= len(s)):\n        return ("Err", -9110)',
          '    if not (utf8ref.is_boundary(s, start) and utf8ref.is_boundary(s, end)):\n        return ("Err", -9111)\n'
          '    if not (0 <= start and start <= end and end <= len(s)):\n        return ("Err", -9110)',
          note="frontera antes que rango"),
        M("T11", "text", "model.py", "    i = 1 if len(s) > 0 and s[0] == 45 else 0", "    i = 1 if len(s) > 0 and s[0] in (43, 45) else 0",
          note="parse acepta +"),
        M("T12", "text", "model.py", "(K section 3).\"\"\"\n    i = 1 if", "(K section 3).\"\"\"\n    s = s.strip()\n    i = 1 if",
          note="parse recorta espacios"),
        M("T13", "text", "model.py", "    for k in range(i, len(s)):\n        if s[k] < 48 or s[k] > 57:",
          "    for k in range(i, min(len(s), 19)):\n        if s[k] < 48 or s[k] > 57:",
          note="overflow antes de validar el resto"),
        M("T14", "text", "model.py", "    if i == 1:\n        value = -value\n",
          '    if i == 1:\n        if value == 0:\n            return ("Err", -9101)\n        value = -value\n',
          note="parse rechaza -0"),
        M("T15", "text", "model.py", "        if s[k] < 48 or s[k] > 57:\n            return",
          "        if s[k] < 48 or (s[k] > 57 and s[k] < 128):\n            return", note="digitos Unicode"),
        M("T16", "text", "model.py", '    if i >= len(s):\n        return ("Err", -9101)',
          '    if i >= len(s):\n        return ("Ok", 0)', note="vacio vale 0"),
        M("T17", "text", "model.py", "    if value < I64_MIN or value > I64_MAX:", "    if value <= I64_MIN or value > I64_MAX:",
          note="MIN falla con -9102"),
        M("T18", "text", "model.py", "    if n < 0:\n        digits.append(45)", "    if n < 0 and n != -9223372036854775808:\n        digits.append(45)",
          note="FromI64 en MIN sin signo"),
        M("T19", "text", "model.py", "return list(scalars_of(b))", "return list(b)", note="Scalars por bytes"),
        M("T20", "text", "model.py", "return len(b) == 0", "return len(b) <= 1", note="IsEmpty"),
        M("T21", "text", "model.py", "return self.new_str(b + args[0].b, key)", "return self.new_str(args[0].b + b, key)",
          note="Concat invertido"),
    ]
    # ---- nominals (K 4, 5)
    c += [
        M("N01", "nominal", "model.py", "assigned[d.fields[positional.index(a)].name] = v",
          "assigned[d.fields[len(positional) - 1 - positional.index(a)].name] = v", note="asociacion posicional invertida"),
        M("N02", "nominal", "model.py", "            order = positional + [a for a in e.args if a.label is not None]",
          "            order = positional + sorted([a for a in e.args if a.label is not None],\n"
          "                                        key=lambda a: [f.name for f in d.fields].index(a.label))",
          note="evaluacion en orden de campo"),
        M("N03", "nominal", "model.py", "            for x in reversed(list(v.fields.values())):\n                self.close_value(x)",
          "            for x in list(v.fields.values()):\n                self.close_value(x)", note="limpieza por orden de declaracion"),
        M("N04", "nominal", "model.py", "                for v in reversed(prepared):\n                    self.close_value(v)",
          "                for v in prepared:\n                    self.close_value(v)", note="limpieza parcial en orden de evaluacion"),
        M("N05", "nominal", "model.py", "                return all(self.copyable(c, seen + (t,)) for c in self.comps(t))",
          "                return any(self.copyable(c, seen + (t,)) for c in self.comps(t))",
          note="copiable si lo es alguna alternativa"),
        M("N06", "nominal", "model.py", "                    if best is None or key < best[0]:", "                    if best is None or key > best[0]:",
          note="ciclo mas largo"),
        M("N07", "nominal", "model.py", '            self.emit("NX-E1323", "recursive_value_layout", (norm[-1][2], "all"),',
          '            self.emit("NX-E1323", "recursive_value_layout", (norm[0][2], "all"),', note="primario en la primera arista"),
        M("N08", "nominal", "model.py", '        elif lt not in ("I64", "String", UNKNOWN):\n            self.emit("NX-E0102", "type_mismatch", (e, "op")',
          '        elif lt not in ("I64", "String", UNKNOWN) and self.lacking(lt, "PartialEq") is not None:\n'
          '            self.emit("NX-E0102", "type_mismatch", (e, "op")', note="derive concede orden"),
        M("N09", "nominal", "model.py", "            found = self.lacking(x, cap, seen)\n", "            found = None\n",
          note="Array vacio sin capacidad"),
        M("N10", "nominal", "model.py", "                if cap not in d.derives:\n                    return (\"nominal\", d)",
          "                if cap not in d.derives and not (cap == \"PartialEq\" and \"Format\" in d.derives):\n"
          "                    return (\"nominal\", d)", note="Format implica PartialEq"),
        M("N11", "nominal", "model.py", "        if not self.same(lt, rt):\n            self.emit(\"NX-E0102\", \"type_mismatch\", (e.right, \"all\"),",
          "        if op in (\"==\", \"!=\") and self.lacking(lt, \"PartialEq\") and not self.same(lt, rt):\n"
          "            self.need_cap(lt, \"PartialEq\", (e, \"op\"))\n            return \"Bool\"\n"
          "        if not self.same(lt, rt):\n            self.emit(\"NX-E0102\", \"type_mismatch\", (e.right, \"all\"),",
          note="capacidad antes que tipos distintos"),
        M("N12", "nominal", "model.py", 'return utf8ref.text(v.name) + b"(" + inner + b")"',
          'return b"package." + utf8ref.text(v.name) + b"(" + inner + b")"', note="Format con package."),
        M("N13", "nominal", "model.py", 'return head + b"(" + b", ".join(format_value(x) for x in v.payload) + b")"',
          'return head + b"(" + b",".join(format_value(x) for x in v.payload) + b")"', note="separador de payload"),
    ]
    # ---- permissions and methods (K 4.3, 5.1)
    c += [
        M("P01", "permisos", "model.py", '            if info.kind == "param" and info.mode == "view":', '            if False and info.kind == "param" and info.mode == "view":',
          note="view escribe"),
        M("P02", "permisos", "model.py", '            if info.kind == "let":\n                self.emit("NX-E0105"', '            if False:\n                self.emit("NX-E0105"',
          note="asignacion a let"),
        M("P03", "permisos", "model.py", '        else:\n            self.emit("NX-E1107", None, (e.recv, "all"), (), data)', "        else:\n            pass",
          note="temporal sin NX-E1107"),
        M("P04", "permisos", "model.py", "        if rank[requested] >= rank[required]:", "        if rank[requested] > rank[required]:",
          note="modo igual al requerido rechazado"),
        M("P05", "permisos", "model.py", "            if rt not in self.activated:", "            if False:", note="metodo sin use extension"),
    ]
    # ---- literal grammar (K 2.1)
    c += [
        M("L01", "literal", "literal.py", "        out.append(out[-1] + len(utf8ref.text(ch)))", "        out.append(out[-1] + 1)",
          note="offsets en escalares"),
        M("L02", "literal", "literal.py", '            if src[i + 1:i + 2] == "{":\n                text_buf.append("{")',
          '            if False:\n                text_buf.append("{")', note="{{ como Hole"),
        M("L03", "literal", "literal.py", '            if src[i + 1:i + 2] == "}":\n                text_buf.append("}")',
          '            if False:\n                text_buf.append("}")', note="}} sin escapar"),
        M("L04", "literal", "literal.py", '        if c in "\\r\\n":\n            res.other = "raw_control_in_text"',
          '        if c in "\\n":\n            res.other = "raw_control_in_text"', note="CR crudo aceptado"),
        M("L05", "literal", "literal.py", "            res.end = recover(src, i + 1, 0)\n            return res\n        text_buf.append(c)",
          "            res.end = i + 1\n            return res\n        text_buf.append(c)", note="recuperacion tras llave suelta"),
    ]
    # ---- barrier (K 9.1)
    c += [
        M("B01", "barrera", "barrier.py", "    out.sort(key=lambda item: (item[0], item[1]))", "    out.sort(key=lambda item: (item[0], -item[1]))",
          note="K08 antes que K02 en un empate"),
        M("B02", "barrera", "barrier.py", "        for dep in DEPENDS[k]:\n            if dep not in declared:", "        for dep in ():\n            if dep not in declared:",
          equivalent=("Las dependencias de K 12 (K02<-K01 ... K10<-K06+K07+K09) ya las imponen las listas de cada fase "
                      "(debe contener / puede contener de plan 2.1): todo manifiesto que las viola incumple tambien "
                      "una de esas listas y falla con la misma ManifestError (exit 2); solo cambia el mensaje, que "
                      "ningun literal compara."),
          note="dependencias K sin validar"),
        M("B03", "barrera", "barrier.py", "    return diagnostics.barrier(CONSTRUCT_K[chosen.kind], file=file, primary=chosen.span)",
          "    return diagnostics.barrier(CONSTRUCT_K[chosen.kind], file=file, primary=(0, 0))", note="primario de literal omitido"),
    ]
    # ---- console and application (K 6.1, 10.6, 11)
    c += [
        M("C01", "consola", "app.py", 'item = self.reads.pop(0) if self.reads else ("err", -9000)', 'item = self.reads.pop(0) if self.reads else ("eof",)',
          groups="G9", note="cola vacia como EOF"),
        M("C02", "consola", "app.py", '        self.event("println:err:%d" % item[1])\n        return ("err", item[1])',
          '        self.event("println:err:%d" % item[1])\n        self.order.append(("stdout", text))\n        return ("err", item[1])',
          groups="G9", note="Println Err con stdout"),
        M("C03", "consola", "app.py", '(8 if c == "event" else 0)', "0", groups="G9", note="bytes sin mock-io:"),
        M("C04", "consola", "app.py", 'elif isinstance(v, str) and "\\r" not in v and "\\n" not in v:', 'elif isinstance(v, str) and "\\n" not in v:',
          groups="G9", note="CR aceptado"),
        M("C05", "consola", "app.py", '                        self.bad("lone high surrogate")\n                    self.i += 2',
          '                        v = 0xFFFD\n                        out.append(chr(v))\n                        continue\n                    self.i += 2',
          groups="G9", note="surrogate suelto reemplazado por U+FFFD"),
        M("C06", "consola", "app.py", "    if 10 <= value <= 100:", "    if 10 < value <= 100:", groups="G9", note="frontera del descuento"),
        M("C07", "consola", "app.py", "return (numero, text, base + base * 21 // 100)", "return (numero, text, base + (base * 21 + 99) // 100)",
          groups="G9", note="IVA redondeado hacia arriba"),
        M("C08", "consola", "app.py", "for numero in range(1, MAX_LINES + 1):", "for numero in range(1, MAX_LINES + 2):", groups="G9",
          note="lee la linea 101"),
        M("C09", "consola", "app.py", 'if got[0] == "none" or got[1] == "fin":', 'if got[0] == "none" or got[1].lower() == "fin":', groups="G9",
          note="fin insensible a mayusculas"),
        M("C10", "consola", "app.py", "    media = total // count if count != 0 else 0", "    media = round(total / count) if count != 0 else 0",
          groups="G9", note="media redondeada"),
        M("C11", "consola", "app.py", "    if value < 0 or value > 10000:", "    if value < 0 or value >= 10000:", groups="G9",
          note="dominio de importe"),
    ]
    # ---- quota recipes (K 8.2, 8.3)
    c += [
        M("Q01", "cuotas", "charges.py", '        if nbytes > lim["string_bytes"]:', '        if nbytes >= lim["string_bytes"]:', groups="G9,G10",
          note="string_bytes en la frontera"),
        M("Q02", "cuotas", "charges.py", '        elif self.allocs + 1 > lim["text_allocations"]:', '        elif self.allocs + 1 >= lim["text_allocations"]:',
          groups="G9,G10", note="text_allocations en la frontera"),
        M("Q03", "cuotas", "charges.py", "                st.live -= taken\n", "                pass\n", groups="G9,G10",
          note="las cadenas de un bucle no mueren"),
        M("Q04", "cuotas", "charges.py", '"limit": limit, "observed": limit + 1})', '"limit": limit, "observed": limit + 2})',
          groups="G9,G10", note="observed de miembros"),
    ]
    # ---- harness (class ARNES) and its judge
    c += [
        M("H01", "arnes", "runner.py", '            if totals["divergences"] or totals["failures"]:\n                code = 1',
          '            if totals["failures"]:\n                code = 1', groups="HARNESS", note="exit 0 con divergencia"),
        M("H02", "arnes", "runner.py", '            and PHASE_ORDER[phase] <= PHASE_ORDER[entry["max_phase"]])', "            and True)",
          groups="HARNESS", note="known_issue sin caducidad"),
        M("H03", "arnes", "runner.py", "                if inv.error is not None:\n                    findings.append(_finding(rec, verb, which, inv.error",
          '                if inv.error is not None and inv.error != "timeout_confirmed":\n                    findings.append(_finding(rec, verb, which, inv.error',
          groups="HARNESS", note="timeout cuenta como PASS"),
        M("H04", "arnes", "runner.py", '    if rec["expected_sha256"] != want:\n', '    if False:\n', groups="HARNESS",
          note="expected_sha256 corrupto aceptado"),
        M("H05", "arnes", "runner.py", "                path = _first_difference({\"exit\": d.exit, \"doc\": d.document},", "                path = None and _first_difference({\"exit\": d.exit, \"doc\": d.document},",
          groups="HARNESS", note="debug y release no se comparan"),
        M("H06", "arnes", "generic.py", "    if type(expected) is not type(actual):\n        return path", "    if False:\n        return path",
          groups="HARNESS", note="tipos JSON 1/true/\"1\" equivalentes (pin AST y arnes)"),
        M("H07", "arnes", "runner.py", "    if isinstance(out, dict) and out.get(\"kind\") == \"err\" and type(out.get(\"error\")) is int:",
          "    if False:", groups="HARNESS", note="Err(i64) solo como cadena"),
        M("H08", "arnes", "runner.py", '"keys": sorted(seed_matched, key=_total)}', '"keys": sorted(seed_matched)}',
          groups="HARNESS", note="orden parcial de claves con codigo None"),
        M("H09", "arnes", "runner.py", "    out.update({k: v for k, v in d.items() if k not in DIAG_KEYS})\n",
          "    pass\n", groups="HARNESS", note="campo extra de un diagnostico ignorado"),
        M("H10", "arnes", "runner.py", "        if key not in TOP_SCHEMA:\n            obs[key] = doc[key]",
          "        if False:\n            obs[key] = doc[key]", groups="HARNESS", note="clave extra de primer nivel ignorada"),
        M("H11", "arnes", "cli.py", '    if not getattr(args, "no_literals", False):', '    if getattr(args, "no_literals", False):',
          groups="HARNESS", note="literales de K desactivados por defecto"),
        M("L06", "literal", "literal.py", '"/", "%", "<", ">")', '"/", "<", ">")', groups="G0", note="% fuera de los operadores de un Hole"),
        M("L07", "literal", "literal.py", '"t": "\\t"}', '"t": " "}', groups="G0", note="escape \\t decodificado como espacio"),
        M("D01", "diagnostico", "diagnostics.py", '"evidence": "verified"', '"evidence": "unknown"', groups="G0",
          note="evidence de la cadena causal"),
        M("Q05", "cuotas", "model.py", 'DEFAULT_LIMITS = {"string_bytes": 65536,', 'DEFAULT_LIMITS = {"string_bytes": 65537,',
          groups="G9", note="limite string_bytes del evaluador"),
        M("Q06", "cuotas", "model.py", '"live_text_bytes": 262144,', '"live_text_bytes": 262145,', groups="G9",
          note="limite live_text_bytes del evaluador"),
        M("R01", "texto", "model.py", "            if isinstance(e.index, Rng):\n", "            if False and isinstance(e.index, Rng):\n",
          groups="G3", note="s[a..<b] sin range/later_adr"),
        M("R02", "texto", "model.py", "        if st.op != \"=\":\n            # K 3:", "        if False:\n            # K 3:",
          groups="G3", note="compuesta += sobre String aceptada"),
        M("R03", "nominal", "model.py", "            self.emit(\"NX-E0102\", \"type_mismatch\", primary, [(\"found_type\", primary)], {})\n            return\n",
          "            return\n", groups="G3", note="Print de un recurso aceptado"),
        M("R04", "nominal", "model.py", "            if len(arg_types) == 2 and self.same(arg_types[0], arg_types[1]):\n",
          "            if False:\n", groups="G3", note="AssertEq sin exigir PartialEq"),
        M("S01", "literal", "model.py",
          "        buf = b\"\"\n        for part in e.parts:\n            if isinstance(part, str):\n                buf += utf8ref.text(part)\n            else:\n                buf += format_value(self.eval(part, scope))\n",
          "        vals = {}\n        for k in reversed(range(len(e.parts))):\n            if not isinstance(e.parts[k], str):\n                vals[k] = format_value(self.eval(e.parts[k], scope))\n"
          "        buf = b\"\"\n        for k, part in enumerate(e.parts):\n            buf += utf8ref.text(part) if isinstance(part, str) else vals[k]\n",
          groups="G7", note="Holes evaluados de derecha a izquierda"),
        M("B04", "barrera", "barrier.py", "    return found[0][3] if found else None", "    return found[-1][3] if found else None",
          groups="G0", note="barrera elige la ultima construccion"),
        M("B05", "barrera", "barrier.py", "    return len(distinct_pending_scopes(constructs, closed)) == 1",
          "    return len(distinct_pending_scopes(constructs, closed)) >= 1", groups="G0,GEN",
          note="clase B admite varias K pendientes"),
    ]
    return c


MUTATIONS = _catalogue()


# ------------------------------------------------------------------ private copy with restoration

class Copy:
    """One private copy of the oracle: one mutation chain at a time (the copy lock)."""

    def __init__(self, root: Path):
        self.root = Path(root)
        self.lock = threading.Lock()
        self.blobs: dict = {}

    def build(self, source_root: Path = ROOT):
        if self.root.exists():
            shutil.rmtree(self.root, ignore_errors=True)
        for rel in COPY_DIRS:
            shutil.copytree(source_root / rel, self.root / rel, ignore=shutil.ignore_patterns("__pycache__"))
        fx = self.root / "tests" / "fixtures"
        fx.mkdir(parents=True)
        for item in (source_root / "tests" / "fixtures").iterdir():
            if item.name.startswith("f4_oracle_"):
                shutil.copyfile(item, fx / item.name)
        for rel in COPY_FILES:
            (self.root / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source_root / rel, self.root / rel)
        return self

    def apply(self, rel: str, old: str, new: str) -> str:
        """Substitute ``old`` by ``new`` (exactly one occurrence); returns the original text."""
        path = self.root / rel
        data = path.read_bytes()
        text = data.decode("utf-8")
        if text.count(old) != 1:
            raise MutationError(f"{rel}: the pattern occurs {text.count(old)} times")
        self.blobs[rel] = (data, sha(path))
        path.write_bytes(text.replace(old, new, 1).encode("utf-8"))
        return text

    def restore(self, rel: str):
        data, digest = self.blobs.pop(rel)
        path = self.root / rel
        path.write_bytes(data)
        if sha(path) != digest:
            raise MutationError(f"{rel}: SHA-256 differs after the restoration")

    def close(self):
        shutil.rmtree(self.root, ignore_errors=True)


def run_selftest(copy_root: Path, groups: str, timeout: float, full: bool = False) -> dict:
    """Run the self-test of a copy; returns {status, label, detail, seconds}."""
    command = [sys.executable, "-B", "-m", "scripts.f4_oracle.cli", "selftest"]
    if not full:
        command += ["--groups", groups, "--fail-fast"]
    env = dict(os.environ, PYTHONUTF8="1", PYTHONHASHSEED="0", F4_ORACLE_MUTANT="1")
    started = time.time()
    try:
        done = subprocess.run(command, cwd=str(copy_root), capture_output=True, timeout=timeout, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "killed", "label": "timeout", "detail": "no termination", "seconds": time.time() - started}
    out = done.stdout.decode("utf-8", "replace").splitlines()
    err = done.stderr.decode("utf-8", "replace").strip().splitlines()
    fails = [ln for ln in out if ln.startswith("FAIL ")]
    seconds = round(time.time() - started, 1)
    if done.returncode == 0:
        return {"status": "survived", "label": None, "detail": "", "seconds": seconds}
    if fails:
        name, _, detail = fails[0][5:].partition(": ")
        return {"status": "killed", "label": name, "detail": detail[:160], "seconds": seconds}
    return {"status": "killed", "label": "crash", "detail": (err[-1] if err else "exit %d" % done.returncode)[:160],
            "seconds": seconds}


def verdict(m: Mutation, result: dict) -> str:
    if m.equivalent is not None:
        return "equivalent" if result["status"] == "survived" else "refuted"
    return "killed" if result["status"] == "killed" else "SURVIVOR"


def run_one(copy: Copy, m: Mutation, timeout: float, full: bool) -> dict:
    with copy.lock:
        copy.apply(m.file, m.old, m.new)
        try:
            result = run_selftest(copy.root, m.groups, timeout, full or m.equivalent is not None)
        finally:
            copy.restore(m.file)
    result = dict(result, id=m.ident, family=m.family, note=m.note, file=m.file)
    result["verdict"] = verdict(m, result)
    return result


def run_campaign(mutations: list, workers: int = 2, timeout: float = 900.0, full: bool = False, out_path=None) -> tuple:
    """Run the mutations on ``workers`` private copies; returns (exit code, results)."""
    TMP.mkdir(parents=True, exist_ok=True)
    copies = [Copy(TMP / f"f4-oracle-mut-{os.getpid()}-{w}").build() for w in range(workers)]
    free: list = list(copies)
    guard = threading.Lock()
    results: list = []

    def job(m: Mutation):
        with guard:
            copy = free.pop()
        try:
            res = run_one(copy, m, timeout, full)
        finally:
            with guard:
                free.append(copy)
        print(f"{res['verdict']:10} {m.ident} {m.note}: {res['label']} ({res['seconds']}s)", flush=True)
        return res

    try:
        with ThreadPoolExecutor(max_workers=workers) as pool:
            results = list(pool.map(job, mutations))
    except MutationError as exc:
        print(f"harness error: {exc}", file=sys.stderr)
        return 2, results
    finally:
        for copy in copies:
            copy.close()
    if out_path:
        Path(out_path).write_text(json.dumps(results, ensure_ascii=False, indent=1), encoding="utf-8")
    bad = [r for r in results if r["verdict"] in ("SURVIVOR", "refuted")]
    killed = sum(1 for r in results if r["verdict"] == "killed")
    equivalent = sum(1 for r in results if r["verdict"] == "equivalent")
    print(f"mutate: {len(results)} mutants, {killed} killed, {equivalent} equivalent, {len(bad)} open")
    return (1 if bad else 0), results


def check_catalogue() -> list:
    """Problems of the catalogue itself: an ``old`` that does not occur exactly once, duplicate ids."""
    problems, seen = [], set()
    for m in MUTATIONS:
        if m.ident in seen:
            problems.append(f"{m.ident}: duplicate id")
        seen.add(m.ident)
        text = (ROOT / m.file).read_bytes().decode("utf-8")
        n = text.count(m.old)
        if n != 1:
            problems.append(f"{m.ident}: pattern occurs {n} times in {m.file}")
        if m.old == m.new:
            problems.append(f"{m.ident}: empty substitution")
    return problems


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="f4_oracle.mutate")
    parser.add_argument("label", help="campaign label, e.g. F4-Ok")
    parser.add_argument("--only", default="", help="comma separated mutant ids")
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--timeout", type=float, default=900.0)
    parser.add_argument("--full", action="store_true", help="run the complete self-test for every mutant")
    parser.add_argument("--check", action="store_true", help="only validate the catalogue")
    parser.add_argument("--output", default="")
    args = parser.parse_args(argv)
    problems = check_catalogue()
    if problems:
        for p in problems:
            print(f"catalogue: {p}", file=sys.stderr)
        return 2
    if args.check:
        print(f"catalogue: {len(MUTATIONS)} mutants")
        return 0
    wanted = [m for m in MUTATIONS if not args.only or m.ident in args.only.split(",")]
    code, _ = run_campaign(wanted, args.workers, args.timeout, args.full, args.output or None)
    return code


if __name__ == "__main__":
    sys.exit(main())
